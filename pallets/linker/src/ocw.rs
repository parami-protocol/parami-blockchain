use crate::{did, is_task, types, Call, Config, Error, Pallet, PendingOf};
use codec::Encode;
use frame_system::offchain::{CreateSignedTransaction, SubmitTransaction};
use sp_runtime::offchain::{http, Duration};
use sp_runtime_interface::runtime_interface;
use sp_std::prelude::*;

pub const USER_AGENT: &str =
    "GoogleBot (compatible; ParamiValidator/1.0; +http://parami.io/validator/)";

#[runtime_interface]
pub trait Images {
    fn decode_jpeg(data: Vec<u8>) -> Option<types::RawImage> {
        #[cfg(feature = "std")]
        {
            use image::io::Reader as ImageReader;
            use std::io::Cursor;

            let image = match ImageReader::new(Cursor::new(data)).with_guessed_format() {
                Ok(image) => match image.decode() {
                    Ok(image) => image,
                    Err(_) => return None,
                },
                Err(_) => return None,
            };

            let image = image.grayscale().into_luma8();

            Some(types::RawImage::new(
                image.width(),
                image.height(),
                image.as_raw().clone(),
            ))
        }

        #[cfg(not(feature = "std"))]
        {
            unimplemented!()
        }
    }
}

macro_rules! submit_unsigned {
    ($call:expr) => {
        SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction($call.into())
    };
}

impl<T: Config + CreateSignedTransaction<Call<T>>> Pallet<T> {
    pub fn ocw_begin_block(block_number: T::BlockNumber) -> Result<(), Error<T>> {
        use types::AccountType::*;

        for site in [Telegram, Twitter] {
            let pending = <PendingOf<T>>::iter_prefix(&site);

            for (did, task) in pending {
                if task.deadline <= block_number {
                    // call to remove
                    Self::ocw_submit_link(did, site, Vec::<u8>::new(), false);

                    return Err(Error::<T>::Deadline);
                };

                if task.created < block_number {
                    // only start once (at created + 1)
                    continue;
                };

                let profile = sp_std::str::from_utf8(&task.profile) //
                    .map_err(|_| Error::<T>::HttpFetchingError)?;

                let result = match site {
                    Telegram if is_task!(task.profile, b"https://t.me/") => {
                        Self::ocw_verify_telegram(did, profile)
                    }
                    Twitter if is_task!(task.profile, b"https://twitter.com/") => {
                        Self::ocw_verify_twitter(did, profile)
                    }
                    _ => {
                        // drop unsupported sites
                        Self::ocw_submit_link(did, site, Vec::<u8>::new(), false);

                        continue;
                    }
                };

                if let Ok(()) = result {
                    Self::ocw_submit_link(did, site, task.profile, true);
                }
            }
        }

        Ok(())
    }

    pub(crate) fn ocw_submit_link(
        did: T::DecentralizedId,
        site: types::AccountType,
        profile: Vec<u8>,
        ok: bool,
    ) {
        let call = Call::submit_link {
            did,
            site,
            profile,
            ok,
        };

        let _ = submit_unsigned!(call);
    }

    pub(crate) fn ocw_verify_telegram<U: AsRef<str>>(
        did: T::DecentralizedId,
        profile: U,
    ) -> Result<(), Error<T>> {
        let res = Self::ocw_fetch(profile)?;

        let res = sp_std::str::from_utf8(&res).map_err(|_| Error::<T>::HttpFetchingError)?;

        let res = res.replace(" ", "");

        let start = res
            .find("<metaproperty=\"og:image\"content=\"")
            .ok_or(Error::<T>::HttpFetchingError)?
            + 33;
        let end = res[start..]
            .find("\"")
            .ok_or(Error::<T>::HttpFetchingError)?;

        let avatar = &res[start..start + end];

        Self::ocw_check_avatar(avatar, did)
    }

    pub(crate) fn ocw_verify_twitter<U: AsRef<str>>(
        did: T::DecentralizedId,
        profile: U,
    ) -> Result<(), Error<T>> {
        let res = Self::ocw_fetch(profile)?;

        let res = sp_std::str::from_utf8(&res).map_err(|_| Error::<T>::HttpFetchingError)?;

        let res = res.replace(" ", "");

        let start = res
            .find("<imgclass=\"ProfileAvatar-image\"src=\"")
            .ok_or(Error::<T>::HttpFetchingError)?
            + 36;
        let end = res[start..]
            .find("\"")
            .ok_or(Error::<T>::HttpFetchingError)?;

        let avatar = &res[start..start + end];

        Self::ocw_check_avatar(avatar, did)
    }

    pub(crate) fn ocw_check_avatar<U: AsRef<str>>(
        avatar: U,
        did: T::DecentralizedId,
    ) -> Result<(), Error<T>> {
        let res = Self::ocw_fetch(avatar)?;

        let did = did.encode();

        match did::parse(res) {
            Some(res) if res == did => Ok(()),
            _ => Err(Error::<T>::InvalidSignature),
        }
    }

    pub(crate) fn ocw_fetch<U: AsRef<str>>(url: U) -> Result<Vec<u8>, Error<T>> {
        let url = url.as_ref();

        let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(3_000));

        let request = http::Request::get(url);

        let pending = request
            .add_header("User-Agent", USER_AGENT)
            .deadline(deadline)
            .send()
            .map_err(|_| Error::<T>::HttpFetchingError)?;

        let response = pending
            .try_wait(deadline)
            .map_err(|_| Error::<T>::HttpFetchingError)?
            .map_err(|_| Error::<T>::HttpFetchingError)?;

        if response.code != 200 {
            tracing::warn!("Unexpected status code: {}", response.code);
            return Err(Error::<T>::HttpFetchingError);
        }

        Ok(response.body().collect::<Vec<u8>>())
    }
}
