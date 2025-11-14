use crate::api::client::ApiClient;
use crate::core::error::AdminError;
use crate::models::admin::{
    ApiKeyQuery, SuccessResponse, TorrentAddQuery, TorrentRemoveQuery,
    UserAddQuery, UserRemoveQuery,
};
use crate::models::torrent::Torrent;
use crate::models::user::User;
use crate::core::startup::populate_from_api;
use crate::core::state::AppState;
use crate::utils::auth::verify_api_key;
use crate::wal::wal::WalOperation;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use std::sync::Arc;
use tracing::{info, warn};

/// Add a torrent to the cache
///
/// GET /torrent/add?api_key=<key>&id=<id>&info_hash=<hash>&freeleech=<0|1>
pub async fn torrent_add_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<TorrentAddQuery>,
) -> Result<Response, AdminError> {
    // Verify API key
    if !verify_api_key(&params.api_key, &state.config.sync.api_key) {
        warn!("Unauthorized torrent add attempt");
        return Err(AdminError::InvalidApiKey);
    }

    // Decode info_hash from hex
    let info_hash_bytes = hex::decode(&params.info_hash)
        .map_err(|e| AdminError::HexDecodeError(e.to_string()))?;

    if info_hash_bytes.len() != 20 {
        warn!("info_hash must be 20 bytes");
        return Err(AdminError::InvalidLength {
            expected: 20,
            actual: info_hash_bytes.len(),
        });
    }

    let mut info_hash = [0u8; 20];
    info_hash.copy_from_slice(&info_hash_bytes);

    let freeleech = params.freeleech != 0;

    // Create torrent
    let torrent = Torrent::new(params.id, info_hash, freeleech, true);

    // Add to cache
    state.torrent_cache.add_torrent(torrent);

    // Log to WAL
    if let Err(e) = state.wal.log_operation(WalOperation::AddTorrent {
        id: params.id,
        info_hash,
        freeleech,
    }) {
        warn!(error = %e, "Failed to log torrent add to WAL");
        // Continue anyway - cache is updated
    }

    info!(
        torrent_id = params.id,
        info_hash = %params.info_hash,
        freeleech = freeleech,
        "Torrent added"
    );

    Ok((
        StatusCode::OK,
        Json(SuccessResponse {
            success: true,
            message: "Torrent added successfully".to_string(),
        }),
    )
        .into_response())
}

/// Remove a torrent from the cache
///
/// GET /torrent/remove?api_key=<key>&info_hash=<hash>
pub async fn torrent_remove_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<TorrentRemoveQuery>,
) -> Result<Response, AdminError> {
    // Verify API key
    if !verify_api_key(&params.api_key, &state.config.sync.api_key) {
        warn!("Unauthorized torrent remove attempt");
        return Err(AdminError::InvalidApiKey);
    }

    // Decode info_hash from hex
    let info_hash_bytes = hex::decode(&params.info_hash)
        .map_err(|e| AdminError::HexDecodeError(e.to_string()))?;

    if info_hash_bytes.len() != 20 {
        warn!("info_hash must be 20 bytes");
        return Err(AdminError::InvalidLength {
            expected: 20,
            actual: info_hash_bytes.len(),
        });
    }

    let mut info_hash = [0u8; 20];
    info_hash.copy_from_slice(&info_hash_bytes);

    // Check if torrent exists
    if state.torrent_cache.get_torrent(info_hash).is_none() {
        warn!(info_hash = %params.info_hash, "Torrent not found");
        return Err(AdminError::NotFound("Torrent not found".to_string()));
    }

    // Remove from cache
    state.torrent_cache.remove_torrent(info_hash);

    // Log to WAL
    if let Err(e) = state.wal.log_operation(WalOperation::RemoveTorrent { info_hash }) {
        warn!(error = %e, "Failed to log torrent remove to WAL");
        // Continue anyway - cache is updated
    }

    info!(info_hash = %params.info_hash, "Torrent removed");

    Ok((
        StatusCode::OK,
        Json(SuccessResponse {
            success: true,
            message: "Torrent removed successfully".to_string(),
        }),
    )
        .into_response())
}

/// Add a user to the cache
///
/// GET /user/add?api_key=<key>&id=<id>&passkey=<passkey>&class=<class>
pub async fn user_add_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<UserAddQuery>,
) -> Result<Response, AdminError> {
    // Verify API key
    if !verify_api_key(&params.api_key, &state.config.sync.api_key) {
        warn!("Unauthorized user add attempt");
        return Err(AdminError::InvalidApiKey);
    }

    // Decode passkey from hex
    let passkey_bytes = hex::decode(&params.passkey)
        .map_err(|e| AdminError::HexDecodeError(e.to_string()))?;

    if passkey_bytes.len() != 32 {
        warn!("passkey must be 32 bytes");
        return Err(AdminError::InvalidLength {
            expected: 32,
            actual: passkey_bytes.len(),
        });
    }

    let mut passkey = [0u8; 32];
    passkey.copy_from_slice(&passkey_bytes);

    // Create user (active by default)
    let user = User::new(params.id, passkey, params.class, true);

    // Add to cache
    state.user_cache.add_user(user);

    // Log to WAL
    if let Err(e) = state.wal.log_operation(WalOperation::AddUser {
        id: params.id,
        passkey,
        class: params.class,
    }) {
        warn!(error = %e, "Failed to log user add to WAL");
        // Continue anyway - cache is updated
    }

    info!(
        user_id = params.id,
        passkey = %params.passkey,
        class = params.class,
        "User added"
    );

    Ok((
        StatusCode::OK,
        Json(SuccessResponse {
            success: true,
            message: "User added successfully".to_string(),
        }),
    )
        .into_response())
}

/// Remove a user from the cache
/// 
/// GET /user/remove?api_key=<key>&passkey=<passkey>
pub async fn user_remove_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<UserRemoveQuery>,
) -> Result<Response, AdminError> {
    // Verify API key
    if !verify_api_key(&params.api_key, &state.config.sync.api_key) {
        warn!("Unauthorized user remove attempt");
        return Err(AdminError::InvalidApiKey);
    }

    // Decode passkey from hex
    let passkey_bytes = hex::decode(&params.passkey)
        .map_err(|e| AdminError::HexDecodeError(e.to_string()))?;

    if passkey_bytes.len() != 32 {
        warn!("passkey must be 32 bytes");
        return Err(AdminError::InvalidLength {
            expected: 32,
            actual: passkey_bytes.len(),
        });
    }

    let mut passkey = [0u8; 32];
    passkey.copy_from_slice(&passkey_bytes);

    // Check if user exists
    if state.user_cache.get_user(passkey).is_none() {
        warn!(passkey = %params.passkey, "User not found");
        return Err(AdminError::NotFound("User not found".to_string()));
    }

    // Remove from cache
    state.user_cache.remove_user(passkey);

    // Log to WAL
    if let Err(e) = state.wal.log_operation(WalOperation::RemoveUser { passkey }) {
        warn!(error = %e, "Failed to log user remove to WAL");
        // Continue anyway - cache is updated
    }

    info!(passkey = %params.passkey, "User removed");

    Ok((
        StatusCode::OK,
        Json(SuccessResponse {
            success: true,
            message: "User removed successfully".to_string(),
        }),
    )
        .into_response())
}

/// Reload user and torrent data from external API
/// 
/// POST /reload?api_key=<key>
pub async fn reload_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ApiKeyQuery>,
) -> Result<Response, AdminError> {
    // Verify API key
    if !verify_api_key(&params.api_key, &state.config.sync.api_key) {
        warn!("Unauthorized reload attempt");
        return Err(AdminError::InvalidApiKey);
    }

    info!("Starting cache reload from external API");

    // Clear existing caches
    state.user_cache.clear();
    state.torrent_cache.clear();

    info!("Caches cleared");

    // Create API client
    let api_client = ApiClient::new(
        state.config.sync.data_endpoint.clone(),
        state.config.sync.api_key.clone(),
    )
    .map_err(|e| AdminError::ApiClientError(e.to_string()))?;

    // Fetch fresh data from external API and populate caches
    populate_from_api(&state, &api_client)
        .await
        .map_err(|e| AdminError::ExternalApiError(e.to_string()))?;

    // Truncate WAL
    if let Err(e) = state.wal.truncate() {
        warn!(error = %e, "Failed to truncate WAL");
        // Continue anyway - caches are updated
    }

    info!(
        users = state.user_cache.len(),
        torrents = state.torrent_cache.len(),
        "Cache reload completed successfully"
    );

    Ok((
        StatusCode::OK,
        Json(SuccessResponse {
            success: true,
            message: format!(
                "Reload successful: {} users, {} torrents",
                state.user_cache.len(),
                state.torrent_cache.len()
            ),
        }),
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::{
        AntiCheatConfig, Config, LoggingConfig, MemoryConfig, PerformanceConfig, SecurityConfig,
        ServerConfig, SyncConfig,
    };
    use crate::wal::wal::Wal;
    use tempfile::TempDir;

    fn create_test_config() -> Config {
        Config {
            server: ServerConfig {
                port: Some(8080),
                unix_socket: None,
                num_threads: 4,
                max_connections: 1000,
            },
            memory: MemoryConfig {
                peer_capacity: 10000,
                torrent_cache_size: 1000,
                user_cache_size: 1000,
            },
            performance: PerformanceConfig {
                min_announce_interval: 1800,
                max_requests_per_minute: 60,
                cleanup_interval: 300,
                peer_timeout: 3600,
            },
            sync: SyncConfig {
                data_endpoint: "http://localhost:8000/api".to_string(),
                api_key: "test-api-key".to_string(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                path: None,
                console: true,
            },
            anti_cheat: AntiCheatConfig {
                max_ips_per_user: 3,
                max_ratio: 10.0,
                max_upload_speed: 100.0,
                max_download_speed: 100.0,
                min_seeder_upload: 1024,
            },
            security: SecurityConfig {
                banned_ips: vec![],
                banned_clients: vec![],
            },
        }
    }

    fn create_test_state() -> Arc<AppState> {
        let temp_dir = TempDir::new().unwrap();
        let wal_path = temp_dir.path().join("test.wal");
        let wal = Wal::new(wal_path).unwrap();
        let config = create_test_config();
        
        Arc::new(AppState::new(config, wal))
    }

    #[tokio::test]
    async fn test_torrent_add_success() {
        let state = create_test_state();
        let info_hash = "0101010101010101010101010101010101010101"; // 40 hex chars = 20 bytes
        
        let params = TorrentAddQuery {
            api_key: "test-api-key".to_string(),
            id: 123,
            info_hash: info_hash.to_string(),
            freeleech: 1,
        };

        let response = torrent_add_handler(State(state.clone()), Query(params)).await.unwrap();
        
        // Check response status
        assert_eq!(response.status(), StatusCode::OK);
        
        // Verify torrent was added to cache
        let info_hash_bytes = hex::decode(info_hash).unwrap();
        let mut hash = [0u8; 20];
        hash.copy_from_slice(&info_hash_bytes);
        
        let torrent = state.torrent_cache.get_torrent(hash);
        assert!(torrent.is_some());
        let torrent = torrent.unwrap();
        assert_eq!(torrent.id, 123);
        assert_eq!(torrent.is_freeleech, true);
    }

    #[tokio::test]
    async fn test_torrent_add_invalid_api_key() {
        let state = create_test_state();
        
        let params = TorrentAddQuery {
            api_key: "wrong-key".to_string(),
            id: 123,
            info_hash: "0101010101010101010101010101010101010101".to_string(),
            freeleech: 0,
        };

        let result = torrent_add_handler(State(state), Query(params)).await;
        assert!(result.is_err());
        let response = result.unwrap_err().into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_torrent_add_invalid_hash() {
        let state = create_test_state();
        
        let params = TorrentAddQuery {
            api_key: "test-api-key".to_string(),
            id: 123,
            info_hash: "invalid-hex".to_string(),
            freeleech: 0,
        };

        let result = torrent_add_handler(State(state), Query(params)).await;
        assert!(result.is_err());
        let response = result.unwrap_err().into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_torrent_remove_success() {
        let state = create_test_state();
        let info_hash = "0202020202020202020202020202020202020202";
        
        // First add a torrent
        let info_hash_bytes = hex::decode(info_hash).unwrap();
        let mut hash = [0u8; 20];
        hash.copy_from_slice(&info_hash_bytes);
        
        let torrent = Torrent::new(456, hash, false, true);
        state.torrent_cache.add_torrent(torrent);
        
        // Now remove it
        let params = TorrentRemoveQuery {
            api_key: "test-api-key".to_string(),
            info_hash: info_hash.to_string(),
        };

        let response = torrent_remove_handler(State(state.clone()), Query(params)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        
        // Verify it was removed
        assert!(state.torrent_cache.get_torrent(hash).is_none());
    }

    #[tokio::test]
    async fn test_torrent_remove_not_found() {
        let state = create_test_state();
        
        let params = TorrentRemoveQuery {
            api_key: "test-api-key".to_string(),
            info_hash: "0303030303030303030303030303030303030303".to_string(),
        };

        let result = torrent_remove_handler(State(state), Query(params)).await;
        assert!(result.is_err());
        let response = result.unwrap_err().into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_user_add_success() {
        let state = create_test_state();
        let passkey = "0404040404040404040404040404040404040404040404040404040404040404"; // 64 hex chars = 32 bytes
        
        let params = UserAddQuery {
            api_key: "test-api-key".to_string(),
            id: 789,
            passkey: passkey.to_string(),
            class: 2,
        };

        let response = user_add_handler(State(state.clone()), Query(params)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        
        // Verify user was added to cache
        let passkey_bytes = hex::decode(passkey).unwrap();
        let mut key = [0u8; 32];
        key.copy_from_slice(&passkey_bytes);
        
        let user = state.user_cache.get_user(key);
        assert!(user.is_some());
        let user = user.unwrap();
        assert_eq!(user.id, 789);
        assert_eq!(user.class, 2);
        assert_eq!(user.is_active, true);
    }

    #[tokio::test]
    async fn test_user_add_invalid_passkey() {
        let state = create_test_state();
        
        let params = UserAddQuery {
            api_key: "test-api-key".to_string(),
            id: 789,
            passkey: "too-short".to_string(),
            class: 1,
        };

        let result = user_add_handler(State(state), Query(params)).await;
        assert!(result.is_err());
        let response = result.unwrap_err().into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_user_remove_success() {
        let state = create_test_state();
        let passkey = "0505050505050505050505050505050505050505050505050505050505050505";
        
        // First add a user
        let passkey_bytes = hex::decode(passkey).unwrap();
        let mut key = [0u8; 32];
        key.copy_from_slice(&passkey_bytes);
        
        let user = User::new(999, key, 1, true);
        state.user_cache.add_user(user);
        
        // Now remove it
        let params = UserRemoveQuery {
            api_key: "test-api-key".to_string(),
            passkey: passkey.to_string(),
        };

        let response = user_remove_handler(State(state.clone()), Query(params)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        
        // Verify it was removed
        assert!(state.user_cache.get_user(key).is_none());
    }

    #[tokio::test]
    async fn test_user_remove_not_found() {
        let state = create_test_state();
        
        let params = UserRemoveQuery {
            api_key: "test-api-key".to_string(),
            passkey: "0606060606060606060606060606060606060606060606060606060606060606".to_string(),
        };

        let result = user_remove_handler(State(state), Query(params)).await;
        assert!(result.is_err());
        let response = result.unwrap_err().into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
