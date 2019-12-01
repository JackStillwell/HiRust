use chrono::{Date, Datelike, Utc};
use pbr::ProgressBar;
use std::cmp;
use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::hi_rez_constants::{DataConstants, ReturnDataType, UrlConstants};
use crate::models::{GetMatchIdsByQueueReply, PlayerMatchDetails};
use crate::session_manager::SessionManager;
use crate::url_builder;

cfg_if::cfg_if! {
    if #[cfg(test)] {
        use galvanic_test::test_suite;
        use crate::reqwest_wrapper::Wrapper;
        use crate::reqwest_wrapper::MockWrapper as ReqwestWrapper;
    }
    else {
        use crate::reqwest_wrapper::ReqwestWrapper;
    }
}

const VALID_HOURS: [&str; 25] = [
    "-1", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15",
    "16", "17", "18", "19", "20", "21", "22", "23",
];
const VALID_MINUTES: [&str; 7] = ["", "00", "10", "20", "30", "40", "50"];

fn format_date(date: Date<Utc>) -> String {
    format!("{}{:02}{:02}", date.year(), date.month(), date.day(),)
}

fn construct_batch_match_id_string(match_ids: Vec<String>) -> String {
    let mut ret_string = String::from("/");
    for id in match_ids {
        ret_string.push_str(&format!("{},", id));
    }
    ret_string.truncate(ret_string.len() - 1);
    ret_string
}

#[derive(Debug, Clone)]
pub struct GetMatchIdsByQueueRequest {
    pub queue_id: DataConstants,
    pub date: Date<Utc>,
    pub hour: String,
    pub minute: String,
}

pub struct RequestMaker {
    session_manager: Arc<SessionManager>,
    reqwest: Arc<ReqwestWrapper>,
}

impl RequestMaker {
    #[cfg(not(test))]
    pub fn new(session_manager: SessionManager) -> RequestMaker {
        RequestMaker {
            session_manager: Arc::new(session_manager),
            reqwest: Arc::new(ReqwestWrapper {}),
        }
    }

    #[cfg(test)]
    pub fn mock(reqwest: ReqwestWrapper) -> RequestMaker {
        let mut dummy_reqwest = ReqwestWrapper::new();
        dummy_reqwest
            .expect_get_to_text()
            .return_const(Ok(String::from(
            "{ \"ret_msg\": \"Approved\", \"session_id\": \"1234567890\", \"timestamp\": null }",
        )));
        RequestMaker {
            session_manager: Arc::new(SessionManager::mock(dummy_reqwest)),
            reqwest: Arc::new(reqwest),
        }
    }

    pub fn get_match_ids_by_queue(
        &mut self,
        requests: Vec<GetMatchIdsByQueueRequest>,
    ) -> Result<Vec<String>, String> {
        let mut url_optionals: Vec<String> = Vec::new();
        for request in requests {
            let queue_id = request.queue_id;
            let date = request.date;
            let hour = request.hour;
            let minute = request.minute;

            let mut time_window_to_retrieve: String;

            match VALID_HOURS.iter().find(|&&x| x == hour) {
                Some(_) => {}
                None => return Err(String::from("Invalid hour specified")),
            };
            match VALID_MINUTES.iter().find(|&&x| x == minute) {
                Some(_) => {}
                None => return Err(String::from("Invalid minute specified")),
            };

            if &hour == "-1" && &minute == "" {
                time_window_to_retrieve = String::from("-1");
            } else if &hour == "-1" && &minute != "" {
                return Err(String::from("Invalid combination of hour and minute"));
            } else {
                time_window_to_retrieve = format!("{},{}", hour, minute);
            }
            url_optionals.push(format!(
                "/{}/{}/{}",
                queue_id.val(),
                format_date(date),
                time_window_to_retrieve
            ));
        }

        let responses = self.concurrent_reqwest(UrlConstants::GetMatchIdsByQueue, url_optionals);

        let mut all_ids: Vec<String> = Vec::new();
        for response_text in responses {
            let replies: Vec<GetMatchIdsByQueueReply> =
                match serde_json::from_str(&response_text.clone()) {
                    Ok(json) => json,
                    Err(msg) => {
                        let mut file = File::create("debug_dump.json").unwrap();
                        file.write_all(response_text.as_bytes()).unwrap();
                        return Err(format!(
                            "Error deserializing get match ids by queue reply: {}",
                            msg
                        ));
                    }
                };

            match &replies[0].ret_msg {
                Some(msg) => return Err(format!("GetMatchIdsByQueue Request Error: {}", msg)),
                None => {}
            };

            let mut replies: Vec<String> = replies
                .into_iter()
                .filter_map(|x| match x.Active_Flag {
                    Some('n') => x.Match,
                    _ => None,
                })
                .collect();

            all_ids.append(&mut replies);
        }

        Ok(all_ids)
    }

    pub fn get_match_details(
        &self,
        mut match_ids: Vec<String>,
    ) -> Result<Vec<Result<PlayerMatchDetails, String>>, String> {
        let match_ids_len: f32 = match_ids.len() as f32;
        let num_urls_needed: f32 = match_ids_len / 10_f32;
        let num_urls_needed: usize = num_urls_needed.ceil() as usize;

        let mut id_strings = vec![];
        for _ in 0..num_urls_needed {
            let limit = cmp::min(match_ids.len(), 10);
            let ids = match_ids.drain(..limit).collect();
            id_strings.push(construct_batch_match_id_string(ids));
        }

        let responses = self.concurrent_reqwest(UrlConstants::GetMatchDetailsBatch, id_strings);

        let mut replies: Vec<Result<PlayerMatchDetails, String>> = Vec::new();
        for response in responses {
            let mut reply: Vec<PlayerMatchDetails> = match serde_json::from_str(&response) {
                Ok(json) => json,
                Err(msg) => {
                    let mut file = File::create("debug_dump.json").unwrap();
                    file.write_all(response.as_bytes()).unwrap();
                    replies.push(Err(format!(
                        "Error deserializing get match details batch reply: {}",
                        msg
                    )));
                    continue;
                }
            };
            replies.append(&mut reply.into_iter().map(|x| Ok(x)).collect());
        }

        // allow the response to be empty
        // this may happen because too many matches were played in the q that day
        // should probably update this to detect that and get it hourly
        if replies.len() > 0 {
            match &replies[0] {
                Ok(reply) => match &reply.ret_msg {
                    Some(msg) => return Err(format!("GetMatchDetails Request Error: {}", msg)),
                    None => {}
                },
                Err(_) => {}
            };
        }

        Ok(replies)
    }

    pub fn concurrent_reqwest(
        &self,
        endpoint: UrlConstants,
        mut url_optionals: Vec<String>,
    ) -> Vec<String> {
        let arc_endpoint = Arc::new(endpoint);
        let responses = Arc::new(Mutex::new(Vec::new()));
        let mut pb = ProgressBar::new(url_optionals.len() as u64);

        let num_requests: f32 = url_optionals.len() as f32;
        let num_groups_needed: f32 = num_requests / 45_f32;
        let num_groups_needed: usize = num_groups_needed.ceil() as usize;

        let mut request_groups = vec![];
        for _ in 0..num_groups_needed {
            let limit = cmp::min(url_optionals.len(), 45);
            let urls: Vec<String> = url_optionals.drain(..limit).collect();
            request_groups.push(urls);
        }

        for request_group in request_groups {
            let mut handles = vec![];
            for url_optional in request_group {
                let session_manager = Arc::clone(&self.session_manager);
                let reqwest = Arc::clone(&self.reqwest);
                let endpoint = Arc::clone(&arc_endpoint);
                let responses = Arc::clone(&responses);

                let handle = thread::spawn(move || {
                    let mut response_text: String;
                    loop {
                        let session_key = match session_manager.get_session_key_concurrent() {
                            Ok(key) => key,
                            Err(msg) => {
                                println!("{}", msg);
                                return;
                            }
                        };
                        let url = url_builder::url(
                            &session_manager.credentials.dev_id,
                            &session_manager.credentials.dev_key,
                            &String::from(session_key.clone()),
                            &session_manager.base_url,
                            &(*endpoint),
                            &ReturnDataType::Json,
                            &url_optional,
                        );

                        response_text = match reqwest.get_to_text(url) {
                            Ok(text) => text,
                            Err(msg) => {
                                println!("{}", msg);
                                return;
                            }
                        };

                        if response_text.contains("Invalid session id") {
                            session_manager.remove_invalid_session(session_key);
                        } else {
                            session_manager.replace_session(session_key);
                            break;
                        }
                    }

                    responses.lock().unwrap().push(response_text);
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.join().unwrap();
                pb.inc();
            }
        }

        pb.finish();
        let to_ret = (*responses.lock().unwrap()).clone();
        to_ret
    }
}

#[cfg(test)]
test_suite! {
    name test_request_maker;
    use super::*;
    use chrono::TimeZone;
    use crate::test_responses;

    test format_date_correct() {
        let expected_string = String::from("20190810");
        let date = Utc.ymd(2019, 8, 10);
        let generated_string: String = format_date(date);
        assert_eq!(generated_string, expected_string);
    }

    test construct_batch_match_id_string_correct() {
        let match_ids: Vec<String> = vec!["1", "2", "3"].into_iter().map(|x| x.to_string()).collect();
        let expected_string = String::from("/1,2,3");
        assert_eq!(expected_string, construct_batch_match_id_string(match_ids));
    }

    fixture match_ids_reqwest() -> RequestMaker {
        setup(&mut self) {
            let mut reqwest = ReqwestWrapper::new();
            reqwest.expect_get_to_text().returning(|_x| Ok(String::from(test_responses::GET_MATCH_IDS_BY_QUEUE)));
            RequestMaker::mock(reqwest)
        }
    }

    fixture time_combos(hour: String, minute: String, response: String) -> String {
        params {
            vec![
                (String::from("0"),String::from("00"), String::from("956598608")),
                (String::from("-1"),String::from(""), String::from("956598608")),
                (String::from("-1"),String::from("20"), String::from("Invalid combination of hour and minute")),
                (String::from("-2"),String::from("20"), String::from("Invalid hour specified")),
                (String::from("0"),String::from("23"), String::from("Invalid minute specified")),
            ].into_iter()
        }
        setup(&mut self) {
            String::from(self.response)
        }
    }

    test get_match_ids_by_queue_valid_time(time_combos, match_ids_reqwest) {
        let mut request_maker = match_ids_reqwest.val;
        let replies = request_maker
            .get_match_ids_by_queue(vec![GetMatchIdsByQueueRequest {
                queue_id: DataConstants::RankedConquest,
                date: Utc.ymd(2019, 8, 5),
                hour: String::from(time_combos.params.hour),
                minute: String::from(time_combos.params.minute),
            }]);

        match replies {
            Ok(response) => assert_eq!(response[0], time_combos.val),
            Err(response) => assert_eq!(response, time_combos.val)
        }
    }

    fixture multiple_match_ids(request: Vec<GetMatchIdsByQueueRequest>, response_len: u8) -> u8 {
        params {
            vec![
                (
                    vec![GetMatchIdsByQueueRequest {
                        queue_id: DataConstants::RankedConquest,
                        date: Utc.ymd(2019, 8, 5),
                        hour: String::from("0"),
                        minute: String::from("00"),
                    }],
                    20
                ),
                (
                    vec![
                        GetMatchIdsByQueueRequest {
                            queue_id: DataConstants::RankedConquest,
                            date: Utc.ymd(2019, 8, 5),
                            hour: String::from("0"),
                            minute: String::from("00"),
                        },
                        GetMatchIdsByQueueRequest {
                            queue_id: DataConstants::RankedConquest,
                            date: Utc.ymd(2019, 8, 5),
                            hour: String::from("0"),
                            minute: String::from("00"),
                        }
                    ],
                    40
                )
            ].into_iter()
        }
        setup(&mut self) {
            *self.response_len
        }
    }

    test get_match_ids_by_queue(match_ids_reqwest, multiple_match_ids) {
        let mut request_maker = match_ids_reqwest.val;
        let replies = request_maker
            .get_match_ids_by_queue((*multiple_match_ids.params.request).clone())
            .unwrap();

        assert_eq!(replies.len(), multiple_match_ids.val as usize);
    }

    fixture num_ids(request_vec: Vec<String>, num_calls: u8, response_len: usize) -> usize {
        params {
            vec![
                (vec!["match_id"; 30].into_iter().map(|x| x.to_string()).collect(), 3, 30),
                (vec!["match_id"; 31].into_iter().map(|x| x.to_string()).collect(), 4, 40),
            ].into_iter()
        }
        setup(&mut self) {
            *self.response_len
        }
    }

    test get_match_details_multiple_ids(num_ids) {
        let mut reqwest = ReqwestWrapper::new();

        // tests that 30 ids leads to 3 calls
        reqwest.expect_get_to_text()
               .times(*num_ids.params.num_calls as usize)
               .returning(|_x| Ok(String::from(test_responses::GET_MATCH_DETAILS)));

        let request_maker = RequestMaker::mock(reqwest);

        let replies = request_maker
            .get_match_details((*num_ids.params.request_vec).clone())
            .unwrap();

        assert_eq!(replies.len(), num_ids.val);
    }
}
