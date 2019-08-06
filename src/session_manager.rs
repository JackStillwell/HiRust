use chrono::Utc;
use reqwest;
use serde_json;
use std::collections::VecDeque;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use crate::hi_rez_constants::{LimitConstants, ReturnDataType, UrlConstants};
use crate::models::CreateSessionReply;
use crate::url_builder;

pub struct Auth {
    dev_id: String,
    dev_key: String,
}

impl Auth {
    pub fn from_file(path: &str) -> Auth {
        // open the file specified by cli input
        let path = Path::new(&path);
        let mut file = File::open(&path).unwrap();

        // read the contents of the file to a string
        let mut s = String::new();
        file.read_to_string(&mut s).unwrap();

        // split the file by line, stripping \n
        let s: Vec<&str> = s.split("\n").collect();

        Auth {
            dev_id: String::from(s[0]),
            dev_key: String::from(s[1]),
        }
    }
}

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

pub struct SessionManager {
    idle_sessions: VecDeque<Session>,
    active_sessions: VecDeque<Session>,
    sessions_created: u16,
    dev_id: String,
    dev_key: String,
    base_url: UrlConstants,
}

impl SessionManager {
    pub fn new(dev_id: String, dev_key: String, base_url: UrlConstants) -> SessionManager {
        SessionManager {
            idle_sessions: VecDeque::new(),
            active_sessions: VecDeque::new(),
            sessions_created: 0,
            dev_id,
            dev_key,
            base_url,
        }
    }

    pub fn get_session(&mut self) -> Result<Session, &str> {
        match self.idle_sessions.pop_front() {
            Some(session) => Ok(session),
            None => Err("No sessions available"),
        }
    }

    fn create_session(&mut self) {
        let url = url_builder::session_url(
            &self.base_url,
            &ReturnDataType::Json,
            &self.dev_id,
            &self.dev_key,
        );

        let response_result = reqwest::get(&url);

        let mut response = match response_result {
            Ok(response) => response,
            Err(_) => panic!("Error Creating Session"),
        };

        let response_text: String = match response.text() {
            Ok(text) => text,
            Err(_) => panic!("Error decoding create session reply"),
        };

        let json: CreateSessionReply = match serde_json::from_str(&response_text.clone()) {
            Ok(json) => json,
            Err(_) => panic!("Error deserializing create session reply"),
        };

        let new_session = Session {
            session_key: json.session_id,
            creation_timestamp: Utc::now().timestamp(),
        };

        self.idle_sessions.push_back(new_session);
        self.sessions_created += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_create_session() {
        let auth = Auth::from_file("../hirez-dev-credentials.txt");
        let mut session_manager =
            SessionManager::new(auth.dev_id, auth.dev_key, UrlConstants::UrlBase);

        session_manager.create_session();

        assert_eq!(session_manager.sessions_created, 1);
        assert_eq!(session_manager.idle_sessions.len(), 1);
    }
}
