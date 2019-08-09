use chrono::Utc;
use rand::{thread_rng, Rng};
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
use crate::request_maker;
use crate::url_builder;

const SECONDS_IN_A_DAY: i64 = 86400;

pub struct Auth {
    pub dev_id: String,
    pub dev_key: String,
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
    num_requests: Mutex<u16>,
    pub credentials: Auth,
    pub base_url: UrlConstants,
}

impl Drop for SessionManager {
    fn drop(&mut self) {
        self.store();
    }
}

impl SessionManager {
    pub fn store(&self) {
        let mut active_sessions = self.active_sessions.lock().unwrap();
        let mut idle_sessions = self.idle_sessions.lock().unwrap();

        let mut file = File::create("sessions.txt").unwrap();

        while let Some(session) = active_sessions.pop() {
            let session_str = format!("{} {}\n", session.session_key, session.creation_timestamp);
            file.write_all(session_str.as_bytes()).unwrap();
        }

        while let Some(session) = idle_sessions.pop_front() {
            let session_str = format!("{} {}\n", session.session_key, session.creation_timestamp);
            file.write_all(session_str.as_bytes()).unwrap();
        }
    }

    fn load() -> VecDeque<Session> {
        // open the file specified by cli input
        let path = Path::new("sessions.txt");
        let mut file = match File::open(&path) {
            Ok(f) => f,
            Err(_) => return VecDeque::new(),
        };

        // read the contents of the file to a string
        let mut all_text = String::new();
        file.read_to_string(&mut all_text).unwrap();

        // split the file by line, stripping \n
        let lines: Vec<&str> = all_text.split("\n").collect();

        let mut idle_sessions = VecDeque::new();
        for line in lines {
            let session_vec: Vec<&str> = line.split(" ").collect();
            if session_vec.len() > 1 {
                idle_sessions.push_back(Session {
                    session_key: String::from(session_vec[0]),
                    creation_timestamp: session_vec[1].parse::<i64>().unwrap(),
                });
            }
        }

        idle_sessions
    }

    pub fn new(credentials: Auth, base_url: UrlConstants) -> SessionManager {
        let idle_sessions: VecDeque<Session> = SessionManager::load();
        let valid_session_count: u8 = idle_sessions.len().try_into().unwrap();
        let sessions_created: u16 = idle_sessions
            .iter()
            .filter(|x| {
                let seconds_active = Utc::now().timestamp() - x.creation_timestamp;
                seconds_active < SECONDS_IN_A_DAY
            })
            .count()
            .try_into()
            .unwrap();
        SessionManager {
            idle_sessions: Mutex::new(idle_sessions),
            active_sessions: Mutex::new(Vec::new()),
            sessions_created: Mutex::new(sessions_created),
            valid_session_count: Mutex::new(valid_session_count),
            num_requests: Mutex::new(0),
            credentials,
            base_url,
        }
    }

    /*
     * Retrieves the first valid session, creating if possible and discarding any invalid sessions
     */
    pub fn get_session_key(&self) -> Option<String> {
        let mut active_sessions = self.active_sessions.lock().unwrap();
        let mut idle_sessions = self.idle_sessions.lock().unwrap();
        let mut valid_session_count = self.valid_session_count.lock().unwrap();
        let num_sessions: u16 = (*valid_session_count).try_into().unwrap();
        let mut sessions_created = self.sessions_created.lock().unwrap();
        let mut num_requests = self.num_requests.lock().unwrap();
        // check every session in idle_sessions and if valid, return
        //   if not valid, discard and look for the next one
        while let Some(session) = idle_sessions.pop_front() {
            match session.is_valid() {
                true => {
                    let key = session.session_key.clone();
                    active_sessions.push(session);
                    *num_requests += 1;
                    return Some(key);
                }
                false => {
                    *valid_session_count -= 1;
                }
            }
        }

        // TODO: Implement this pattern better
        match *sessions_created < LimitConstants::SessionsPerDay.val() {
            true => match num_sessions < LimitConstants::ConcurrentSessions.val() {
                true => match *num_requests < LimitConstants::RequestsPerDay.val() {
                    true => {
                        let new_session = self.create_session();
                        let key = new_session.session_key.clone();
                        active_sessions.push(new_session);
                        *valid_session_count += 1;
                        *sessions_created += 1;
                        *num_requests += 1;
                        Some(key)
                    }
                    false => panic!("Maximum number of requests per day reached"),
                },
                false => None,
            },
            false => panic!("Maximum number of sessions per day reached"),
        }
    }

    pub fn get_session_key_concurrent(&self) -> String {
        let mut wait_count = 0;
        let mut rng = thread_rng();
        loop {
            match self.get_session_key() {
                Some(key) => {
                    println!("Waited {} seconds for a session", wait_count);
                    return key;
                }
                // sleep for one second and between 0 and 5 nanoseconds
                None => {
                    wait_count += 1;
                    sleep(Duration::new(1, rng.gen_range(0, 5)));
                }
            };
        }
    }

    pub fn replace_session(&self, session_key: String) {
        let mut active_sessions = self.active_sessions.lock().unwrap();
        let mut idle_sessions = self.idle_sessions.lock().unwrap();
        let index = active_sessions
            .iter()
            .position(|x| x.session_key == session_key)
            .unwrap();
        idle_sessions.push_back(active_sessions[index].clone());
        active_sessions.remove(index);
    }

    pub fn remove_invalid_session(&self, session_key: String) {
        let mut active_sessions = self.active_sessions.lock().unwrap();
        let index = active_sessions
            .iter()
            .position(|x| x.session_key == session_key)
            .unwrap();
        active_sessions.remove(index);
        *self.valid_session_count.lock().unwrap() -= 1;
    }

    fn create_session(&self) -> Session {
        let url = url_builder::session_url(
            &self.base_url,
            &ReturnDataType::Json,
            &self.credentials.dev_id,
            &self.credentials.dev_key,
        );

        let response_text: String = request_maker::reqwest_to_text(url);

        let reply: CreateSessionReply = match serde_json::from_str(&response_text.clone()) {
            Ok(json) => json,
            Err(_) => panic!("Error deserializing create session reply"),
        };

        match reply.ret_msg {
            Some(msg) => {
                if msg != String::from("Approved") {
                    panic!(format!("CreateSession Request Error: {}", msg))
                }
            }
            None => panic!("CreateSession Request Error: ret_msg was null"),
        }

        let key = match reply.session_id {
            Some(key) => key,
            None => panic!("CreateSession Request Error: session_id was null"),
        };

        Session {
            session_key: key,
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
        let session_manager = SessionManager::new(auth, UrlConstants::UrlBase);

        assert!({ *session_manager.sessions_created.lock().unwrap() == 0 });
        assert!({ *session_manager.valid_session_count.lock().unwrap() == 0 });
        assert!({ session_manager.active_sessions.lock().unwrap().len() == 0 });
        assert!({ session_manager.idle_sessions.lock().unwrap().len() == 0 });

        let key = session_manager.get_session_key().unwrap();
        assert!(key != "");

        assert!({ *session_manager.sessions_created.lock().unwrap() == 1 });
        assert!({ *session_manager.valid_session_count.lock().unwrap() == 1 });
        assert!({ session_manager.active_sessions.lock().unwrap().len() == 1 });
        assert!({ session_manager.idle_sessions.lock().unwrap().len() == 0 });

        let first_key = key.clone();
        session_manager.replace_session(key);

        assert!({ *session_manager.sessions_created.lock().unwrap() == 1 });
        assert!({ *session_manager.valid_session_count.lock().unwrap() == 1 });
        assert!({ session_manager.active_sessions.lock().unwrap().len() == 0 });
        assert!({ session_manager.idle_sessions.lock().unwrap().len() == 1 });

        let first_key_again = session_manager.get_session_key().unwrap();
        assert_eq!(first_key, first_key_again);

        assert!({ *session_manager.sessions_created.lock().unwrap() == 1 });
        assert!({ *session_manager.valid_session_count.lock().unwrap() == 1 });
        assert!({ session_manager.active_sessions.lock().unwrap().len() == 1 });
        assert!({ session_manager.idle_sessions.lock().unwrap().len() == 0 });

        let second_key = session_manager.get_session_key().unwrap();
        assert!("" != second_key);
        assert!(first_key != second_key);

        assert!({ *session_manager.sessions_created.lock().unwrap() == 2 });
        assert!({ *session_manager.valid_session_count.lock().unwrap() == 2 });
        assert!({ session_manager.active_sessions.lock().unwrap().len() == 2 });
        assert!({ session_manager.idle_sessions.lock().unwrap().len() == 0 });

        session_manager.replace_session(second_key);

        assert!({ *session_manager.sessions_created.lock().unwrap() == 2 });
        assert!({ *session_manager.valid_session_count.lock().unwrap() == 2 });
        assert!({ session_manager.active_sessions.lock().unwrap().len() == 1 });
        assert!({ session_manager.idle_sessions.lock().unwrap().len() == 1 });
    }

    #[test]
    fn test_remove_invalid_session() {
        let auth = Auth::from_file("../hirez-dev-credentials.txt");
        let session_manager = SessionManager::new(auth, UrlConstants::UrlBase);

        assert!({ *session_manager.sessions_created.lock().unwrap() == 0 });
        assert!({ *session_manager.valid_session_count.lock().unwrap() == 0 });
        assert!({ session_manager.active_sessions.lock().unwrap().len() == 0 });
        assert!({ session_manager.idle_sessions.lock().unwrap().len() == 0 });

        let key = session_manager.get_session_key().unwrap();
        assert!(key != "");

        assert!({ *session_manager.sessions_created.lock().unwrap() == 1 });
        assert!({ *session_manager.valid_session_count.lock().unwrap() == 1 });
        assert!({ session_manager.active_sessions.lock().unwrap().len() == 1 });
        assert!({ session_manager.idle_sessions.lock().unwrap().len() == 0 });

        session_manager.remove_invalid_session(key);

        assert!({ *session_manager.sessions_created.lock().unwrap() == 1 });
        assert!({ *session_manager.valid_session_count.lock().unwrap() == 0 });
        assert!({ session_manager.active_sessions.lock().unwrap().len() == 0 });
        assert!({ session_manager.idle_sessions.lock().unwrap().len() == 0 });
    }
}
