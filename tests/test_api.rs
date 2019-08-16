use galvanic_test::test_suite;

test_suite! {
    name integration_test_api;
    use hirust::api::SmiteAPI;

    #[ignore]
    test get_gods() {
        let api = SmiteAPI::new("../hirez-dev-credentials.txt".to_string());
        let gods = api.get_gods();
        assert_eq!(gods.len(), 104);
    }

    #[ignore]
    test get_items() {
        let api = SmiteAPI::new("../hirez-dev-credentials.txt".to_string());
        let gods = api.get_items();
        assert_eq!(gods.len(), 272);
    }
}