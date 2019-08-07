use chrono::Utc;
use rand::{thread_rng, Rng};
use reqwest;
use serde_json;
use std::collections::VecDeque;
use std::convert::TryInto;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::sync::Mutex;
use std::thread::sleep;
use std::time::Duration;

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

#[derive(Clone)]
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
    idle_sessions: Mutex<VecDeque<Session>>,
    active_sessions: Mutex<Vec<Session>>,
    sessions_created: Mutex<u16>,
    valid_session_count: Mutex<u8>,
    dev_id: String,
    dev_key: String,
    base_url: UrlConstants,
}

impl SessionManager {
    pub fn new(dev_id: String, dev_key: String, base_url: UrlConstants) -> SessionManager {
        SessionManager {
            idle_sessions: Mutex::new(VecDeque::new()),
            active_sessions: Mutex::new(Vec::new()),
            sessions_created: Mutex::new(0),
            valid_session_count: Mutex::new(0),
            dev_id,
            dev_key,
            base_url,
        }
    }

    /*
     * Retrieves the first valid session, creating if possible and discarding any invalid sessions
     */
    pub fn get_session_key(&mut self) -> Option<String> {
        let mut valid_session_count = self.valid_session_count.lock().unwrap();
        let num_sessions: u16 = (*valid_session_count).try_into().unwrap();
        let mut sessions_created = self.sessions_created.lock().unwrap();
        let mut idle_sessions = self.idle_sessions.lock().unwrap();
        let mut active_sessions = self.active_sessions.lock().unwrap();

        // check every session in idle_sessions and if valid, return
        //   if not valid, discard and look for the next one
        while let Some(session) = idle_sessions.pop_front() {
            match session.is_valid() {
                true => {
                    let key = session.session_key.clone();
                    active_sessions.push(session);
                    return Some(key);
                }
                false => {
                    *valid_session_count -= 1;
                }
            }
        }

        // if there are no sessions in idle_sessions
        match *sessions_created < LimitConstants::SessionsPerDay.val() {
            true => match num_sessions < LimitConstants::ConcurrentSessions.val() {
                true => {
                    let new_session = self.create_session();
                    let key = new_session.session_key.clone();
                    active_sessions.push(new_session);
                    *valid_session_count += 1;
                    *sessions_created += 1;
                    Some(key)
                }
                false => None,
            },
            false => None,
        }
    }

    pub fn get_session_key_concurrent(&mut self) -> String {
        loop {
            let mut rng = thread_rng();
            match self.get_session_key() {
                Some(key) => return key,
                // sleep for one second and between 0 and 5 nanoseconds
                None => sleep(Duration::new(1, rng.gen_range(0, 5))),
            };
        }
    }

    pub fn replace_session(&mut self, session_key: String) {
        let mut active_sessions = self.active_sessions.lock().unwrap();
        let mut idle_sessions = self.idle_sessions.lock().unwrap();
        let index = active_sessions
            .iter()
            .position(|x| x.session_key == session_key)
            .unwrap();
        idle_sessions.push_back(active_sessions[index].clone());
        active_sessions.remove(index);
    }

    fn create_session(&self) -> Session {
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

        if json.ret_msg != "Approved" {
            panic!(format!("CreateSession Request Error: {}", json.ret_msg));
        }

        Session {
            session_key: json.session_id,
            creation_timestamp: Utc::now().timestamp(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_get_replace_session() {
        let auth = Auth::from_file("../hirez-dev-credentials.txt");
        let mut session_manager = SessionManager::new(auth.dev_id, auth.dev_key, UrlConstants::UrlBase);

        assert!({*session_manager.sessions_created.lock().unwrap() == 0});
        assert!({*session_manager.valid_session_count.lock().unwrap() == 0});
        assert!({session_manager.active_sessions.lock().unwrap().len() == 0});
        assert!({session_manager.idle_sessions.lock().unwrap().len() == 0});

        let key = session_manager.get_session_key().unwrap();
        assert!(key != "");

        assert!({*session_manager.sessions_created.lock().unwrap() == 1});
        assert!({*session_manager.valid_session_count.lock().unwrap() == 1});
        assert!({session_manager.active_sessions.lock().unwrap().len() == 1});
        assert!({session_manager.idle_sessions.lock().unwrap().len() == 0});

        let first_key = key.clone();
        session_manager.replace_session(key);

        assert!({*session_manager.sessions_created.lock().unwrap() == 1});
        assert!({*session_manager.valid_session_count.lock().unwrap() == 1});
        assert!({session_manager.active_sessions.lock().unwrap().len() == 0});
        assert!({session_manager.idle_sessions.lock().unwrap().len() == 1});

        let first_key_again = session_manager.get_session_key().unwrap();
        assert_eq!(first_key, first_key_again);

        assert!({*session_manager.sessions_created.lock().unwrap() == 1});
        assert!({*session_manager.valid_session_count.lock().unwrap() == 1});
        assert!({session_manager.active_sessions.lock().unwrap().len() == 1});
        assert!({session_manager.idle_sessions.lock().unwrap().len() == 0});

        let second_key = session_manager.get_session_key().unwrap();
        assert!("" != second_key);
        assert!(first_key != second_key);

        assert!({*session_manager.sessions_created.lock().unwrap() == 2});
        assert!({*session_manager.valid_session_count.lock().unwrap() == 2});
        assert!({session_manager.active_sessions.lock().unwrap().len() == 2});
        assert!({session_manager.idle_sessions.lock().unwrap().len() == 0});

        session_manager.replace_session(second_key);

        assert!({*session_manager.sessions_created.lock().unwrap() == 2});
        assert!({*session_manager.valid_session_count.lock().unwrap() == 2});
        assert!({session_manager.active_sessions.lock().unwrap().len() == 1});
        assert!({session_manager.idle_sessions.lock().unwrap().len() == 1});
    }
}
