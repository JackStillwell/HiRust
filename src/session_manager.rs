use chrono::Utc;
use reqwest;

use crate::hi_rez_constants::{
    LimitConstants,
    UrlConstants,
    ReturnDataType};
use crate::url_builder;

pub struct Session {
    session_key: String,
    creation_timestamp: i64,
}

impl Session {
    pub fn is_valid(&self) -> bool {
        let seconds_active = Utc::now().timestamp() - self.creation_timestamp;
        return seconds_active < (LimitConstants::SessionTimeLimit.val() as i64);
    }
}

pub struct SessionManager<'a> {
    idle_sessions: Vec<Session>,
    active_sessions: Vec<Session>,
    sessions_created: u16,
    dev_id: &'a str,
    dev_key: &'a str,
    base_url: UrlConstants,
}

impl SessionManager<'_> {
    pub fn get_session(&mut self) -> Result<Session, &str> {
        match self.idle_sessions.pop() {
            Some(session) => Ok(session),
            None => Err("No sessions available")
        }
    }

    fn create_session(&mut self) {
        let url = url_builder::session_url(
            &self.base_url,
            &ReturnDataType::Json,
            self.dev_id, self.dev_key);

        let response_result = reqwest::get(&url);

        match response_result {
            Ok(mut response) => response.text(),
            Err(response) => panic!("not implemented")
        };
    }
}