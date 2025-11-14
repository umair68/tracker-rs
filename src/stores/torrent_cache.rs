use crate::models::torrent::Torrent;
use dashmap::DashMap;
use std::sync::Arc;

/// In-memory cache for torrent data
pub struct TorrentCache {
    torrents: DashMap<[u8; 20], Arc<Torrent>>,
}

impl TorrentCache {
    /// Create a new TorrentCache instance
    pub fn new() -> Self {
        Self {
            torrents: DashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            torrents: DashMap::with_capacity(capacity),
        }
    }

    /// Add a torrent to the cache
    /// If a torrent with the same info_hash already exists, it will be replaced
    pub fn add_torrent(&self, torrent: Torrent) {
        let info_hash = torrent.info_hash;
        self.torrents.insert(info_hash, Arc::new(torrent));
    }

    /// Remove a torrent from the cache by info_hash
    /// Returns the removed torrent if it existed
    pub fn remove_torrent(&self, info_hash: [u8; 20]) -> Option<Arc<Torrent>> {
        self.torrents.remove(&info_hash).map(|(_, torrent)| torrent)
    }

    /// Get a torrent from the cache by info_hash
    /// Returns a clone of the torrent if found
    pub fn get_torrent(&self, info_hash: [u8; 20]) -> Option<Arc<Torrent>> {
        self.torrents.get(&info_hash).map(|entry| Arc::clone(entry.value()))
    }

    pub fn clear(&self) {
        self.torrents.clear();
    }


    pub fn len(&self) -> usize {
        self.torrents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.torrents.is_empty()
    }
}

impl Default for TorrentCache {
    fn default() -> Self {
        Self::new()
    }
}
