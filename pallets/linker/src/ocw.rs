use crate::{did, types, Call, Config, Error, HeightOf, Pallet, PendingOf};
use codec::Encode;
use frame_support::dispatch::DispatchResult;
use frame_system::offchain::{SendTransactionTypes, SubmitTransaction};
use parami_ocw::{submit_unsigned, Pallet as Ocw};
use sp_runtime_interface::runtime_interface;
use sp_std::prelude::*;

#[runtime_interface]
pub trait Images {
    fn decode_jpeg(data: &[u8]) -> Option<types::RawImage> {
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

impl<T: Config + SendTransactionTypes<Call<T>>> Pallet<T> {
    pub fn ocw_begin_block(block_number: HeightOf<T>) -> DispatchResult {
        use parami_traits::types::Network::*;

        for site in [Telegram, Twitter] {
            let pending = <PendingOf<T>>::iter_prefix(site);

            for (did, task) in pending {
                if task.deadline <= block_number {
                    // call to remove
                    Self::ocw_submit_link(did, site, task.task, false);

                    return Err(Error::<T>::Deadline)?;
                }

                if task.created < block_number {
                    // only start once (at created + 1)
                    continue;
                }

                let profile = sp_std::str::from_utf8(&task.task).unwrap_or_default();

                let result = match site {
                    Telegram => Self::ocw_verify_telegram(did, profile),
                    Twitter => Self::ocw_verify_twitter(did, profile),
                    _ => {
                        // drop unsupported sites
                        Self::ocw_submit_link(did, site, task.task, false);

                        continue;
                    }
                };

                if let Ok(()) = result {
                    Self::ocw_submit_link(did, site, task.task, true);
                }
            }
        }

        Ok(())
    }

    pub(super) fn ocw_submit_link(
        did: T::DecentralizedId,
        site: parami_traits::types::Network,
        profile: Vec<u8>,
        validated: bool,
    ) {
        let call = Call::submit_link {
            did,
            site,
            profile,
            validated,
        };

        let _ = submit_unsigned!(call);
    }

    pub(super) fn ocw_verify_telegram<U: AsRef<str>>(
        did: T::DecentralizedId,
        profile: U,
    ) -> DispatchResult {
        let res = Ocw::<T>::ocw_fetch(profile)?;

        let res = sp_std::str::from_utf8(&res).unwrap_or_default();

        let res = res.replace(" ", "");

        let start = res
            .find("<metaproperty=\"og:image\"content=\"")
            .ok_or(Error::<T>::InvalidSignature)?
            + 33;
        let end = res[start..]
            .find("\"")
            .ok_or(Error::<T>::InvalidSignature)?;

        let avatar = &res[start..start + end];

        Self::ocw_check_avatar(avatar, did)
    }

    pub(super) fn ocw_verify_twitter<U: AsRef<str>>(
        did: T::DecentralizedId,
        profile: U,
    ) -> DispatchResult {
        let res = Ocw::<T>::ocw_fetch(profile)?;

        let res = sp_std::str::from_utf8(&res).unwrap_or_default();

        let res = res.replace(" ", "");

        let start = res
            .find("<imgclass=\"ProfileAvatar-image\"src=\"")
            .ok_or(Error::<T>::InvalidSignature)?
            + 36;
        let end = res[start..]
            .find("\"")
            .ok_or(Error::<T>::InvalidSignature)?;

        let avatar = &res[start..start + end];

        Self::ocw_check_avatar(avatar, did)
    }

    pub(self) fn ocw_check_avatar<U: AsRef<str>>(
        avatar: U,
        did: T::DecentralizedId,
    ) -> DispatchResult {
        let res = Ocw::<T>::ocw_fetch(avatar)?;

        let did = did.encode();

        match did::parse(&res) {
            Some(res) if res == did => Ok(()),
            _ => Err(Error::<T>::InvalidSignature)?,
        }
    }
}
