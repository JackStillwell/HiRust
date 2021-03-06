use chrono::{Datelike, Timelike, Utc};
use crypto::{digest::Digest, md5::Md5};

use crate::hi_rez_constants::{ReturnDataType, UrlConstants};

fn build_signature(id: &str, method_name: &UrlConstants, key: &str, date: &str) -> String {
    let mut md5 = Md5::new();
    md5.input_str(&format!("{}{}{}{}", id, method_name.val(), key, date));
    return md5.result_str();
}

fn get_timestamp() -> String {
    let systemtime = Utc::now();
    let timestamp: String = format!(
        "{}{:02}{:02}{:02}{:02}{:02}",
        systemtime.year(),
        systemtime.month(),
        systemtime.day(),
        systemtime.hour(),
        systemtime.minute(),
        systemtime.second(),
    );

    return timestamp;
}

pub fn ping_url(base_url: &UrlConstants, data_type: &ReturnDataType) -> String {
    return format!("{}/{}{}", base_url.val(), "ping", data_type.val(),);
}

pub fn session_url(
    base_url: &UrlConstants,
    data_type: &ReturnDataType,
    id: &str,
    key: &str,
) -> String {
    let method_name = UrlConstants::CreateSession;
    let timestamp: String = get_timestamp();
    let signature: String = build_signature(id, &method_name, key, &timestamp);
    return format!(
        "{}/{}{}/{}/{}/{}",
        base_url.val(),
        method_name.val(),
        data_type.val(),
        id,
        signature,
        timestamp,
    );
}

pub fn url(
    id: &String,
    key: &String,
    session: &String,
    base_url: &UrlConstants,
    method_name: &UrlConstants,
    data_type: &ReturnDataType,
    method_specific: &String,
) -> String {
    let timestamp: String = get_timestamp();
    let signature: String = build_signature(id, method_name, key, &timestamp);
    return format!(
        "{}/{}{}/{}/{}/{}/{}{}",
        base_url.val(),
        method_name.val(),
        data_type.val(),
        id,
        signature,
        session,
        timestamp,
        method_specific,
    );
}
