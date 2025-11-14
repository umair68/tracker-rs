use crate::core::error::AntiCheatError;
use crate::stores::peer_store::PeerStore;
use tracing::warn;

pub fn check_duplicate_peer(
    peer_store: &PeerStore,
    user_id: u32,
    torrent_id: u32,
    max_ips: u32,
) -> Result<(), AntiCheatError> {
    let ip_count = peer_store.get_user_ip_count(user_id, torrent_id);
    
    if ip_count > max_ips as usize {
        warn!(
            user_id = user_id,
            torrent_id = torrent_id,
            ip_count = ip_count,
            max_ips = max_ips,
            severity = "high",
            "Duplicate peer violation detected: user exceeds maximum IP addresses per torrent"
        );
        
        return Err(AntiCheatError::TooManyIps {
            count: ip_count,
            max: max_ips,
        });
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::peer::Peer;
    use std::net::{IpAddr, Ipv4Addr};

    fn create_test_peer(
        user_id: u32,
        torrent_id: u32,
        peer_id: [u8; 20],
        ip: IpAddr,
    ) -> Peer {
        Peer::new(
            user_id,
            torrent_id,
            peer_id,
            ip,
            6881,
            1024,
            512,
            1000,
            1000,
            "TestClient/1.0".to_string(),
        )
    }

    #[test]
    fn test_duplicate_peer_within_limit() {
        let store = PeerStore::new();
        let info_hash = [1u8; 20];
        
        // Add 2 peers from same user with different IPs
        let ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));
        
        let peer1 = create_test_peer(1, 1, [1u8; 20], ip1);
        let peer2 = create_test_peer(1, 1, [2u8; 20], ip2);
        
        store.add_peer(info_hash, peer1).unwrap();
        store.add_peer(info_hash, peer2).unwrap();
        
        // Should pass with max_ips = 3
        let result = check_duplicate_peer(&store, 1, 1, 3);
        assert!(result.is_ok());
    }

    #[test]
    fn test_duplicate_peer_exceeds_limit() {
        let store = PeerStore::new();
        let info_hash = [1u8; 20];
        
        // Add 4 peers from same user with different IPs
        for i in 0..4 {
            let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, i + 1));
            let peer = create_test_peer(1, 1, [i; 20], ip);
            store.add_peer(info_hash, peer).unwrap();
        }
        
        // Should fail with max_ips = 3
        let result = check_duplicate_peer(&store, 1, 1, 3);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Too many IPs"));
    }

    #[test]
    fn test_duplicate_peer_different_users() {
        let store = PeerStore::new();
        let info_hash = [1u8; 20];
        
        // Add peers from different users
        let ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));
        
        let peer1 = create_test_peer(1, 1, [1u8; 20], ip1);
        let peer2 = create_test_peer(2, 1, [2u8; 20], ip2);
        
        store.add_peer(info_hash, peer1).unwrap();
        store.add_peer(info_hash, peer2).unwrap();
        
        // Each user has only 1 IP, should pass
        let result1 = check_duplicate_peer(&store, 1, 1, 1);
        let result2 = check_duplicate_peer(&store, 2, 1, 1);
        
        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }
}
