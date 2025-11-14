use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// API client for communicating with the external backend
pub struct ApiClient {
    client: reqwest::Client,
    endpoint: String,
    api_key: String,
}

#[derive(Debug, Deserialize)]
pub struct ApiData {
    pub torrents: Vec<ApiTorrent>,
    pub users: Vec<ApiUser>,
    #[serde(default)]
    pub pagination: Option<ApiPagination>,
    #[serde(default)]
    pub timestamp: Option<i64>,
}

/// Pagination information from API
#[derive(Debug, Deserialize)]
pub struct ApiPagination {
    pub current_page: u32,
    pub per_page: u32,
    pub total_torrents: u32,
    pub total_users: u32,
}

#[derive(Debug, Deserialize)]
pub struct ApiTorrent {
    pub id: u32,
    pub info_hash: String, // hex-encoded
    pub is_freeleech: bool,
    #[serde(default)]
    pub seeders: u32,
    #[serde(default)]
    pub leechers: u32,
}

#[derive(Debug, Deserialize)]
pub struct ApiUser {
    pub id: u32,
    pub passkey: String, // hex-encoded (32 characters)
    pub user_class_id: u8,
    pub can_download: bool,
    #[serde(default)]
    pub security_locked: bool,
    #[serde(default)]
    pub has_freeleech: bool,
}

#[derive(Debug, Serialize)]
pub struct UpdateData {
    pub peers: Vec<PeerUpdate>,
    pub torrents: Vec<TorrentUpdate>,
    pub timestamp: i64,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
pub struct TorrentUpdate {
    pub torrent_id: u32,
    pub seeders: u32,
    pub leechers: u32,
}

impl ApiClient {
    pub fn new(endpoint: String, api_key: String) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            endpoint,
            api_key,
        })
    }

    /// Fetch user and torrent data from the external API
    /// Handles pagination automatically by fetching all pages
    pub async fn fetch_data(&self) -> Result<ApiData> {
        let mut all_torrents = Vec::new();
        let mut all_users = Vec::new();
        let mut page = 1;
        let mut last_pagination = None;
        let mut last_timestamp = None;

        loop {
            let response = self
                .client
                .get(&self.endpoint)
                .query(&[("api_key", &self.api_key), ("page", &page.to_string())])
                .send()
                .await
                .context("Failed to send request to external API")?;

            if !response.status().is_success() {
                bail!(
                    "External API returned error status: {}",
                    response.status()
                );
            }

            let data = response
                .json::<ApiData>()
                .await
                .context("Failed to parse JSON response from external API")?;

            let has_more = !data.torrents.is_empty() || !data.users.is_empty();
            
            all_torrents.extend(data.torrents);
            all_users.extend(data.users);
            
            if data.pagination.is_some() {
                last_pagination = data.pagination;
            }
            if data.timestamp.is_some() {
                last_timestamp = data.timestamp;
            }

            if !has_more {
                break;
            }

            page += 1;
            
            // Safety check: don't loop forever
            if page > 1000 {
                bail!("Too many pages (>1000), possible infinite loop");
            }
        }

        Ok(ApiData {
            torrents: all_torrents,
            users: all_users,
            pagination: last_pagination,
            timestamp: last_timestamp,
        })
    }

    /// Upload peer data to the external API
    pub async fn upload_peer_data(&self, data: UpdateData) -> Result<()> {
        let response = self
            .client
            .post(&self.endpoint)
            .query(&[("api_key", &self.api_key)])
            .json(&data)
            .send()
            .await
            .context("Failed to send update data to external API")?;

        if !response.status().is_success() {
            bail!(
                "External API returned error status: {}",
                response.status()
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_client_creation() {
        let client = ApiClient::new(
            "http://localhost:8000/api/tracker/data".to_string(),
            "test-api-key".to_string(),
        );
        assert!(client.is_ok());
    }

    #[test]
    fn test_update_data_serialization() {
        let update = UpdateData {
            peers: vec![PeerUpdate {
                torrent_id: 123,
                user_id: 456,
                peer_id: "abcd1234".to_string(),
                ipv4: Some("192.168.1.1".to_string()),
                ipv6: None,
                port: 51413,
                uploaded: 1024,
                downloaded: 512,
                left: 0,
                last_announce: 1699564800,
                user_agent: "qBittorrent/4.5.0".to_string(),
                user_class: 1,
            }],
            torrents: vec![TorrentUpdate {
                torrent_id: 123,
                seeders: 5,
                leechers: 3,
            }],
            timestamp: 1699564800,
        };

        let json = serde_json::to_string(&update);
        assert!(json.is_ok());
    }
}
