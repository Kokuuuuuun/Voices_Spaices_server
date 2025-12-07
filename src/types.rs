use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub color: String,
    pub x: f64,
    pub y: f64,
    #[serde(rename = "roomId")]
    pub room_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomObject {
    pub id: String,
    #[serde(rename = "type")]
    pub obj_type: String, // "image", "note", "gif"
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub content: String, // URL or text content
    #[serde(default, rename = "zIndex")]
    pub z_index: i32,
    #[serde(default)]
    pub rotation: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Room {
    pub id: String,
    pub name: String,
    pub users: Vec<User>,
    #[serde(default)]
    pub objects: Vec<RoomObject>,
    #[serde(default)]
    pub background: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "userName")]
    pub user_name: String,
    pub text: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DrawData {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub color: String,
    pub width: f64,
}

// WebRTC Signaling Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalPayload {
    #[serde(rename = "userToSignal")]
    pub user_to_signal: Option<String>,
    #[serde(rename = "callerID")]
    pub caller_id: String,
    pub signal: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnSignalPayload {
    pub signal: serde_json::Value,
    pub id: String,
}
