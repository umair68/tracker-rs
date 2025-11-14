use std::sync::atomic::{AtomicU64, Ordering};
use crate::stores::peer_store::PeerStore;
use crate::stores::user_cache::UserCache;
use crate::stores::torrent_cache::TorrentCache;
use crate::security::ip_blacklist::IpBlacklist;
use crate::security::client_blacklist::ClientBlacklist;
use serde::Serialize;

pub struct Metrics {
    pub total_announces: AtomicU64,
    pub successful_announces: AtomicU64,
    pub failed_announces: AtomicU64,
    pub blocked_requests: AtomicU64,
    pub start_time: i64,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct MetricsSnapshot {
    pub total_announces: u64,
    pub successful_announces: u64,
    pub failed_announces: u64,
    pub success_rate: f64,
    pub active_peers: usize,
    #[serde(rename = "cached_torrents")]
    pub active_torrents: usize,
    #[serde(rename = "cached_users")]
    pub active_users: usize,
    pub blocked_requests: u64,
    pub banned_ipv4: usize,
    pub banned_ipv6: usize,
    pub banned_clients: usize,
    pub uptime_seconds: i64,
    pub requests_per_second: f64,
}

impl Metrics {
    pub fn new() -> Self {
        let start_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Self {
            total_announces: AtomicU64::new(0),
            successful_announces: AtomicU64::new(0),
            failed_announces: AtomicU64::new(0),
            blocked_requests: AtomicU64::new(0),
            start_time,
        }
    }


    pub fn increment_announces(&self) {
        self.total_announces.fetch_add(1, Ordering::Relaxed);
    }


    pub fn increment_successful(&self) {
        self.successful_announces.fetch_add(1, Ordering::Relaxed);
    }


    pub fn increment_failed(&self) {
        self.failed_announces.fetch_add(1, Ordering::Relaxed);
    }


    pub fn increment_blocked(&self) {
        self.blocked_requests.fetch_add(1, Ordering::Relaxed);
    }


    /// Collects metrics from all components and calculates derived metrics
    /// like success_rate, requests_per_second, and uptime_seconds.
    pub fn get_snapshot(
        &self,
        peer_store: &PeerStore,
        user_cache: &UserCache,
        torrent_cache: &TorrentCache,
        ip_blacklist: &IpBlacklist,
        client_blacklist: &ClientBlacklist,
    ) -> MetricsSnapshot {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let total_announces = self.total_announces.load(Ordering::Relaxed);
        let successful_announces = self.successful_announces.load(Ordering::Relaxed);
        let failed_announces = self.failed_announces.load(Ordering::Relaxed);
        let blocked_requests = self.blocked_requests.load(Ordering::Relaxed);

        // Calculate success rate
        let success_rate = if total_announces > 0 {
            (successful_announces as f64 / total_announces as f64) * 100.0
        } else {
            0.0
        };

        // Calculate uptime
        let uptime_seconds = current_time - self.start_time;

        // Calculate requests per second
        let requests_per_second = if uptime_seconds > 0 {
            total_announces as f64 / uptime_seconds as f64
        } else {
            0.0
        };

        MetricsSnapshot {
            total_announces,
            successful_announces,
            failed_announces,
            success_rate,
            active_peers: peer_store.total_peers(),
            active_torrents: torrent_cache.len(),
            active_users: user_cache.len(),
            blocked_requests,
            banned_ipv4: ip_blacklist.list_ipv4().len(),
            banned_ipv6: ip_blacklist.list_ipv6().len(),
            banned_clients: client_blacklist.len(),
            uptime_seconds,
            requests_per_second,
        }
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stores::peer_store::PeerStore;
    use crate::stores::user_cache::UserCache;
    use crate::stores::torrent_cache::TorrentCache;
    use crate::security::ip_blacklist::IpBlacklist;
    use crate::security::client_blacklist::ClientBlacklist;
    use crate::models::peer::Peer;
    use crate::models::user::User;
    use crate::models::torrent::Torrent;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_new_metrics() {
        let metrics = Metrics::new();
        
        assert_eq!(metrics.total_announces.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.successful_announces.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.failed_announces.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.blocked_requests.load(Ordering::Relaxed), 0);
        assert!(metrics.start_time > 0);
    }

    #[test]
    fn test_increment_announces() {
        let metrics = Metrics::new();
        
        metrics.increment_announces();
        metrics.increment_announces();
        
        assert_eq!(metrics.total_announces.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_increment_successful() {
        let metrics = Metrics::new();
        
        metrics.increment_successful();
        metrics.increment_successful();
        metrics.increment_successful();
        
        assert_eq!(metrics.successful_announces.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn test_increment_failed() {
        let metrics = Metrics::new();
        
        metrics.increment_failed();
        
        assert_eq!(metrics.failed_announces.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_increment_blocked() {
        let metrics = Metrics::new();
        
        metrics.increment_blocked();
        metrics.increment_blocked();
        
        assert_eq!(metrics.blocked_requests.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_get_snapshot_empty() {
        let metrics = Metrics::new();
        let peer_store = PeerStore::new();
        let user_cache = UserCache::new();
        let torrent_cache = TorrentCache::new();
        let ip_blacklist = IpBlacklist::new();
        let client_blacklist = ClientBlacklist::new();
        
        let snapshot = metrics.get_snapshot(
            &peer_store,
            &user_cache,
            &torrent_cache,
            &ip_blacklist,
            &client_blacklist,
        );
        
        assert_eq!(snapshot.total_announces, 0);
        assert_eq!(snapshot.successful_announces, 0);
        assert_eq!(snapshot.failed_announces, 0);
        assert_eq!(snapshot.success_rate, 0.0);
        assert_eq!(snapshot.active_peers, 0);
        assert_eq!(snapshot.active_torrents, 0);
        assert_eq!(snapshot.active_users, 0);
        assert_eq!(snapshot.blocked_requests, 0);
        assert_eq!(snapshot.banned_ipv4, 0);
        assert_eq!(snapshot.banned_ipv6, 0);
        assert_eq!(snapshot.banned_clients, 0);
        assert!(snapshot.uptime_seconds >= 0);
        assert_eq!(snapshot.requests_per_second, 0.0);
    }

    #[test]
    fn test_get_snapshot_with_data() {
        let metrics = Metrics::new();
        let peer_store = PeerStore::new();
        let user_cache = UserCache::new();
        let torrent_cache = TorrentCache::new();
        let ip_blacklist = IpBlacklist::new();
        let client_blacklist = ClientBlacklist::new();
        
        // Add some metrics
        metrics.increment_announces();
        metrics.increment_announces();
        metrics.increment_announces();
        metrics.increment_successful();
        metrics.increment_successful();
        metrics.increment_failed();
        metrics.increment_blocked();
        
        // Add some data to stores
        let user = User {
            id: 1,
            passkey: [1u8; 32],
            class: 1,
            is_active: true,
        };
        user_cache.add_user(user);
        
        let torrent = Torrent {
            id: 1,
            info_hash: [1u8; 20],
            is_freeleech: false,
            is_active: true,
        };
        torrent_cache.add_torrent(torrent);
        
        let peer = Peer {
            user_id: 1,
            torrent_id: 1,
            peer_id: [1u8; 20],
            ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            port: 6881,
            uploaded: 1024,
            downloaded: 512,
            left: 0,
            last_announce: 1000,
            user_agent: "TestClient/1.0".to_string(),
            is_seeder: true,
        };
        peer_store.add_peer([1u8; 20], peer).unwrap();
        
        ip_blacklist.ban(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
        client_blacklist.ban("BadClient".to_string());
        
        let snapshot = metrics.get_snapshot(
            &peer_store,
            &user_cache,
            &torrent_cache,
            &ip_blacklist,
            &client_blacklist,
        );
        
        assert_eq!(snapshot.total_announces, 3);
        assert_eq!(snapshot.successful_announces, 2);
        assert_eq!(snapshot.failed_announces, 1);
        assert!((snapshot.success_rate - 66.666).abs() < 0.01);
        assert_eq!(snapshot.active_peers, 1);
        assert_eq!(snapshot.active_torrents, 1);
        assert_eq!(snapshot.active_users, 1);
        assert_eq!(snapshot.blocked_requests, 1);
        assert_eq!(snapshot.banned_ipv4, 1);
        assert_eq!(snapshot.banned_ipv6, 0);
        assert_eq!(snapshot.banned_clients, 1);
        assert!(snapshot.uptime_seconds >= 0);
        // requests_per_second can be 0.0 if uptime is 0 (test runs too fast)
        assert!(snapshot.requests_per_second >= 0.0);
    }

    #[test]
    fn test_success_rate_calculation() {
        let metrics = Metrics::new();
        let peer_store = PeerStore::new();
        let user_cache = UserCache::new();
        let torrent_cache = TorrentCache::new();
        let ip_blacklist = IpBlacklist::new();
        let client_blacklist = ClientBlacklist::new();
        
        // 8 successful out of 10 total = 80%
        for _ in 0..10 {
            metrics.increment_announces();
        }
        for _ in 0..8 {
            metrics.increment_successful();
        }
        for _ in 0..2 {
            metrics.increment_failed();
        }
        
        let snapshot = metrics.get_snapshot(
            &peer_store,
            &user_cache,
            &torrent_cache,
            &ip_blacklist,
            &client_blacklist,
        );
        
        assert_eq!(snapshot.success_rate, 80.0);
    }

    #[test]
    fn test_requests_per_second_calculation() {
        let metrics = Metrics::new();
        let peer_store = PeerStore::new();
        let user_cache = UserCache::new();
        let torrent_cache = TorrentCache::new();
        let ip_blacklist = IpBlacklist::new();
        let client_blacklist = ClientBlacklist::new();
        
        // Add some announces
        for _ in 0..100 {
            metrics.increment_announces();
        }
        
        let snapshot = metrics.get_snapshot(
            &peer_store,
            &user_cache,
            &torrent_cache,
            &ip_blacklist,
            &client_blacklist,
        );
        
        // Verify the calculation logic
        assert!(snapshot.uptime_seconds >= 0);
        assert!(snapshot.requests_per_second >= 0.0);
        
        // If uptime > 0, verify the calculation is correct
        if snapshot.uptime_seconds > 0 {
            let expected_rps = 100.0 / snapshot.uptime_seconds as f64;
            assert!((snapshot.requests_per_second - expected_rps).abs() < 0.01);
        }
    }
}
