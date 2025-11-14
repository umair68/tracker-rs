use crate::core::error::MonitoringError;
use crate::core::state::AppState;
use crate::utils::auth::verify_api_key;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::sync::Arc;
use tracing::warn;

#[derive(Debug, Deserialize)]
pub struct UpdateQuery {
    pub api_key: String,
}

/// Peer data for external API
#[derive(Debug, Serialize, Deserialize)]
pub struct PeerUpdate {
    pub torrent_id: u32,
    pub user_id: u32,
    pub peer_id: String,
    pub ipv4: Option<String>,
    pub ipv6: Option<String>,
    pub port: u16,
    pub uploaded: u64,
    pub downloaded: u64,
    pub left: u64,
    pub last_announce: i64,
    pub user_agent: String,
    pub user_class: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TorrentUpdate {
    pub torrent_id: u32,
    pub seeders: u32,
    pub leechers: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateResponse {
    pub peers: Vec<PeerUpdate>,
    pub torrents: Vec<TorrentUpdate>,
    pub timestamp: i64,
}

/// Update handler
/// 
/// Returns JSON containing all active peers and torrent statistics.
/// Used by the external API to synchronize tracker state.
/// 
/// Response includes:
/// - peers: Array of peer data with torrent_id, user_id, peer_id, IP, port, stats, user_agent, user_class
/// - torrents: Array of torrent stats with torrent_id, seeders, leechers
/// - timestamp: Current Unix timestamp
/// 
/// Requires valid API key for authentication.
pub async fn update_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<UpdateQuery>,
) -> Result<Response, MonitoringError> {
    if !verify_api_key(&params.api_key, &state.config.sync.api_key) {
        warn!("Unauthorized update access attempt");
        return Err(MonitoringError::InvalidApiKey);
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let mut peers = Vec::new();
    let mut torrents = Vec::new();


    for torrent_entry in state.peer_store.peers.iter() {
        let info_hash = *torrent_entry.key();
        let peer_map = torrent_entry.value();


        let (seeders, leechers) = state.peer_store.get_stats(info_hash);


        if let Some(torrent) = state.torrent_cache.get_torrent(info_hash) {
            torrents.push(TorrentUpdate {
                torrent_id: torrent.id,
                seeders,
                leechers,
            });


            for peer_entry in peer_map.iter() {
                let peer = peer_entry.value();

                // Get user class from user cache
                let user_class = if let Some(user) = state.user_cache.get_user_by_id(peer.user_id) {
                    user.class
                } else {
                    0 // Default class if user not found
                };


                let peer_id_hex = hex::encode(peer.peer_id);

                // Split IP into IPv4 and IPv6
                let (ipv4, ipv6) = match peer.ip {
                    IpAddr::V4(ip) => (Some(ip.to_string()), None),
                    IpAddr::V6(ip) => (None, Some(ip.to_string())),
                };

                peers.push(PeerUpdate {
                    torrent_id: torrent.id,
                    user_id: peer.user_id,
                    peer_id: peer_id_hex,
                    ipv4,
                    ipv6,
                    port: peer.port,
                    uploaded: peer.uploaded,
                    downloaded: peer.downloaded,
                    left: peer.left,
                    last_announce: peer.last_announce,
                    user_agent: peer.user_agent.clone(),
                    user_class,
                });
            }
        }
    }

    let response = UpdateResponse {
        peers,
        torrents,
        timestamp,
    };

    Ok((StatusCode::OK, Json(response)).into_response())
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::{
        AntiCheatConfig, Config, LoggingConfig, MemoryConfig, PerformanceConfig, SecurityConfig,
        ServerConfig, SyncConfig,
    };
    use crate::models::peer::Peer;
    use crate::models::torrent::Torrent;
    use crate::models::user::User;
    use crate::wal::wal::Wal;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
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
    async fn test_update_handler_success() {
        use axum::body::Body;
        use http_body_util::BodyExt;
        
        let state = create_test_state();

        let params = UpdateQuery {
            api_key: "test-api-key".to_string(),
        };

        let response = update_handler(State(state), Query(params)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Verify response structure
        let (_, body) = response.into_parts();
        let body = Body::new(body);
        let bytes = body.collect().await.unwrap().to_bytes();
        let update: UpdateResponse = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(update.peers.len(), 0);
        assert_eq!(update.torrents.len(), 0);
        assert!(update.timestamp > 0);
    }

    #[tokio::test]
    async fn test_update_handler_invalid_api_key() {
        let state = create_test_state();

        let params = UpdateQuery {
            api_key: "wrong-key".to_string(),
        };

        let result = update_handler(State(state), Query(params)).await;
        assert!(result.is_err());
        let response = result.unwrap_err().into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_update_handler_with_peers() {
        use axum::body::Body;
        use http_body_util::BodyExt;
        
        let state = create_test_state();

        // Add a user
        let user = User::new(123, [1u8; 32], 2, true);
        state.user_cache.add_user(user);

        // Add a torrent
        let info_hash = [2u8; 20];
        let torrent = Torrent::new(456, info_hash, false, true);
        state.torrent_cache.add_torrent(torrent);

        // Add a peer
        let peer = Peer::new(
            123,
            456,
            [3u8; 20],
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            6881,
            1024,
            512,
            0,
            1000,
            "TestClient/1.0".to_string(),
        );
        state.peer_store.add_peer(info_hash, peer).unwrap();

        let params = UpdateQuery {
            api_key: "test-api-key".to_string(),
        };

        let response = update_handler(State(state), Query(params)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let (_, body) = response.into_parts();
        let body = Body::new(body);
        let bytes = body.collect().await.unwrap().to_bytes();
        let update: UpdateResponse = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(update.peers.len(), 1);
        assert_eq!(update.torrents.len(), 1);

        let peer_update = &update.peers[0];
        assert_eq!(peer_update.torrent_id, 456);
        assert_eq!(peer_update.user_id, 123);
        assert_eq!(peer_update.ipv4, Some("192.168.1.1".to_string()));
        assert_eq!(peer_update.ipv6, None);
        assert_eq!(peer_update.port, 6881);
        assert_eq!(peer_update.uploaded, 1024);
        assert_eq!(peer_update.downloaded, 512);
        assert_eq!(peer_update.left, 0);
        assert_eq!(peer_update.user_agent, "TestClient/1.0");
        assert_eq!(peer_update.user_class, 2);

        let torrent_update = &update.torrents[0];
        assert_eq!(torrent_update.torrent_id, 456);
        assert_eq!(torrent_update.seeders, 1);
        assert_eq!(torrent_update.leechers, 0);
    }

    #[tokio::test]
    async fn test_update_handler_with_ipv6_peer() {
        use axum::body::Body;
        use http_body_util::BodyExt;
        
        let state = create_test_state();

        // Add a user
        let user = User::new(789, [4u8; 32], 1, true);
        state.user_cache.add_user(user);

        // Add a torrent
        let info_hash = [5u8; 20];
        let torrent = Torrent::new(999, info_hash, true, true);
        state.torrent_cache.add_torrent(torrent);

        // Add an IPv6 peer
        let peer = Peer::new(
            789,
            999,
            [6u8; 20],
            IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
            51413,
            2048,
            1024,
            500,
            2000,
            "qBittorrent/4.5.0".to_string(),
        );
        state.peer_store.add_peer(info_hash, peer).unwrap();

        let params = UpdateQuery {
            api_key: "test-api-key".to_string(),
        };

        let response = update_handler(State(state), Query(params)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let (_, body) = response.into_parts();
        let body = Body::new(body);
        let bytes = body.collect().await.unwrap().to_bytes();
        let update: UpdateResponse = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(update.peers.len(), 1);

        let peer_update = &update.peers[0];
        assert_eq!(peer_update.ipv4, None);
        assert_eq!(peer_update.ipv6, Some("2001:db8::1".to_string()));
        assert_eq!(peer_update.user_class, 1);
    }

    #[tokio::test]
    async fn test_update_handler_multiple_peers_and_torrents() {
        use axum::body::Body;
        use http_body_util::BodyExt;
        
        let state = create_test_state();

        // Add users
        for i in 1..=3 {
            let user = User::new(i, [i as u8; 32], i as u8, true);
            state.user_cache.add_user(user);
        }

        // Add torrents and peers
        for i in 1..=2 {
            let info_hash = [i as u8; 20];
            let torrent = Torrent::new(i * 100, info_hash, false, true);
            state.torrent_cache.add_torrent(torrent);

            // Add 2 peers per torrent
            for j in 1..=2 {
                let peer = Peer::new(
                    j,
                    i * 100,
                    [(i * 10 + j) as u8; 20],
                    IpAddr::V4(Ipv4Addr::new(192, 168, i as u8, j as u8)),
                    6881 + j as u16,
                    1024 * j as u64,
                    512 * j as u64,
                    if j == 1 { 0 } else { 1000 },
                    1000 + j as i64,
                    format!("Client{}/1.0", j),
                );
                state.peer_store.add_peer(info_hash, peer).unwrap();
            }
        }

        let params = UpdateQuery {
            api_key: "test-api-key".to_string(),
        };

        let response = update_handler(State(state), Query(params)).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let (_, body) = response.into_parts();
        let body = Body::new(body);
        let bytes = body.collect().await.unwrap().to_bytes();
        let update: UpdateResponse = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(update.peers.len(), 4); // 2 torrents * 2 peers
        assert_eq!(update.torrents.len(), 2);

        // Verify torrent stats
        for torrent_update in &update.torrents {
            assert_eq!(torrent_update.seeders, 1);
            assert_eq!(torrent_update.leechers, 1);
        }
    }
}
