#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use sp_runtime::{
    offchain::{http, Duration},
    DispatchError,
};
use sp_std::prelude::*;

pub const USER_AGENT: &str = "GoogleBot (compatible; ParamiWorker/1.0; +http://parami.io/worker/)";

mod macros {
    #[macro_export]
    macro_rules! submit_unsigned {
        ($call:expr) => {
            SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction($call.into())
        };
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
    pub fn ocw_fetch<U: AsRef<str>>(url: U) -> Result<Vec<u8>, DispatchError> {
        let url = url.as_ref();

        let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(3_000));

        let request = http::Request::get(url);

        let pending = request
            .add_header("User-Agent", USER_AGENT)
            .deadline(deadline)
            .send()
            .map_err(|_| Error::<T>::RequestError)?;

        let response = pending
            .try_wait(deadline)
            .map_err(|_| Error::<T>::ResponseError)?
            .map_err(|_| Error::<T>::ResponseError)?;

        if response.code >= 400 {
            tracing::warn!("Unexpected status code: {}", response.code);
            Err(Error::<T>::HttpError)?
        }

        Ok(response.body().collect::<Vec<u8>>())
    }
}
