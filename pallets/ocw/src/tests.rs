use crate::{mock::*, USER_AGENT};
use sp_core::offchain::{testing, OffchainWorkerExt};

#[test]
fn should_ocw_fetch() {
    let url: String = "https://example.com".into();

    let (offchain, state) = testing::TestOffchainExt::new();
    let mut t = new_test_ext();
    t.register_extension(OffchainWorkerExt::new(offchain));

    {
        let mut state = state.write();
        state.expect_request(testing::PendingRequest {
            method: "GET".into(),
            uri: url.clone(),
            headers: vec![("User-Agent".into(), USER_AGENT.into())],
            response: Some(b"Example Domain".to_vec()),
            sent: true,
            ..Default::default()
        });
    }

    t.execute_with(|| {
        let result = Ocw::ocw_fetch(url).unwrap();

        assert_eq!(result, b"Example Domain".to_vec());
    });
}
