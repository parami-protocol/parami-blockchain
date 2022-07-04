use sp_runtime::DispatchError;

pub trait Nfts<AccountId> {
    // force transfer all assets of account src to account dest
    fn force_transfer_all_fractions(src: &AccountId, dest: &AccountId)
        -> Result<(), DispatchError>;
}

impl<AccountId> Nfts<AccountId> for () {
    fn force_transfer_all_fractions(
        _src: &AccountId,
        _dest: &AccountId,
    ) -> Result<(), DispatchError> {
        Ok(())
    }
}
