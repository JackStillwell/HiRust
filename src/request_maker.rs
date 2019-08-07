use chrono::{DateTime, Datelike, Utc};

use crate::hi_rez_constants::{DataConstants, ReturnDataType, UrlConstants};
use crate::models::GetMatchIdsByQueueReply;
use crate::session_manager::SessionManager;
use crate::url_builder;

const VALID_HOURS: [&str; 25] = [
    "-1", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15",
    "16", "17", "18", "19", "20", "21", "22", "23",
];
const VALID_MINUTES: [&str; 7] = ["", "00", "10", "20", "30", "40", "50"];

fn format_date(date: DateTime<Utc>) -> String {
    format!("{}{:02}{:02}", date.year(), date.month(), date.day(),)
}

pub fn reqwest_to_text(url: String) -> String {
    let response_result = reqwest::get(&url);

    let mut response = match response_result {
        Ok(response) => response,
        Err(_) => panic!("Error reqwesting session"),
    };

    match response.text() {
        Ok(text) => text,
        Err(_) => panic!("Error decoding response"),
    }
}

pub struct RequestMaker {
    session_manager: SessionManager,
    base_url: UrlConstants,
}

impl RequestMaker {
    pub fn get_match_ids_by_queue(
        &mut self,
        queue_id: DataConstants,
        date: DateTime<Utc>,
        hour: String,
        minute: String,
    ) -> GetMatchIdsByQueueReply {
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
            &self.base_url,
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

        let json: GetMatchIdsByQueueReply = match serde_json::from_str(&response_text.clone()) {
            Ok(json) => json,
            Err(_) => panic!("Error deserializing create session reply"),
        };

        if json.ret_msg != "Approved" {
            panic!(format!("CreateSession Request Error: {}", json.ret_msg));
        }

        json
    }
}
