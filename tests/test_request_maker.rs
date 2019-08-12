use galvanic_test::test_suite;
use chrono::{Utc, TimeZone};

use hirust::session_manager::{Auth, SessionManager};
use hirust::request_maker::{RequestMaker, GetMatchIdsByQueueRequest};
use hirust::hi_rez_constants::{UrlConstants, DataConstants};

test_suite! {
    name integration_test_request_maker;
    use super::*;

    test bulk_pull() {
        let auth = Auth::from_file("../hirez-dev-credentials.txt");
        let session_manager = SessionManager::new(auth, UrlConstants::UrlBase);
        let mut request_maker = RequestMaker::new(session_manager);

        let ids = request_maker.get_match_ids_by_queue(vec![GetMatchIdsByQueueRequest {
            queue_id: DataConstants::RankedConquest,
            date: Utc.ymd(2019, 8, 10),
            hour: String::from("-1"),
            minute: String::from(""),
        }]).unwrap();

        let match_details = request_maker.get_match_details(ids);

        assert_eq!(true, false);
    }
}