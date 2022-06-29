use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::U256;
use sp_runtime_interface::pass_by::PassByCodec;

#[derive(PassByCodec, Encode, Decode, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
pub enum ParamType {
    /// Unsigned integer.
    Uint(u32),
}

#[derive(PassByCodec, Encode, Decode, Eq, PartialEq, TypeInfo, MaxEncodedLen)]
pub enum Token {
    /// Unisnged integer.
    ///
    /// solidity name: uint
    Uint(U256),
}

#[cfg(feature = "std")]
impl From<&Token> for ethabi::Token {
    fn from(token: &Token) -> ethabi::Token {
        match token {
            Token::Uint(number) => ethabi::Token::Uint(number.into()),
        }
    }
}

#[cfg(feature = "std")]
impl From<&ParamType> for ethabi::ParamType {
    fn from(param_type: &ParamType) -> ethabi::ParamType {
        match param_type {
            ParamType::Uint(number) => ethabi::ParamType::Uint(*number as usize),
        }
    }
}
