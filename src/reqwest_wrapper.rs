use ::reqwest::{Error, Response};
use mockall::*;

#[automock]
trait Reqwest {
    fn get(url: &str) -> Result<Response, Error>;
}

cfg_if::cfg_if! {
    if #[cfg(test)] {
        use MockReqwest as reqwest;
    } else {
        use reqwest;
    }
}

pub struct ReqwestWrapper {}

#[automock]
impl ReqwestWrapper {
    pub fn get_to_text(&self, url: String) -> Result<String, String> {
        let mut error_messages: Vec<String> = Vec::new();
        let mut ret_text: String = String::new();
        for _ in 0..3 {
            let response_result = reqwest::get(&url);

            let mut response = match response_result {
                Ok(response) => response,
                Err(msg) => {
                    error_messages.push(format!("Error reqwesting url: {}", msg));
                    continue;
                }
            };

            match response.text() {
                Ok(text) => {
                    ret_text = text;
                    break;
                }
                Err(msg) => error_messages.push(format!("Error decoding response: {}", msg)),
            }
        }

        if error_messages.len() < 3 {
            return Ok(ret_text);
        } else {
            let mut error_string: String = String::new();
            for error in error_messages {
                let msg: String = format!(" {} |", error);
                error_string.push_str(&msg);
            }
            return Err(error_string);
        }
    }
}
