use crate::core::error::BlacklistError;
use crate::models::admin::{
    ClientBanQuery, ClientListResponse, IpBanQuery, IpListResponse,
    SuccessResponse,
};
use crate::core::state::AppState;
use crate::utils::auth::verify_api_key;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use std::sync::Arc;
use tracing::{info, warn};

/// Ban an IP address
pub async fn ip_ban_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<IpBanQuery>,
) -> Result<Response, BlacklistError> {
    if !verify_api_key(&params.api_key, &state.config.sync.api_key) {
        warn!("Unauthorized IP ban attempt");
        return Err(BlacklistError::InvalidApiKey);
    }


    let ip = params.ip.parse()
        .map_err(|e| BlacklistError::InvalidIpAddress(format!("{}: {}", params.ip, e)))?;


    state.ip_blacklist.ban(ip);

    info!(ip = %params.ip, "IP address banned");

    Ok((
        StatusCode::OK,
        Json(SuccessResponse {
            success: true,
            message: "IP address banned successfully".to_string(),
        }),
    )
        .into_response())
}

/// Unban an IP address
pub async fn ip_unban_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<IpBanQuery>,
) -> Result<Response, BlacklistError> {
    // Verify API key
    if !verify_api_key(&params.api_key, &state.config.sync.api_key) {
        warn!("Unauthorized IP unban attempt");
        return Err(BlacklistError::InvalidApiKey);
    }


    let ip = params.ip.parse()
        .map_err(|e| BlacklistError::InvalidIpAddress(format!("{}: {}", params.ip, e)))?;


    state.ip_blacklist.unban(ip);

    info!(ip = %params.ip, "IP address unbanned");

    Ok((
        StatusCode::OK,
        Json(SuccessResponse {
            success: true,
            message: "IP address unbanned successfully".to_string(),
        }),
    )
        .into_response())
}

/// List all banned IP addresses
pub async fn ip_list_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<crate::models::admin::ApiKeyQuery>,
) -> Result<Response, BlacklistError> {
    // Verify API key
    if !verify_api_key(&params.api_key, &state.config.sync.api_key) {
        warn!("Unauthorized IP list attempt");
        return Err(BlacklistError::InvalidApiKey);
    }

    // Get all banned IPs
    let ipv4 = state
        .ip_blacklist
        .list_ipv4()
        .iter()
        .map(|ip| ip.to_string())
        .collect();
    let ipv6 = state
        .ip_blacklist
        .list_ipv6()
        .iter()
        .map(|ip| ip.to_string())
        .collect();

    Ok((
        StatusCode::OK,
        Json(IpListResponse {
            success: true,
            ipv4,
            ipv6,
        }),
    )
        .into_response())
}

/// Ban a BitTorrent client
pub async fn client_ban_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ClientBanQuery>,
) -> Result<Response, BlacklistError> {
    if !verify_api_key(&params.api_key, &state.config.sync.api_key) {
        warn!("Unauthorized client ban attempt");
        return Err(BlacklistError::InvalidApiKey);
    }

    state.client_blacklist.ban(params.client.clone());

    info!(client = %params.client, "Client banned");

    Ok((
        StatusCode::OK,
        Json(SuccessResponse {
            success: true,
            message: "Client banned successfully".to_string(),
        }),
    )
        .into_response())
}

/// Unban a BitTorrent client
pub async fn client_unban_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ClientBanQuery>,
) -> Result<Response, BlacklistError> {
    if !verify_api_key(&params.api_key, &state.config.sync.api_key) {
        warn!("Unauthorized client unban attempt");
        return Err(BlacklistError::InvalidApiKey);
    }

    // Unban the client
    state.client_blacklist.unban(&params.client);

    info!(client = %params.client, "Client unbanned");

    Ok((
        StatusCode::OK,
        Json(SuccessResponse {
            success: true,
            message: "Client unbanned successfully".to_string(),
        }),
    )
        .into_response())
}

/// List all banned clients
pub async fn client_list_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<crate::models::admin::ApiKeyQuery>,
) -> Result<Response, BlacklistError> {
    if !verify_api_key(&params.api_key, &state.config.sync.api_key) {
        warn!("Unauthorized client list attempt");
        return Err(BlacklistError::InvalidApiKey);
    }

    // Get all banned clients
    let clients = state.client_blacklist.list();

    Ok((
        StatusCode::OK,
        Json(ClientListResponse {
            success: true,
            clients,
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
    async fn test_ip_ban_success() {
        let state = create_test_state();

        let params = IpBanQuery {
            api_key: "test-api-key".to_string(),
            ip: "192.168.1.1".to_string(),
        };

        let response = ip_ban_handler(State(state.clone()), Query(params)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Verify IP was banned
        let ip = "192.168.1.1".parse().unwrap();
        assert!(state.ip_blacklist.is_banned(ip));
    }

    #[tokio::test]
    async fn test_ip_ban_invalid_api_key() {
        let state = create_test_state();

        let params = IpBanQuery {
            api_key: "wrong-key".to_string(),
            ip: "192.168.1.1".to_string(),
        };

        let result = ip_ban_handler(State(state), Query(params)).await;
        assert!(result.is_err());
        let response = result.unwrap_err().into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_ip_ban_invalid_ip() {
        let state = create_test_state();

        let params = IpBanQuery {
            api_key: "test-api-key".to_string(),
            ip: "invalid-ip".to_string(),
        };

        let result = ip_ban_handler(State(state), Query(params)).await;
        assert!(result.is_err());
        let response = result.unwrap_err().into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_ip_unban_success() {
        let state = create_test_state();

        // First ban an IP
        let ip = "192.168.1.1".parse().unwrap();
        state.ip_blacklist.ban(ip);
        assert!(state.ip_blacklist.is_banned(ip));


        let params = IpBanQuery {
            api_key: "test-api-key".to_string(),
            ip: "192.168.1.1".to_string(),
        };

        let response = ip_unban_handler(State(state.clone()), Query(params)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);


        assert!(!state.ip_blacklist.is_banned(ip));
    }

    #[tokio::test]
    async fn test_ip_list_success() {
        let state = create_test_state();


        state.ip_blacklist.ban("192.168.1.1".parse().unwrap());
        state.ip_blacklist.ban("10.0.0.1".parse().unwrap());
        state.ip_blacklist.ban("2001:db8::1".parse().unwrap());

        let params = crate::models::admin::ApiKeyQuery {
            api_key: "test-api-key".to_string(),
        };

        let response = ip_list_handler(State(state), Query(params)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_ip_list_invalid_api_key() {
        let state = create_test_state();

        let params = crate::models::admin::ApiKeyQuery {
            api_key: "wrong-key".to_string(),
        };

        let result = ip_list_handler(State(state), Query(params)).await;
        assert!(result.is_err());
        let response = result.unwrap_err().into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_client_ban_success() {
        let state = create_test_state();

        let params = ClientBanQuery {
            api_key: "test-api-key".to_string(),
            client: "BadClient".to_string(),
        };

        let response = client_ban_handler(State(state.clone()), Query(params)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);


        assert!(state.client_blacklist.is_banned("BadClient/1.0"));
    }

    #[tokio::test]
    async fn test_client_ban_invalid_api_key() {
        let state = create_test_state();

        let params = ClientBanQuery {
            api_key: "wrong-key".to_string(),
            client: "BadClient".to_string(),
        };

        let result = client_ban_handler(State(state), Query(params)).await;
        assert!(result.is_err());
        let response = result.unwrap_err().into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_client_unban_success() {
        let state = create_test_state();

        // First ban a client
        state.client_blacklist.ban("BadClient".to_string());
        assert!(state.client_blacklist.is_banned("BadClient/1.0"));

        // Now unban it
        let params = ClientBanQuery {
            api_key: "test-api-key".to_string(),
            client: "BadClient".to_string(),
        };

        let response = client_unban_handler(State(state.clone()), Query(params)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Verify client was unbanned
        assert!(!state.client_blacklist.is_banned("BadClient/1.0"));
    }

    #[tokio::test]
    async fn test_client_list_success() {
        let state = create_test_state();

        // Ban some clients
        state.client_blacklist.ban("BadClient1".to_string());
        state.client_blacklist.ban("BadClient2".to_string());

        let params = crate::models::admin::ApiKeyQuery {
            api_key: "test-api-key".to_string(),
        };

        let response = client_list_handler(State(state), Query(params)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_client_list_invalid_api_key() {
        let state = create_test_state();

        let params = crate::models::admin::ApiKeyQuery {
            api_key: "wrong-key".to_string(),
        };

        let result = client_list_handler(State(state), Query(params)).await;
        assert!(result.is_err());
        let response = result.unwrap_err().into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
