// Application state (AppState)

use crate::core::config::Config;
use crate::metrics::collector::Metrics;
use crate::security::{client_blacklist::ClientBlacklist, ip_blacklist::IpBlacklist, rate_limiter::RateLimiter};
use crate::stores::{peer_store::PeerStore, torrent_cache::TorrentCache, user_cache::UserCache};
use crate::wal::wal::Wal;
use std::sync::Arc;

/// Shared application state
/// 
/// Contains all shared components that are accessed by request handlers.
/// All fields are wrapped in Arc for efficient cloning across threads.
#[derive(Clone)]
pub struct AppState {
    /// Peer store for tracking active peers
    pub peer_store: Arc<PeerStore>,
    
    /// User cache for authentication
    pub user_cache: Arc<UserCache>,
    
    /// Torrent cache for authorization
    pub torrent_cache: Arc<TorrentCache>,
    
    /// IP blacklist for banning malicious IPs
    pub ip_blacklist: Arc<IpBlacklist>,
    
    /// Client blacklist for banning malicious clients
    pub client_blacklist: Arc<ClientBlacklist>,
    
    /// Rate limiter for preventing abuse
    pub rate_limiter: Arc<RateLimiter>,
    
    /// Metrics collector for tracking statistics
    pub metrics: Arc<Metrics>,
    
    /// Write-Ahead Log for persistence
    pub wal: Arc<Wal>,
    
    /// Configuration
    pub config: Arc<Config>,
}

impl AppState {
    pub fn new(config: Config, wal: Wal) -> Self {
        let config = Arc::new(config);
        
        let ip_blacklist = Arc::new(IpBlacklist::with_banned_ips(&config.security.banned_ips));
        
        let client_blacklist = Arc::new(ClientBlacklist::with_banned_clients(&config.security.banned_clients));
        
        let rate_limiter = Arc::new(RateLimiter::new(config.performance.max_requests_per_minute));
        
        Self {
            peer_store: Arc::new(PeerStore::new()),
            user_cache: Arc::new(UserCache::with_capacity(config.memory.user_cache_size)),
            torrent_cache: Arc::new(TorrentCache::with_capacity(config.memory.torrent_cache_size)),
            ip_blacklist,
            client_blacklist,
            rate_limiter,
            metrics: Arc::new(Metrics::new()),
            wal: Arc::new(wal),
            config,
        }
    }
}
