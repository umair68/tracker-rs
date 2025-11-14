// Centralized error handling for the tracker

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

/// Errors that can occur during announce processing
#[derive(Error, Debug)]
pub enum AnnounceError {
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Missing required parameter: {0}")]
    MissingParameter(String),

    #[error("This is a BitTorrent tracker announce URL, not meant to be opened in a web browser. Please add this URL to your torrent client (like qBittorrent, Transmission, etc.) as the tracker for your torrent. Your torrent client will automatically send the required parameters."
    )]
    BrowserAccess,

    #[error("Invalid passkey provided")]
    InvalidPasskey,

    #[error("User account is disabled")]
    UserDisabled,

    #[error("Torrent not registered")]
    TorrentNotFound,

    #[error("Torrent is not active")]
    TorrentInactive,

    #[error("IP address is banned")]
    IpBanned,

    #[error("Client is banned")]
    ClientBanned,

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Too many IPs for this torrent")]
    DuplicatePeer,

    #[error("Announce interval too short")]
    AnnounceIntervalTooShort,

    #[error("Suspicious client detected")]
    SuspiciousClient,

    #[error("Internal server error")]
    InternalError(#[from] anyhow::Error),
}

impl IntoResponse for AnnounceError {
    fn into_response(self) -> Response {
        // Special case: BrowserAccess returns plain text for users
        if matches!(self, AnnounceError::BrowserAccess) {
            let message = "Nothing to see here".to_string();

            return Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/plain; charset=utf-8")
                .body(message.into())
                .unwrap();
        }

        // For all other errors, return bencode response
        use crate::bencode::encoder::BencodeEncode;
        
        let message = self.to_string();

        // Build bencode error response: d14:failure reason<len>:<message>e
        let mut buf = Vec::with_capacity(128);

        buf.extend_from_slice(b"d");

        "failure reason".bencode(&mut buf);
        message.as_str().bencode(&mut buf);

        buf.extend_from_slice(b"e");

        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/plain")
            .body(buf.into())
            .unwrap()
    }
}

#[derive(Error, Debug)]
pub enum AdminError {
    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Failed to parse hex: {0}")]
    HexDecodeError(String),

    #[error("Invalid length: expected {expected}, got {actual}")]
    InvalidLength { expected: usize, actual: usize },

    #[error("Failed to create API client: {0}")]
    ApiClientError(String),

    #[error("Failed to fetch data from external API: {0}")]
    ExternalApiError(String),

    #[error("Failed to write to WAL: {0}")]
    WalError(String),

    #[error("Internal server error: {0}")]
    InternalError(String),
}

impl IntoResponse for AdminError {
    fn into_response(self) -> Response {
        use crate::models::admin::ErrorResponse;
        use axum::response::Json;

        let (status, error_message) = match &self {
            AdminError::InvalidApiKey => (StatusCode::UNAUTHORIZED, self.to_string()),
            AdminError::InvalidParameter(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AdminError::NotFound(_) => (StatusCode::NOT_FOUND, self.to_string()),
            AdminError::HexDecodeError(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AdminError::InvalidLength { .. } => (StatusCode::BAD_REQUEST, self.to_string()),
            AdminError::ApiClientError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            AdminError::ExternalApiError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            AdminError::WalError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            AdminError::InternalError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        (
            status,
            Json(ErrorResponse {
                success: false,
                error: error_message,
            }),
        )
            .into_response()
    }
}

#[derive(Error, Debug)]
pub enum BlacklistError {
    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Invalid IP address: {0}")]
    InvalidIpAddress(String),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Internal server error: {0}")]
    InternalError(String),
}

impl IntoResponse for BlacklistError {
    fn into_response(self) -> Response {
        use crate::models::admin::ErrorResponse;
        use axum::response::Json;

        let (status, error_message) = match &self {
            BlacklistError::InvalidApiKey => (StatusCode::UNAUTHORIZED, self.to_string()),
            BlacklistError::InvalidIpAddress(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            BlacklistError::InvalidParameter(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            BlacklistError::InternalError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };

        (
            status,
            Json(ErrorResponse {
                success: false,
                error: error_message,
            }),
        )
            .into_response()
    }
}


#[derive(Error, Debug)]
pub enum MonitoringError {
    #[error("Invalid API key")]
    InvalidApiKey,

    #[error("Internal server error: {0}")]
    InternalError(String),
}

impl IntoResponse for MonitoringError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            MonitoringError::InvalidApiKey => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            MonitoringError::InternalError(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error"),
        };

        (status, message).into_response()
    }
}

#[derive(Error, Debug)]
pub enum AntiCheatError {
    #[error("Announce interval too short: {elapsed}s < {min_interval}s")]
    AnnounceIntervalTooShort { elapsed: i64, min_interval: i64 },

    #[error("Too many IPs for this torrent: {count} > {max}")]
    TooManyIps { count: usize, max: u32 },

    #[error("Suspicious upload speed: {speed_mbps:.2} MB/s > {max_mbps:.2} MB/s")]
    SuspiciousUploadSpeed { speed_mbps: f64, max_mbps: f64 },

    #[error("Suspicious download speed: {speed_mbps:.2} MB/s > {max_mbps:.2} MB/s")]
    SuspiciousDownloadSpeed { speed_mbps: f64, max_mbps: f64 },

    #[error("Suspicious ratio: {ratio:.2} > {max_ratio:.2}")]
    SuspiciousRatio { ratio: f64, max_ratio: f64 },

    #[error("Ghost seeder detected: uploaded {uploaded} bytes < {min_upload} bytes")]
    GhostSeeder { uploaded: u64, min_upload: u64 },
}

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Missing required parameter: {0}")]
    MissingParameter(String),

    #[error("Invalid parameter format: {0}")]
    InvalidFormat(String),

    #[error("Parameter out of range: {0}")]
    OutOfRange(String),

    #[error("Invalid hex encoding: {0}")]
    InvalidHex(String),

    #[error("Invalid length: expected {expected}, got {actual}")]
    InvalidLength { expected: usize, actual: usize },
}
