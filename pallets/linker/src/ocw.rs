use crate::{did, is_stask, types, Call, Config, Error, Pallet, PendingOf};
use codec::Encode;
use frame_system::offchain::{CreateSignedTransaction, SubmitTransaction};
use sp_runtime::offchain::{http, Duration};
use sp_runtime_interface::runtime_interface;
use sp_std::prelude::*;

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

        let pending = <PendingOf<T>>::iter();

        for (did, site, task) in pending {
            if task.deadline <= block_number {
                // call to remove
                Self::ocw_submit_link(did, site, Vec::<u8>::new(), false);

                return Err(Error::<T>::Deadline);
            };

            if task.created < block_number {
                // only start once (at created + 1)
                continue;
            };

            let result = match site {
                Telegram if is_stask!(task.profile, b"https://t.me/") => {
                    Self::ocw_link_telegram(did, task.profile.clone())
                }
                _ => {
                    // drop unsupported sites
                    Self::ocw_submit_link(did, site, Vec::<u8>::new(), false);

                    continue;
                }
            };

            if result.is_ok() {
                Self::ocw_submit_link(did, site, task.profile, true);
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
        let call = Call::submit_link_unsigned {
            did,
            site,
            profile,
            ok,
        };

        let _ = submit_unsigned!(call);
    }

    pub(crate) fn ocw_link_telegram<U: AsRef<[u8]>>(
        did: T::DecentralizedId,
        profile: U,
    ) -> Result<(), Error<T>> {
        let res = Self::ocw_fetch(profile)?;

        let res = sp_std::str::from_utf8(&res).map_err(|_| Error::<T>::HttpFetchingError)?;

        let res = res.replace(" ", "");

        let start = res
            .find("<metaproperty=\"og:image\"content=\"")
            .ok_or(Error::<T>::HttpFetchingError)?;
        let end = res.find(".jpg\"").ok_or(Error::<T>::HttpFetchingError)?;

        let avatar = &res[start + 33..end + 4];

        log::info!("telegram avatar uri: {}", avatar);

        let res = Self::ocw_fetch(avatar)?;

        match did::parse(res) {
            Some(res) if res == did.encode() => Ok(()),
            _ => Err(Error::<T>::InvalidSignature),
        }
    }

    pub(crate) fn ocw_fetch<U: AsRef<[u8]>>(url: U) -> Result<Vec<u8>, Error<T>> {
        let url = url.as_ref();
        let url = sp_std::str::from_utf8(url).map_err(|_| Error::<T>::HttpFetchingError)?;

        let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(3_000));

        let request = http::Request::get(url);

        let pending = request
            .add_header("User-Agent", "ParamiLinker/1.0")
            .deadline(deadline)
            .send()
            .map_err(|_| Error::<T>::HttpFetchingError)?;

        let response = pending
            .try_wait(deadline)
            .map_err(|_| Error::<T>::HttpFetchingError)?
            .map_err(|_| Error::<T>::HttpFetchingError)?;

        if response.code != 200 {
            log::warn!("Unexpected status code: {}", response.code);
            return Err(Error::<T>::HttpFetchingError);
        }

        Ok(response.body().collect::<Vec<u8>>())
    }
}
