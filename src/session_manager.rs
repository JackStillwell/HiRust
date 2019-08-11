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
use crate::url_builder;

cfg_if::cfg_if! { 
    if #[cfg(test)] {
        use crate::reqwest_wrapper::MockReqwestWrapper as ReqwestWrapper;
        use galvanic_test::test_suite;
    } else {
        use crate::reqwest_wrapper::ReqwestWrapper;
    }
}

#[cfg(not(test))]
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

pub struct SessionManager {
    idle_sessions: Mutex<VecDeque<Session>>,
    active_sessions: Mutex<Vec<Session>>,
    sessions_created: Mutex<u16>,
    valid_session_count: Mutex<u8>,
    num_requests: Mutex<u16>,
    reqwest: ReqwestWrapper,
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

    #[cfg(not(test))]
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

    #[cfg(not(test))]
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
            reqwest: ReqwestWrapper {},
            credentials,
            base_url,
        }
    }

    #[cfg(test)]
    pub fn mock(reqwest: ReqwestWrapper) -> SessionManager {
        SessionManager {
            idle_sessions: Mutex::new(VecDeque::new()),
            active_sessions: Mutex::new(Vec::new()),
            sessions_created: Mutex::new(0),
            valid_session_count: Mutex::new(0),
            num_requests: Mutex::new(0),
            reqwest,
            credentials: Auth {
                dev_id: String::from("dummy"),
                dev_key: String::from("creds"),
            },
            base_url: UrlConstants::UrlBase,
        }
    }

    /*
     * Retrieves the first valid session, creating if necessary
     */
    pub fn get_session_key(&self) -> Result<String, String> {
        let mut active_sessions = self.active_sessions.lock().unwrap();
        let mut idle_sessions = self.idle_sessions.lock().unwrap();
        let mut valid_session_count = self.valid_session_count.lock().unwrap();
        let num_sessions: u16 = (*valid_session_count).try_into().unwrap();
        let mut sessions_created = self.sessions_created.lock().unwrap();
        let mut num_requests = self.num_requests.lock().unwrap();

        match idle_sessions.pop_front() {
            Some(session) => {
                let key = session.session_key.clone();
                active_sessions.push(session);
                *num_requests += 1;
                return Ok(key);
            }
            None => {}
        }

        if *sessions_created >= LimitConstants::SessionsPerDay.val() {
            return Err(String::from("Maximum number of sessions per day reached"));
        } else if *num_requests >= LimitConstants::RequestsPerDay.val() {
            return Err(String::from("Maximum number of requests per day reached"));
        } else if num_sessions >= LimitConstants::ConcurrentSessions.val() {
            return Err(String::from("No sessions available"));
        } else {
            let new_session = self.create_session();
            match new_session {
                Ok(new_session) => {
                    let key = new_session.session_key.clone();
                    active_sessions.push(new_session);
                    *valid_session_count += 1;
                    *sessions_created += 1;
                    *num_requests += 1;
                    return Ok(key);
                }
                Err(msg) => return Err(msg),
            }
        }
    }

    pub fn get_session_key_concurrent(&self) -> Result<String, String> {
        let mut rng = thread_rng();
        loop {
            match self.get_session_key() {
                Ok(key) => return Ok(key),
                // sleep for one second and between 0 and 5 nanoseconds
                Err(msg) => {
                    if msg == String::from("No sessions available") {
                        sleep(Duration::new(1, rng.gen_range(0, 5)));
                    } else {
                        return Err(msg);
                    }
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

    fn create_session(&self) -> Result<Session, String> {
        let url = url_builder::session_url(
            &self.base_url,
            &ReturnDataType::Json,
            &self.credentials.dev_id,
            &self.credentials.dev_key,
        );

        let response_text: String = match self.reqwest.get_to_text(url) {
            Ok(text) => text,
            Err(msg) => return Err(msg),
        };

        let reply: CreateSessionReply = match serde_json::from_str(&response_text.clone()) {
            Ok(json) => json,
            Err(msg) => return Err(format!("Error deserializing create session reply: {}", msg)),
        };

        match reply.ret_msg {
            Some(msg) => {
                if msg != String::from("Approved") {
                    return Err(format!("CreateSession Request Error: {}", msg));
                }
            }
            None => {
                return Err(String::from(
                    "CreateSession Request Error: ret_msg was null",
                ))
            }
        }

        let key = match reply.session_id {
            Some(key) => key,
            None => {
                return Err(String::from(
                    "CreateSession Request Error: session_id was null",
                ))
            }
        };

        Ok(Session {
            session_key: key,
            creation_timestamp: Utc::now().timestamp(),
        })
    }
}

#[cfg(test)]
test_suite! {
    name test_session_manager;
    use super::*;
    use rand::{thread_rng, Rng};

    fixture create_sm() -> SessionManager {
        setup(&mut self) {
            // NOTE: this is a MockReqwestWrapper
            let mut reqwest = ReqwestWrapper::new();
            reqwest.expect_get_to_text().returning({
                |_x| {
                    let mut randgen = thread_rng();
                    let mut session_id_array = [0u8; 10];
                    randgen.fill(&mut session_id_array);
                    let mut session_id = String::new();
                    for num in session_id_array.iter() {
                        session_id.push_str(&num.to_string());
                    }
                    Ok(format!(
                        "{{ \"ret_msg\": \"Approved\", \"session_id\": \"{}\", \"timestamp\": null }}",
                        session_id
                    ))
                }
            });
            SessionManager::mock(reqwest)
        }
    }

    test get_replace_session(create_sm) {
        let session_manager = create_sm.val;
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

    test remove_invalid_session(create_sm) {
        let session_manager = create_sm.val;

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