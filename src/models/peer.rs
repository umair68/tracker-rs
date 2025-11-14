use std::net::IpAddr;

/// Represents an active peer in the tracker
#[derive(Clone, Debug)]
pub struct Peer {
    /// User ID from the user cache
    pub user_id: u32,
    /// Torrent ID from the torrent cache
    pub torrent_id: u32,
    /// 20-byte peer identifier
    pub peer_id: [u8; 20],
    /// IP address (IPv4 or IPv6)
    pub ip: IpAddr,
    /// Port number
    pub port: u16,
    /// Total bytes uploaded
    pub uploaded: u64,
    /// Total bytes downloaded
    pub downloaded: u64,
    /// Bytes left to download (0 for seeders)
    pub left: u64,
    /// Unix timestamp of last announce
    pub last_announce: i64,
    /// User-Agent string from HTTP header
    pub user_agent: String,
    /// Whether this peer is a seeder (left == 0)
    pub is_seeder: bool,
}

impl Peer {
    pub fn new(
        user_id: u32,
        torrent_id: u32,
        peer_id: [u8; 20],
        ip: IpAddr,
        port: u16,
        uploaded: u64,
        downloaded: u64,
        left: u64,
        last_announce: i64,
        user_agent: String,
    ) -> Self {
        Self {
            user_id,
            torrent_id,
            peer_id,
            ip,
            port,
            uploaded,
            downloaded,
            left,
            last_announce,
            user_agent,
            is_seeder: left == 0,
        }
    }
}
