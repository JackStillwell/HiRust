use chrono::{Datelike, Duration, Timelike, Utc};
use crypto::{digest::Digest, md5::Md5};

use crate::hi_rez_constants::{
    UrlConstants,
    ReturnDataType,
    DataConstants
};

fn build_signature(id: &String, method_name: &UrlConstants, key: &String, date: &String) -> String {
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

fn build_url(
    id: &String,
    key: &String,
    session: &String,
    method_name: &UrlConstants,
    data_type: &ReturnDataType,
    method_specific: &String,
) -> String {
    let timestamp: String = get_timestamp();
    let signature: String = build_signature(
        id,
        method_name,
        key,
        &timestamp,
    );
    return format!("{}/{}{}/{}/{}/{}/{}/{}",
        UrlConstants::UrlBase.val(),
        method_name.val(),
        data_type.val(),
        id,
        signature,
        session,
        timestamp,
        method_specific,
    );
}