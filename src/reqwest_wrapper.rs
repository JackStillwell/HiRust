cfg_if::cfg_if! {
    if #[cfg(test)] {
        use mockall::*;
        use MockReqwest as reqwest;
        use MockReqwestResponse as Response;
        use galvanic_test::test_suite;

        #[automock]
        trait Reqwest {
            fn get(url: &str) -> Result<Response, String>;
        }

        #[automock]
        trait ReqwestResponse {
            fn text(&mut self) -> Result<String, String>;
        }

        #[automock]
        pub trait Wrapper {
            fn get_to_text(&self, url: String) -> Result<String, String>;
        }
    } else {
        use reqwest;
    }
}

pub struct ReqwestWrapper {}

impl ReqwestWrapper {
    pub fn new() -> ReqwestWrapper {
        ReqwestWrapper {}
    }

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
            let str_len = error_string.len();
            let substring = &error_string[1..(str_len - 2)];
            return Err(String::from(substring));
        }
    }
}

#[cfg(test)]
test_suite! {
    name test_reqwest_wrapper;
    use super::*;

    test three_tries_fail_reqwest() {
        reqwest::expect_get().returning(|_x| Err(String::from("Failure")));
        let reqwest_wrapper = ReqwestWrapper::new();
        let results = reqwest_wrapper.get_to_text(String::from("bad_url"));
        let failure_string = "Error reqwesting url: Failure | Error reqwesting url: Failure | Error reqwesting url: Failure";
        assert_eq!(results, Err(String::from(failure_string)));
    }

    test three_tries_fail_response_text() {
        reqwest::expect_get().returning(|_x| Ok(Response::new()));
        let reqwest_wrapper = ReqwestWrapper::new();
        let results = reqwest_wrapper.get_to_text(String::from("bad_url"));
        let failure_string = "Error decoding response: Failure | Error decoding response: Failure | Error decoding response: Failure";
        assert_eq!(results, Err(String::from(failure_string)));
    }
}
