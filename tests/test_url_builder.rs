use reqwest;

use hirust::{hi_rez_constants, url_builder};

#[test]
fn test_ping_url() {
    let url: String = url_builder::ping_url(
        &hi_rez_constants::UrlConstants::UrlBase,
        &hi_rez_constants::ReturnDataType::Json,
    );
    let response = reqwest::get(&url);
    let response_text = response.unwrap().text().unwrap();
    assert_eq!(response_text.contains("SmiteAPI"), true);
}
