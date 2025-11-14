// Metrics endpoint

use crate::core::error::MonitoringError;
use crate::core::state::AppState;
use crate::utils::auth::verify_api_key;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::warn;

#[derive(Debug, Deserialize)]
pub struct MetricsQuery {
    pub api_key: String,
}

/// Returns JSON with all tracker statistics including:
/// - Total announces, successful/failed counts, success rate
/// - Active peers, torrents, users
/// - Blocked requests, banned IPs/clients
/// - Uptime and requests per second
/// 
/// Requires valid API key for authentication.
pub async fn metrics_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<MetricsQuery>,
) -> Result<Response, MonitoringError> {
    if !verify_api_key(&params.api_key, &state.config.sync.api_key) {
        warn!("Unauthorized metrics access attempt");
        return Err(MonitoringError::InvalidApiKey);
    }


    let snapshot = state.metrics.get_snapshot(
        &state.peer_store,
        &state.user_cache,
        &state.torrent_cache,
        &state.ip_blacklist,
        &state.client_blacklist,
    );

    Ok((StatusCode::OK, Json(snapshot)).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::{
        AntiCheatConfig, Config, LoggingConfig, MemoryConfig, PerformanceConfig, SecurityConfig,
        ServerConfig, SyncConfig,
    };
    use crate::metrics::collector::MetricsSnapshot;
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
    async fn test_metrics_handler_success() {
        use axum::body::Body;
        use http_body_util::BodyExt;
        
        let state = create_test_state();

        let params = MetricsQuery {
            api_key: "test-api-key".to_string(),
        };

        let response = metrics_handler(State(state), Query(params)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Verify response contains metrics
        let (parts, body) = response.into_parts();
        assert_eq!(parts.status, StatusCode::OK);

        let body = Body::new(body);
        let bytes = body.collect().await.unwrap().to_bytes();
        let snapshot: MetricsSnapshot = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(snapshot.total_announces, 0);
        assert_eq!(snapshot.active_peers, 0);
        assert!(snapshot.uptime_seconds >= 0);
    }

    #[tokio::test]
    async fn test_metrics_handler_invalid_api_key() {
        let state = create_test_state();

        let params = MetricsQuery {
            api_key: "wrong-key".to_string(),
        };

        let result = metrics_handler(State(state), Query(params)).await;
        assert!(result.is_err());
        let response = result.unwrap_err().into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_metrics_handler_with_data() {
        use axum::body::Body;
        use http_body_util::BodyExt;
        
        let state = create_test_state();

        // Add some metrics
        state.metrics.increment_announces();
        state.metrics.increment_successful();

        let params = MetricsQuery {
            api_key: "test-api-key".to_string(),
        };

        let response = metrics_handler(State(state), Query(params)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let (_, body) = response.into_parts();
        let body = Body::new(body);
        let bytes = body.collect().await.unwrap().to_bytes();
        let snapshot: MetricsSnapshot = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(snapshot.total_announces, 1);
        assert_eq!(snapshot.successful_announces, 1);
    }
}
