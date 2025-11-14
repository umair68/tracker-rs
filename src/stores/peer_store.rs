use crate::models::peer::Peer;
use dashmap::DashMap;
use dashmap::DashSet;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use anyhow::{Result, Context};
use rand::seq::SliceRandom;

#[derive(Debug)]
pub struct TorrentStats {
    pub seeders: AtomicU32,
    pub leechers: AtomicU32,
}

impl TorrentStats {
    fn new() -> Self {
        Self {
            seeders: AtomicU32::new(0),
            leechers: AtomicU32::new(0),
        }
    }
}

/// In-memory peer store 
pub struct PeerStore {
    pub peers: DashMap<[u8; 20], DashMap<[u8; 20], Peer>>,
    stats: DashMap<[u8; 20], Arc<TorrentStats>>,
    user_ips: DashMap<(u32, u32), DashSet<IpAddr>>,
}

impl PeerStore {
    pub fn new() -> Self {
        Self {
            peers: DashMap::new(),
            stats: DashMap::new(),
            user_ips: DashMap::new(),
        }
    }

    /// Add a new peer to the store
    pub fn add_peer(&self, info_hash: [u8; 20], peer: Peer) -> Result<()> {
        let peer_map = self.peers.entry(info_hash).or_insert_with(DashMap::new);
        let stats = self.stats.entry(info_hash).or_insert_with(|| Arc::new(TorrentStats::new()));
        
        let user_ips = self.user_ips
            .entry((peer.user_id, peer.torrent_id))
            .or_insert_with(DashSet::new);
        user_ips.insert(peer.ip);
        
        let is_new = !peer_map.contains_key(&peer.peer_id);
        
        if is_new {
            if peer.is_seeder {
                stats.seeders.fetch_add(1, Ordering::Relaxed);
            } else {
                stats.leechers.fetch_add(1, Ordering::Relaxed);
            }
        }
        
        peer_map.insert(peer.peer_id, peer);
        
        Ok(())
    }

    /// Update an existing peer in the store
    pub fn update_peer(&self, info_hash: [u8; 20], peer_id: [u8; 20], peer: Peer) -> Result<()> {
        let peer_map = self.peers
            .get(&info_hash)
            .context("Torrent not found in peer store")?;
        
        let stats = self.stats
            .get(&info_hash)
            .context("Stats not found for torrent")?;
        
        let user_ips = self.user_ips
            .entry((peer.user_id, peer.torrent_id))
            .or_insert_with(DashSet::new);
        user_ips.insert(peer.ip);
        
        if let Some(old_peer) = peer_map.get(&peer_id) {
            if old_peer.is_seeder != peer.is_seeder {
                if peer.is_seeder {
                    stats.leechers.fetch_sub(1, Ordering::Relaxed);
                    stats.seeders.fetch_add(1, Ordering::Relaxed);
                } else {
                    stats.seeders.fetch_sub(1, Ordering::Relaxed);
                    stats.leechers.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        
        peer_map.insert(peer_id, peer);
        
        Ok(())
    }

    /// Remove a peer from the store
    pub fn remove_peer(&self, info_hash: [u8; 20], peer_id: [u8; 20]) -> Result<()> {
        let peer_map = self.peers
            .get(&info_hash)
            .context("Torrent not found in peer store")?;
        
        let stats = self.stats
            .get(&info_hash)
            .context("Stats not found for torrent")?;
        
        if let Some((_, peer)) = peer_map.remove(&peer_id) {
            if peer.is_seeder {
                stats.seeders.fetch_sub(1, Ordering::Relaxed);
            } else {
                stats.leechers.fetch_sub(1, Ordering::Relaxed);
            }
            
            if let Some(user_ips) = self.user_ips.get(&(peer.user_id, peer.torrent_id)) {
                user_ips.remove(&peer.ip);
                
                if user_ips.is_empty() {
                    drop(user_ips);
                    self.user_ips.remove(&(peer.user_id, peer.torrent_id));
                }
            }
        }
        
        Ok(())
    }

    /// Get a list of peers for a torrent with random selection and numwant limit
    pub fn get_peers(
        &self,
        info_hash: [u8; 20],
        num_want: u32,
        exclude_peer_id: [u8; 20],
    ) -> Vec<Peer> {
        let peer_map = match self.peers.get(&info_hash) {
            Some(map) => map,
            None => return Vec::new(),
        };
        
        let estimated_size = peer_map.len().saturating_sub(1).min(num_want as usize);
        let mut peers: Vec<Peer> = Vec::with_capacity(estimated_size);
        
        for entry in peer_map.iter() {
            if *entry.key() != exclude_peer_id {
                peers.push(entry.value().clone());
            }
        }
        
        drop(peer_map);
        
        let mut rng = rand::thread_rng();
        peers.shuffle(&mut rng);
        
        peers.truncate(num_want as usize);
        
        peers
    }

    /// Get statistics (seeders, leechers) for a torrent
    pub fn get_stats(&self, info_hash: [u8; 20]) -> (u32, u32) {
        match self.stats.get(&info_hash) {
            Some(stats) => (
                stats.seeders.load(Ordering::Relaxed),
                stats.leechers.load(Ordering::Relaxed),
            ),
            None => (0, 0),
        }
    }

    /// Get the number of unique IPs a user is using for a torrent (for duplicate peer detection)
    pub fn get_user_ip_count(&self, user_id: u32, torrent_id: u32) -> usize {
        match self.user_ips.get(&(user_id, torrent_id)) {
            Some(ips) => ips.len(),
            None => 0,
        }
    }

    /// Clean up stale peers that haven't announced within the timeout period
    pub fn cleanup_stale_peers(&self, timeout: i64) -> usize {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        let mut removed_count = 0;
        
        for torrent_entry in self.peers.iter() {
            let info_hash = *torrent_entry.key();
            let peer_map = torrent_entry.value();
            
            let stats = match self.stats.get(&info_hash) {
                Some(s) => s,
                None => continue,
            };
            
            let estimated_stale = peer_map.len() / 10;
            let mut stale_peers: Vec<([u8; 20], Peer)> = Vec::with_capacity(estimated_stale);
            
            for entry in peer_map.iter() {
                if current_time - entry.value().last_announce > timeout {
                    stale_peers.push((*entry.key(), entry.value().clone()));
                }
            }
            
            for (peer_id, peer) in stale_peers {
                peer_map.remove(&peer_id);
                
                if peer.is_seeder {
                    stats.seeders.fetch_sub(1, Ordering::Relaxed);
                } else {
                    stats.leechers.fetch_sub(1, Ordering::Relaxed);
                }
                
                if let Some(user_ips) = self.user_ips.get(&(peer.user_id, peer.torrent_id)) {
                    user_ips.remove(&peer.ip);
                    
                    if user_ips.is_empty() {
                        drop(user_ips);
                        self.user_ips.remove(&(peer.user_id, peer.torrent_id));
                    }
                }
                
                removed_count += 1;
            }
        }
        
        removed_count
    }

    /// Get the total number of active peers across all torrents
    pub fn total_peers(&self) -> usize {
        self.peers.iter().map(|entry| entry.value().len()).sum()
    }

    /// Get the number of torrents with active peers
    pub fn active_torrents(&self) -> usize {
        self.peers.len()
    }
}

impl Default for PeerStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    fn create_test_peer(
        user_id: u32,
        torrent_id: u32,
        peer_id: [u8; 20],
        ip: IpAddr,
        is_seeder: bool,
        last_announce: i64,
    ) -> Peer {
        Peer {
            user_id,
            torrent_id,
            peer_id,
            ip,
            port: 6881,
            uploaded: 1024,
            downloaded: 512,
            left: if is_seeder { 0 } else { 1000 },
            last_announce,
            user_agent: "TestClient/1.0".to_string(),
            is_seeder,
        }
    }

    #[test]
    fn test_add_peer() {
        let store = PeerStore::new();
        let info_hash = [1u8; 20];
        let peer_id = [2u8; 20];
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        
        let peer = create_test_peer(1, 1, peer_id, ip, false, 1000);
        
        store.add_peer(info_hash, peer).unwrap();
        
        let (seeders, leechers) = store.get_stats(info_hash);
        assert_eq!(seeders, 0);
        assert_eq!(leechers, 1);
    }

    #[test]
    fn test_update_peer_seeder_status() {
        let store = PeerStore::new();
        let info_hash = [1u8; 20];
        let peer_id = [2u8; 20];
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        
        // Add as leecher
        let peer = create_test_peer(1, 1, peer_id, ip, false, 1000);
        store.add_peer(info_hash, peer).unwrap();
        
        let (seeders, leechers) = store.get_stats(info_hash);
        assert_eq!(seeders, 0);
        assert_eq!(leechers, 1);
        
        // Update to seeder
        let peer = create_test_peer(1, 1, peer_id, ip, true, 2000);
        store.update_peer(info_hash, peer_id, peer).unwrap();
        
        let (seeders, leechers) = store.get_stats(info_hash);
        assert_eq!(seeders, 1);
        assert_eq!(leechers, 0);
    }

    #[test]
    fn test_remove_peer() {
        let store = PeerStore::new();
        let info_hash = [1u8; 20];
        let peer_id = [2u8; 20];
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        
        let peer = create_test_peer(1, 1, peer_id, ip, true, 1000);
        store.add_peer(info_hash, peer).unwrap();
        
        let (seeders, leechers) = store.get_stats(info_hash);
        assert_eq!(seeders, 1);
        assert_eq!(leechers, 0);
        
        store.remove_peer(info_hash, peer_id).unwrap();
        
        let (seeders, leechers) = store.get_stats(info_hash);
        assert_eq!(seeders, 0);
        assert_eq!(leechers, 0);
    }

    #[test]
    fn test_get_peers() {
        let store = PeerStore::new();
        let info_hash = [1u8; 20];
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        
        // Add 5 peers
        for i in 0..5 {
            let peer_id = [i; 20];
            let peer = create_test_peer(i as u32, 1, peer_id, ip, false, 1000);
            store.add_peer(info_hash, peer).unwrap();
        }
        
        // Request 3 peers, excluding peer 0
        let peers = store.get_peers(info_hash, 3, [0u8; 20]);
        assert_eq!(peers.len(), 3);
        
        // Verify excluded peer is not in the list
        assert!(!peers.iter().any(|p| p.peer_id == [0u8; 20]));
    }

    #[test]
    fn test_cleanup_stale_peers() {
        let store = PeerStore::new();
        let info_hash = [1u8; 20];
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        // Add 3 peers with different last_announce times
        // Peer 1: recent (should not be removed)
        let peer1 = create_test_peer(1, 1, [1u8; 20], ip, true, current_time - 100);
        store.add_peer(info_hash, peer1).unwrap();
        
        // Peer 2: stale (should be removed with 1000s timeout)
        let peer2 = create_test_peer(2, 1, [2u8; 20], ip, false, current_time - 2000);
        store.add_peer(info_hash, peer2).unwrap();
        
        // Peer 3: very stale (should be removed)
        let peer3 = create_test_peer(3, 1, [3u8; 20], ip, true, current_time - 5000);
        store.add_peer(info_hash, peer3).unwrap();
        
        let (seeders, leechers) = store.get_stats(info_hash);
        assert_eq!(seeders, 2);
        assert_eq!(leechers, 1);
        
        // Run cleanup with 1000 second timeout
        let removed = store.cleanup_stale_peers(1000);
        assert_eq!(removed, 2);
        
        // Check stats after cleanup
        let (seeders, leechers) = store.get_stats(info_hash);
        assert_eq!(seeders, 1);
        assert_eq!(leechers, 0);
        
        // Verify only peer 1 remains
        let peers = store.get_peers(info_hash, 10, [0u8; 20]);
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].peer_id, [1u8; 20]);
    }

    #[test]
    fn test_cleanup_no_stale_peers() {
        let store = PeerStore::new();
        let info_hash = [1u8; 20];
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        // Add recent peers
        let peer1 = create_test_peer(1, 1, [1u8; 20], ip, true, current_time - 100);
        store.add_peer(info_hash, peer1).unwrap();
        
        let peer2 = create_test_peer(2, 1, [2u8; 20], ip, false, current_time - 200);
        store.add_peer(info_hash, peer2).unwrap();
        
        // Run cleanup with 1000 second timeout
        let removed = store.cleanup_stale_peers(1000);
        assert_eq!(removed, 0);
        
        // Verify all peers remain
        let (seeders, leechers) = store.get_stats(info_hash);
        assert_eq!(seeders, 1);
        assert_eq!(leechers, 1);
    }

    #[test]
    fn test_user_ip_tracking() {
        let store = PeerStore::new();
        let info_hash = [1u8; 20];
        let ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));
        
        // Add two peers from same user with different IPs
        let peer1 = create_test_peer(1, 1, [1u8; 20], ip1, false, 1000);
        store.add_peer(info_hash, peer1).unwrap();
        
        let peer2 = create_test_peer(1, 1, [2u8; 20], ip2, false, 1000);
        store.add_peer(info_hash, peer2).unwrap();
        
        // Check IP count
        let ip_count = store.get_user_ip_count(1, 1);
        assert_eq!(ip_count, 2);
        
        // Remove one peer
        store.remove_peer(info_hash, [1u8; 20]).unwrap();
        
        // Check IP count again
        let ip_count = store.get_user_ip_count(1, 1);
        assert_eq!(ip_count, 1);
    }

    #[test]
    fn test_total_peers_and_active_torrents() {
        let store = PeerStore::new();
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        
        // Add peers to two different torrents
        let info_hash1 = [1u8; 20];
        let info_hash2 = [2u8; 20];
        
        for i in 0u8..3 {
            let peer = create_test_peer(i as u32, 1, [i; 20], ip, false, 1000);
            store.add_peer(info_hash1, peer).unwrap();
        }
        
        for i in 3u8..5 {
            let peer = create_test_peer(i as u32, 2, [i; 20], ip, true, 1000);
            store.add_peer(info_hash2, peer).unwrap();
        }
        
        assert_eq!(store.total_peers(), 5);
        assert_eq!(store.active_torrents(), 2);
    }
}
