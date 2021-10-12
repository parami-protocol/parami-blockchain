use crate::*;

pub const UNIT: Balance = 1_000_000_000_000_000;
pub const MAX_TAG_TYPE_COUNT: u8 = 30;
pub const MAX_TAG_COUNT: usize = 3;
pub const TAG_DENOMINATOR: TagCoefficient = 10;
pub const ADVERTISER_PAYMENT_WINDOW: parami_primitives::Moment = 10 * 1000; //60*60*24*3;
pub const USER_PAYMENT_WINDOW: parami_primitives::Moment = 60 * 60 * 24 * 7;
pub const MAX_TAG_SCORE_DELTA: TagScore = 5;
pub const MIN_TAG_SCORE_DELTA: TagScore = -5;
pub const DAY_MILLION_SECOND: u64 = 1000 * 60 * 60 * 24;
