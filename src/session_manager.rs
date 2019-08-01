use chrono::{Utc, DateTime, Duration};

use crate::hi_rez_constants::LimitConstants;

pub struct Session {
    session_key: String,
    creation_timestamp: i64,
    // a bool indicating if the session
    //   is currently in use for an API call
    in_use: bool,
}

impl Session {
    pub fn is_valid(&self) -> bool {
        let seconds_active = Utc::now().timestamp() - self.creation_timestamp;
        return seconds_active < (LimitConstants::SessionTimeLimit.val() as i64);
    }

    pub fn is_in_use(&self) -> bool {
        return self.in_use;
    }

    pub fn r#use(&mut self) -> &Session {
        self.in_use = true;
        return self
    }
}

pub struct SessionManager {
    open_sessions: Vec<Session>,
}