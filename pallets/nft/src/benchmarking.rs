use super::*;

#[allow(unused)]
use crate::Pallet as Did;
use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_system::RawOrigin;

benchmarks! {
    mint {
        //
    }: _(RawOrigin::Signed(caller), referer)
    verify {
        //
    }
}

impl_benchmark_test_suite!(Did, crate::mock::new_test_ext(), crate::mock::Test);
