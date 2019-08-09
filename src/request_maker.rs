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

pub fn reqwest_to_text(url: String) -> String {
    let response_result = reqwest::get(&url);

    let mut response = match response_result {
        Ok(response) => response,
        Err(msg) => panic!("Error reqwesting session: {}", msg),
    };

    match response.text() {
        Ok(text) => text,
        Err(msg) => panic!("Error decoding response: {}", msg),
    }
}

pub struct RequestMaker {
    session_manager: Arc<SessionManager>,
}

impl RequestMaker {
    pub fn get_match_ids_by_queue(
        &mut self,
        queue_id: DataConstants,
        date: Date<Utc>,
        hour: String,
        minute: String,
    ) -> Vec<String> {
        let mut time_window_to_retrieve: String;

        match VALID_HOURS.iter().find(|&&x| x == hour) {
            Some(_) => {}
            None => panic!("Invalid hour specified"),
        };
        match VALID_MINUTES.iter().find(|&&x| x == minute) {
            Some(_) => {}
            None => panic!("Invalid minute specified"),
        };

        if &hour == "-1" && &minute == "" {
            time_window_to_retrieve = String::from("-1");
        } else if &hour == "-1" && &minute != "" {
            panic!("Invalid combination of hour and minute");
        } else {
            time_window_to_retrieve = format!("{},{}", hour, minute);
        }

        let session_key = self.session_manager.get_session_key().unwrap();
        let url = url_builder::url(
            &self.session_manager.credentials.dev_id,
            &self.session_manager.credentials.dev_key,
            &session_key,
            &self.session_manager.base_url,
            &UrlConstants::GetMatchIdsByQueue,
            &ReturnDataType::Json,
            &format!(
                "/{}/{}/{}",
                queue_id.val(),
                format_date(date),
                time_window_to_retrieve
            ),
        );

        let response_text: String = reqwest_to_text(url);

        self.session_manager.replace_session(session_key);

        let replies: Vec<GetMatchIdsByQueueReply> =
            match serde_json::from_str(&response_text.clone()) {
                Ok(json) => json,
                Err(_) => panic!("Error deserializing get match ids by queue reply"),
            };

        match &replies[0].ret_msg {
            Some(msg) => panic!(format!("GetMatchIdsByQueue Request Error: {}", msg)),
            None => {}
        };

        replies
            .into_iter()
            .filter_map(|x| match x.Active_Flag {
                Some('n') => x.Match,
                _ => None,
            })
            .collect()
    }

    pub fn get_match_details(&mut self, match_id: String) -> Vec<PlayerMatchDetails> {
        let session_key = self.session_manager.get_session_key().unwrap();
        let url = url_builder::url(
            &self.session_manager.credentials.dev_id,
            &self.session_manager.credentials.dev_key,
            &session_key,
            &self.session_manager.base_url,
            &UrlConstants::GetMatchDetails,
            &ReturnDataType::Json,
            &format!("/{}", match_id),
        );

        let response_text: String = reqwest_to_text(url);

        self.session_manager.replace_session(session_key);

        let replies: Vec<PlayerMatchDetails> = match serde_json::from_str(&response_text.clone()) {
            Ok(json) => json,
            Err(msg) => panic!(format!(
                "Error deserializing get match details reply: {}",
                msg
            )),
        };

        match &replies[0].ret_msg {
            Some(msg) => panic!(format!("GetMatchDetails Request Error: {}", msg)),
            None => {}
        };

        replies
    }

    pub fn get_match_details_batch(&self, mut match_ids: Vec<String>) -> Vec<PlayerMatchDetails> {
        let match_ids_len: f32 = match_ids.len() as f32;
        let num_urls_needed: f32 = match_ids_len / 10_f32;
        let num_urls_needed: usize = num_urls_needed.ceil() as usize;

        let mut id_strings = vec![];
        for _ in 0..num_urls_needed {
            let limit = cmp::min(match_ids.len(), 10);
            let ids = match_ids.drain(..limit).collect();
            id_strings.push(construct_batch_match_id_string(ids));
        }

        let mut handles = vec![];
        let responses = Arc::new(Mutex::new(Vec::new()));
        for id_string in id_strings {
            let session_manager = Arc::clone(&self.session_manager);
            let responses = Arc::clone(&responses);
            let handle = thread::spawn(move || {
                let session_key = session_manager.get_session_key_concurrent();
                let url = url_builder::url(
                    &session_manager.credentials.dev_id,
                    &session_manager.credentials.dev_key,
                    &String::from(session_key.clone()),
                    &session_manager.base_url,
                    &UrlConstants::GetMatchDetailsBatch,
                    &ReturnDataType::Json,
                    &id_string,
                );

                let response_text = reqwest_to_text(url);
                responses.lock().unwrap().push(response_text);
                session_manager.replace_session(session_key);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let responses = (*responses.lock().unwrap()).clone();
        let mut replies: Vec<PlayerMatchDetails> = Vec::new();
        for response in responses {
            let mut reply: Vec<PlayerMatchDetails> = match serde_json::from_str(&response) {
                Ok(json) => json,
                Err(msg) => {
                    let mut file = File::create("debug_dump.json").unwrap();
                    file.write_all(response.as_bytes()).unwrap();
                    panic!(format!(
                        "Error deserializing get match details batch reply: {}",
                        msg
                    ));
                }
            };
            replies.append(&mut reply);
        }

        match &replies[0].ret_msg {
            Some(msg) => panic!(format!("GetMatchDetails Request Error: {}", msg)),
            None => {}
        };

        replies
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_manager::Auth;
    use chrono::TimeZone;

    // #[test]
    // fn test_get_match_ids_by_queue() {
    //     let auth = Auth::from_file("../hirez-dev-credentials.txt");
    //     let session_manager = SessionManager::new(auth, UrlConstants::UrlBase);
    //     let mut request_maker = RequestMaker {
    //         session_manager: Arc::new(session_manager),
    //     };
    //     let replies = request_maker.get_match_ids_by_queue(
    //         DataConstants::RankedConquest,
    //         Utc.ymd(2019, 8, 5),
    //         String::from("0"),
    //         String::from("00"),
    //     );

    //     assert_eq!(replies[0], String::from("956598608"));
    // }

    // #[test]
    // fn test_get_match_details() {
    //     let auth = Auth::from_file("../hirez-dev-credentials.txt");
    //     let session_manager = SessionManager::new(auth, UrlConstants::UrlBase);
    //     let mut request_maker = RequestMaker {
    //         session_manager: Arc::new(session_manager),
    //     };
    //     let replies = request_maker.get_match_details(String::from("956598608"));

    //     assert_eq!(replies[0].playerId, Some(String::from("4203198")));
    // }

    #[test]
    fn test_get_match_details_batch() {
        let auth = Auth::from_file("../hirez-dev-credentials.txt");
        let session_manager = SessionManager::new(auth, UrlConstants::UrlBase);
        let mut request_maker = RequestMaker {
            session_manager: Arc::new(session_manager),
        };
        let replies = request_maker.get_match_ids_by_queue(
            DataConstants::RankedConquest,
            Utc.ymd(2019, 8, 5),
            String::from("-1"),
            String::from(""),
        );

        let replies = request_maker.get_match_details_batch(replies);

        assert_eq!(replies.len(), 30880);
    }
}
