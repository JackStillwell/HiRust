use std::fs::File;
use std::io::Write;

use crate::models::{PlayerMatchDetails, God, Item};
use crate::request_maker::{GetMatchIdsByQueueRequest, RequestMaker};
use crate::hi_rez_constants::UrlConstants;

cfg_if::cfg_if! {
    if #[cfg(test)] {
    } else {
        use crate::session_manager::{Auth, SessionManager};
    }
}

pub struct SmiteAPI {
    request_maker: RequestMaker,
}

impl SmiteAPI {
    #[cfg(not(test))]
    pub fn new(path_to_creds: String) -> SmiteAPI {
        let auth = Auth::from_file(&path_to_creds);
        let session_manager = SessionManager::new(auth, UrlConstants::UrlBase);
        SmiteAPI {
            request_maker: RequestMaker::new(session_manager),
        }
    }

    pub fn get_match_ids_by_queue(
        &mut self,
        requests: Vec<GetMatchIdsByQueueRequest>,
    ) -> Result<Vec<String>, String> {
        self.request_maker.get_match_ids_by_queue(requests)
    }

    pub fn get_match_details(
        &self,
        match_ids: Vec<String>,
    ) -> Result<Vec<Result<PlayerMatchDetails, String>>, String> {
        self.request_maker.get_match_details(match_ids)
    }

    pub fn get_gods(&self) -> Vec<God> {
        let gods_str = self.request_maker.concurrent_reqwest(UrlConstants::GetGods, vec![String::from("/1")]);
        let gods: Vec<God> = match serde_json::from_str(&gods_str[0]) {
            Ok(json) => json,
            Err(msg) => {
                let mut file = File::create("debug_dump.json").unwrap();
                file.write_all(gods_str[0].as_bytes()).unwrap();
                panic!(
                    "Error deserializing get gods: {}",
                    msg
                );
            }
        };
        gods
    }

    pub fn get_items(&self) -> Vec<Item> {
        let items_str = self.request_maker.concurrent_reqwest(UrlConstants::GetItems, vec![String::from("/1")]);
        let items: Vec<Item> = match serde_json::from_str(&items_str[0]) {
            Ok(json) => json,
            Err(msg) => {
                let mut file = File::create("debug_dump.json").unwrap();
                file.write_all(items_str[0].as_bytes()).unwrap();
                panic!(
                    "Error deserializing get items: {}",
                    msg
                );
            }
        };
        items
    }
}
