
pub enum UrlConstants {
    UrlBase,
    DataLimitCheck,
    CreateSession,
    GetMatchDetails,
    GetMatchDetailsBatch,
    GetMatchIdsByQueue,
    GetGods,
    GetItems,
}

impl UrlConstants {
    pub fn val(&self) -> String {
        match *self {
            UrlConstants::UrlBase => String::from("http://api.smitegame.com/smiteapi.svc"),
            UrlConstants::DataLimitCheck => String::from("getdataused"),
            UrlConstants::CreateSession => String::from("createsession"),
            UrlConstants::GetMatchDetails => String::from("getmatchdetails"),
            UrlConstants::GetMatchDetailsBatch => String::from("getmatchdetailsbatch"),
            UrlConstants::GetMatchIdsByQueue => String::from("getmatchdetailsbatch"),
            UrlConstants::GetGods => String::from("getgods"),
            UrlConstants::GetItems => String::from("getitems"),
        }
    }
}

pub enum ReturnDataType {
    Json,
    Xml,
}

impl ReturnDataType {
    pub fn val(&self) -> String {
        match *self {
            ReturnDataType::Json => String::from("json"),
            ReturnDataType::Xml => String::from("xml"),
        }
    }
}

pub enum DataConstants {
    RankedConquest,
}

impl DataConstants {
    pub fn val(&self) -> String {
        match *self {
            DataConstants::RankedConquest => String::from("451"),
        }
    }
}

pub enum LimitConstants {
    ConcurrentSessions,
    SessionsPerDay,
    // the time limit in seconds
    SessionTimeLimit,
    RequestsPerDay,
}

impl LimitConstants {
    pub fn val(&self) -> u16 {
        match *self {
            LimitConstants::ConcurrentSessions => 50,
            LimitConstants::SessionsPerDay => 500,
            LimitConstants::SessionTimeLimit => 900,
            LimitConstants::RequestsPerDay => 7500,
        }
    }
}
