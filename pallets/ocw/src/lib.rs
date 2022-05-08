#![cfg_attr(not(feature = "std"), no_std)]

pub use lite_json::JsonValue;
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use sp_runtime::{
    offchain::{http, Duration},
    DispatchError,
};
use sp_std::{
    borrow::{Cow, ToOwned},
    prelude::*,
};

pub const USER_AGENT: &str = "GoogleBot (compatible; ParamiWorker/1.0; +http://parami.io/worker/)";

mod macros {
    #[macro_export]
    macro_rules! submit_unsigned {
        ($call:expr) => {
            SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction($call.into())
        };
    }
}

pub struct Response(http::Response);

impl Response {
    pub fn status(&self) -> u16 {
        self.0.code
    }

    pub fn body(&self) -> Vec<u8> {
        self.0.body().collect::<Vec<u8>>()
    }

    pub fn text<'a>(&self) -> Cow<'a, str> {
        let body = self.body();

        let text = sp_std::str::from_utf8(&body).unwrap_or_default().to_owned();

        Cow::Owned(text)
    }

    pub fn json(&self) -> JsonValue {
        let body = self.body();
        let text = match sp_std::str::from_utf8(&body) {
            Ok(text) => text,
            Err(_) => return JsonValue::Null,
        };

        lite_json::parse_json(text).unwrap_or(JsonValue::Null)
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {}

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::hooks]
    impl<T: Config> Hooks<T::BlockNumber> for Pallet<T> {}

    #[pallet::error]
    pub enum Error<T> {
        RequestError,
        ResponseError,
        HttpError,
    }
}

impl<T: Config> Pallet<T> {
    pub fn ocw_get<U: AsRef<str>>(url: U) -> Result<Response, DispatchError> {
        Self::ocw_fetch(url, http::Method::Get, Default::default())
    }

    pub fn ocw_post<U: AsRef<str>>(url: U, body: Vec<u8>) -> Result<Response, DispatchError> {
        Self::ocw_fetch(url, http::Method::Post, body)
    }

    fn ocw_fetch<U: AsRef<str>>(
        url: U,
        method: http::Method,
        body: Vec<u8>,
    ) -> Result<Response, DispatchError> {
        let url = url.as_ref();

        let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(3_000));

        let request = http::Request::default()
            .method(method.clone())
            .url(url)
            .deadline(deadline)
            .add_header("User-Agent", USER_AGENT);

        let request = match method {
            http::Method::Post | http::Method::Put | http::Method::Patch if body.len() > 0 => {
                request.body(vec![&body])
            }
            _ => request,
        };

        let pending = request.send().map_err(|_| Error::<T>::RequestError)?;

        let response = pending
            .try_wait(deadline)
            .map_err(|_| Error::<T>::ResponseError)?
            .map_err(|_| Error::<T>::ResponseError)?;

        if response.code >= 400 {
            tracing::warn!("Unexpected status code: {}", response.code);
            Err(Error::<T>::HttpError)?
        }

        Ok(Response(response))
    }
}
