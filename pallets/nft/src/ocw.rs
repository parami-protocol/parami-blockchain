use crate::{Call, Config, Error, Pallet};
use frame_system::offchain::{SendTransactionTypes, SubmitTransaction};
use sp_runtime::{
    offchain::{http, Duration},
    DispatchError,
};

impl<T: Config + SendTransactionTypes<Call<T>>> Pallet<T> {
    pub fn ocw_begin_block(block_number: T::BlockNumber) -> Result<(), DispatchError> {
        //

        Ok(())
    }
}
