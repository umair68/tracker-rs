use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ApiKeyQuery {
    pub api_key: String,
}

#[derive(Deserialize)]
pub struct TorrentAddQuery {
    pub api_key: String,
    pub id: u32,
    pub info_hash: String,
    #[serde(default)]
    pub freeleech: u8,
}

#[derive(Deserialize)]
pub struct TorrentRemoveQuery {
    pub api_key: String,
    pub info_hash: String,
}

#[derive(Deserialize)]
pub struct UserAddQuery {
    pub api_key: String,
    pub id: u32,
    pub passkey: String,
    pub class: u8,
}

#[derive(Deserialize)]
pub struct UserRemoveQuery {
    pub api_key: String,
    pub passkey: String,
}

#[derive(Serialize)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
}

#[derive(Deserialize)]
pub struct IpBanQuery {
    pub api_key: String,
    pub ip: String,
}

#[derive(Deserialize)]
pub struct ClientBanQuery {
    pub api_key: String,
    pub client: String,
}


#[derive(Serialize)]
pub struct IpListResponse {
    pub success: bool,
    pub ipv4: Vec<String>,
    pub ipv6: Vec<String>,
}

#[derive(Serialize)]
pub struct ClientListResponse {
    pub success: bool,
    pub clients: Vec<String>,
}
