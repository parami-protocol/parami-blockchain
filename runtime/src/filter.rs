use crate::Call;
use codec::{Decode, Encode};
use frame_system as system;
use parami_magic::{self as magic, Pallet as Magic};
use scale_info::TypeInfo;
use sp_runtime::{
    traits::{DispatchInfoOf, SignedExtension},
    transaction_validity::{
        InvalidTransaction, TransactionValidity, TransactionValidityError, ValidTransaction,
    },
};

#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ExtrinsicFilter<T: system::Config + magic::Config + Send + Sync>(
    sp_std::marker::PhantomData<T>,
);

impl<T: system::Config + magic::Config + Send + Sync> sp_std::fmt::Debug for ExtrinsicFilter<T> {
    #[cfg(feature = "std")]
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        write!(f, "ExtrinsicFilter")
    }

    #[cfg(not(feature = "std"))]
    fn fmt(&self, _: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        Ok(())
    }
}

impl<T: system::Config + magic::Config + Send + Sync> ExtrinsicFilter<T> {
    /// Create new `SignedExtension` to check transaction version.
    pub fn new() -> Self {
        Self(sp_std::marker::PhantomData)
    }
}

impl<T: system::Config + magic::Config + Send + Sync> SignedExtension for ExtrinsicFilter<T> {
    const IDENTIFIER: &'static str = "ExtrinsicFilter";

    type AccountId = T::AccountId;
    type Call = Call;
    type AdditionalSigned = ();
    type Pre = ();

    fn additional_signed(&self) -> Result<Self::AdditionalSigned, TransactionValidityError> {
        Ok(())
    }

    fn validate(
        &self,
        who: &Self::AccountId,
        call: &Self::Call,
        _info: &DispatchInfoOf<Self::Call>,
        _len: usize,
    ) -> TransactionValidity {
        match call {
            Call::Assets(pallet_assets::Call::create { .. }) => {
                Err(TransactionValidityError::Invalid(InvalidTransaction::Call))
            }
            Call::Balances(..)
                if Magic::<T>::stable_of(&who).is_some()
                    || Magic::<T>::controller_of(&who).is_some() =>
            {
                Err(TransactionValidityError::Invalid(InvalidTransaction::Call))
            }
            _ => Ok(ValidTransaction::default()),
        }
    }
}
