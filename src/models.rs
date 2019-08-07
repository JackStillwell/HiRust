use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CreateSessionReply {
    pub ret_msg: String,
    pub session_id: String,
    pub timestamp: String,
}

#[derive(Deserialize)]
pub struct GetMatchIdsByQueueReply {
    pub ret_msg: String,
}
