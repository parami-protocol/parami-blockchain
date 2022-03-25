use crate::{types, Config, DidOf, Error, Event, Verified, Pallet, PendingTasks, Vetoed};

use frame_support::ensure;
use sp_runtime::{ DispatchResult};
use sp_std::prelude::*;

impl<T: Config> Pallet<T> {
    fn ensure_ipfs(did: &DidOf<T>, ipfs: &[u8]) -> DispatchResult {
        ensure!(!<Verified<T>>::contains_key(did,ipfs), Error::<T>::Exists);
        Ok(())
    }

    pub fn veto_pending(
        did: DidOf<T>,
        ipfs: Vec<u8>,
        range: Vec<u8>,
    ) -> DispatchResult {
        <PendingTasks<T>>::remove( &ipfs);
        let created = <frame_system::Pallet<T>>::block_number();
        <Vetoed<T>>::insert(did, types::Proof{
            ipfs,
            range,
            created,
        });
        Self::deposit_event(Event::<T>::VerifyFailed(did));
        Ok(())
    }

    pub fn insert_verified(
        did: DidOf<T>,
        ipfs: Vec<u8>,
        range: Vec<u8>,
    ) -> DispatchResult {
        <PendingTasks<T>>::remove(&ipfs);
        let created = <frame_system::Pallet<T>>::block_number();
        <Verified<T>>::insert(did, &ipfs, types::Proof{
            ipfs:ipfs.clone(),
            range,
            created,
        });
        Self::deposit_event(Event::<T>::VerifyOk(did));
        Ok(())
    }

    pub fn insert_pending(
        did: DidOf<T>,
        ipfs: Vec<u8>,
    ) -> DispatchResult {
        use frame_support::traits::Get;
        use sp_runtime::traits::Saturating;

        Self::ensure_ipfs(&did, &ipfs)?;

        ensure!(
            !<PendingTasks<T>>::contains_key(&ipfs),
            Error::<T>::Exists
        );

        let created = <frame_system::Pallet<T>>::block_number();
        let lifetime = T::PendingLifetime::get();
        let deadline = created.saturating_add(lifetime);

        <PendingTasks<T>>::insert(
            &ipfs,
            types::Pending {
                did,
                ipfs:ipfs.clone(),
                deadline,
                created,
            },
        );

        Ok(())
    }
}
