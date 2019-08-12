use chrono::{Date, Datelike, Utc};
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
        use crate::reqwest_wrapper::MockReqwestWrapper as ReqwestWrapper;
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

pub struct GetMatchIdsByQueueRequest {
    queue_id: DataConstants,
    date: Date<Utc>,
    hour: String,
    minute: String,
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
    ) -> Result<Vec<PlayerMatchDetails>, String> {
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

        let mut replies: Vec<PlayerMatchDetails> = Vec::new();
        for response in responses {
            let mut reply: Vec<PlayerMatchDetails> = match serde_json::from_str(&response) {
                Ok(json) => json,
                Err(msg) => {
                    let mut file = File::create("debug_dump.json").unwrap();
                    file.write_all(response.as_bytes()).unwrap();
                    return Err(format!(
                        "Error deserializing get match details batch reply: {}",
                        msg
                    ));
                }
            };
            replies.append(&mut reply);
        }

        match &replies[0].ret_msg {
            Some(msg) => return Err(format!("GetMatchDetails Request Error: {}", msg)),
            None => {}
        };

        Ok(replies)
    }

    fn concurrent_reqwest(
        &self,
        endpoint: UrlConstants,
        url_optionals: Vec<String>,
    ) -> Vec<String> {
        let mut handles = vec![];
        let arc_endpoint = Arc::new(endpoint);
        let responses = Arc::new(Mutex::new(Vec::new()));
        for url_optional in url_optionals {
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
        }

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

    test get_match_ids_by_queue() {
        let mut reqwest = ReqwestWrapper::new();
        reqwest.expect_get_to_text().returning(|_x| Ok(String::from(test_responses::GET_MATCH_IDS_BY_QUEUE)));
        let mut request_maker = RequestMaker::mock(reqwest);
        let replies = request_maker
            .get_match_ids_by_queue(vec![GetMatchIdsByQueueRequest {
                queue_id: DataConstants::RankedConquest,
                date: Utc.ymd(2019, 8, 5),
                hour: String::from("0"),
                minute: String::from("00"),
            }])
            .unwrap();

        assert_eq!(replies[0], String::from("956598608"));
    }

    test get_match_details() {
        let mut reqwest = ReqwestWrapper::new();
        reqwest.expect_get_to_text().returning(|_x| Ok(String::from(test_responses::GET_MATCH_DETAILS)));
        let request_maker = RequestMaker::mock(reqwest);

        let replies = request_maker
            .get_match_details(vec![String::from("956598608")])
            .unwrap();

        assert_eq!(replies[0].playerId, Some(String::from("4203198")));
    }
}
