use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct WsMessage {
    #[serde(rename = "type")]
    pub msg_type: String,
    pub data: serde_json::Value,
}

impl WsMessage {
    pub fn new<T: Serialize>(msg_type: &str, data: &T) -> Self {
        Self {
            msg_type: msg_type.to_string(),
            data: serde_json::to_value(data).unwrap_or_default(),
        }
    }
}
