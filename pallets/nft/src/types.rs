use codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct ClassData<Balance> {
    pub deposit: Balance,
    pub metadata: Vec<u8>,
    pub token_type: TokenType,
    pub collection_type: CollectionType,
}

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct AdsSlot<AccountId, Balance, BlockNumber> {
    pub start_time: BlockNumber,
    pub end_time: BlockNumber,
    pub deposit: Balance,
    pub media: Vec<u8>,
    pub owner: AccountId,
}

#[derive(Clone, Decode, Default, Encode, Eq, PartialEq, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct AssetData<Balance> {
    pub deposit: Balance,
    pub name: Vec<u8>,
    pub description: Vec<u8>,
}

#[derive(Clone, Copy, Decode, Encode, Eq, Ord, PartialEq, PartialOrd, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum CollectionType {
    Collectable,
    Executable,
}

impl CollectionType {
    pub fn is_collectable(&self) -> bool {
        match *self {
            CollectionType::Collectable => true,
            _ => false,
        }
    }

    pub fn is_executable(&self) -> bool {
        match *self {
            CollectionType::Executable => true,
            _ => false,
        }
    }
}

impl Default for CollectionType {
    fn default() -> Self {
        CollectionType::Collectable
    }
}

#[derive(Clone, Copy, Decode, Encode, Eq, Ord, PartialEq, PartialOrd, RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum TokenType {
    Transferable,
    BoundToAddress,
}

impl TokenType {
    pub fn is_transferable(&self) -> bool {
        match *self {
            TokenType::Transferable => true,
            _ => false,
        }
    }
}

impl Default for TokenType {
    fn default() -> Self {
        TokenType::BoundToAddress
    }
}
