use crate::{is_stask, types, Call, Config, Error, Pallet, PendingOf};
use frame_system::offchain::{CreateSignedTransaction, SubmitTransaction};
use sp_runtime::offchain::{http, Duration};
use sp_std::prelude::*;

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
                let call = Call::submit_link_unsigned {
                    did,
                    site,
                    profile: Default::default(),
                    ok: false,
                };

                let _ = submit_unsigned!(call);

                return Err(Error::<T>::Deadline);
            };

            let result = match site {
                Telegram if is_stask!(task.profile, b"https://t.me/") => {
                    Self::ocw_link_telegram(did, task.profile.clone())
                }
                _ => {
                    // drop unsupported sites
                    let call = Call::submit_link_unsigned {
                        did,
                        site,
                        profile: Default::default(),
                        ok: false,
                    };

                    let _ = submit_unsigned!(call);

                    continue;
                }
            };

            if result.is_ok() {
                let call = Call::submit_link_unsigned {
                    did,
                    site,
                    profile: task.profile,
                    ok: true,
                };

                let _ = submit_unsigned!(call);
            }
        }

        Ok(())
    }

    fn ocw_link_telegram<U: AsRef<[u8]>>(
        did: T::DecentralizedId,
        profile: U,
    ) -> Result<(), Error<T>> {
        let res = Self::ocw_fetch(profile)?;

        let res = sp_std::str::from_utf8(&res).map_err(|_| {
            log::warn!("No UTF8 body");
            Error::<T>::HttpFetchingError
        })?;

        log::info!("{}", res);

        let data = Self::generate_message(&did);
        let data = sp_std::str::from_utf8(&data).map_err(|_| {
            log::warn!("No UTF8 body");
            Error::<T>::HttpFetchingError
        })?;

        log::info!("{}", data);

        if res.contains(data) {
            Ok(())
        } else {
            Err(Error::<T>::InvalidSignature)
        }
    }

    fn ocw_fetch<U: AsRef<[u8]>>(url: U) -> Result<Vec<u8>, Error<T>> {
        let url = url.as_ref();
        let url = sp_std::str::from_utf8(url).map_err(|e| {
            log::error!("{:?}", e);
            Error::<T>::HttpFetchingError
        })?;

        let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(3_000));

        let request = http::Request::get(url);

        let pending = request.deadline(deadline).send().map_err(|e| {
            log::error!("{:?}", e);
            Error::<T>::HttpFetchingError
        })?;

        let response = pending
            .try_wait(deadline)
            .map_err(|e| {
                log::error!("{:?}", e);
                Error::<T>::HttpFetchingError
            })?
            .map_err(|e| {
                log::error!("{:?}", e);
                Error::<T>::HttpFetchingError
            })?;

        if response.code != 200 {
            log::warn!("Unexpected status code: {}", response.code);
            return Err(Error::<T>::HttpFetchingError);
        }

        Ok(response.body().collect::<Vec<u8>>())
    }
}
