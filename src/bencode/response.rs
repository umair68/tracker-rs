use crate::models::peer::Peer;
use std::net::IpAddr;

use super::encoder::BencodeEncode;

/// Build a bencode-encoded announce response
///
/// # Arguments
/// * `peers` - List of peers to include in response
/// * `seeders` - Total number of seeders for this torrent
/// * `leechers` - Total number of leechers for this torrent
/// * `compact` - Whether to use compact format (true) or dictionary format (false)
///
/// # Returns
/// A bencode-encoded response as bytes
pub fn build_announce_response(
    peers: &[Peer],
    seeders: u32,
    leechers: u32,
    compact: bool,
) -> Vec<u8> {
    let capacity = if compact {
        100 + (peers.len() * 6)
    } else {
        100 + (peers.len() * 50)
    };
    let mut buf = Vec::with_capacity(capacity);

    buf.extend_from_slice(b"d");

    "complete".bencode(&mut buf);
    (seeders as i64).bencode(&mut buf);

    "incomplete".bencode(&mut buf);
    (leechers as i64).bencode(&mut buf);

    "interval".bencode(&mut buf);
    1800i64.bencode(&mut buf);

    "min interval".bencode(&mut buf);
    900i64.bencode(&mut buf);

    if compact {
        "peers".bencode(&mut buf);
        encode_compact_peers(peers, &mut buf);

        "peers6".bencode(&mut buf);
        encode_compact_peers_ipv6(peers, &mut buf);
    } else {
        "peers".bencode(&mut buf);
        encode_dict_peers(peers, &mut buf);
    }

    buf.extend_from_slice(b"e");

    buf
}

/// Encode IPv4 peers in compact format (6 bytes per peer: 4 for IP, 2 for port)
fn encode_compact_peers(peers: &[Peer], buf: &mut Vec<u8>) {
    let ipv4_count = peers.iter().filter(|p| matches!(p.ip, IpAddr::V4(_))).count();

    if ipv4_count == 0 {
        buf.extend_from_slice(b"0:");
        return;
    }

    let peer_bytes = ipv4_count * 6;

    let mut itoa_buf = itoa::Buffer::new();
    buf.extend_from_slice(itoa_buf.format(peer_bytes).as_bytes());
    buf.extend_from_slice(b":");

    buf.reserve(peer_bytes);

    for peer in peers.iter().filter(|p| matches!(p.ip, IpAddr::V4(_))) {
        if let IpAddr::V4(ip) = peer.ip {
            buf.extend_from_slice(&ip.octets());
            buf.extend_from_slice(&peer.port.to_be_bytes());
        }
    }
}

fn encode_compact_peers_ipv6(peers: &[Peer], buf: &mut Vec<u8>) {
    let ipv6_count = peers.iter().filter(|p| matches!(p.ip, IpAddr::V6(_))).count();

    if ipv6_count == 0 {
        buf.extend_from_slice(b"0:");
        return;
    }

    let peer_bytes = ipv6_count * 18;

    let mut itoa_buf = itoa::Buffer::new();
    buf.extend_from_slice(itoa_buf.format(peer_bytes).as_bytes());
    buf.extend_from_slice(b":");

    buf.reserve(peer_bytes);

    for peer in peers.iter().filter(|p| matches!(p.ip, IpAddr::V6(_))) {
        if let IpAddr::V6(ip) = peer.ip {
            buf.extend_from_slice(&ip.octets());
            buf.extend_from_slice(&peer.port.to_be_bytes());
        }
    }
}

fn encode_dict_peers(peers: &[Peer], buf: &mut Vec<u8>) {
    buf.extend_from_slice(b"l");

    for peer in peers {
        buf.extend_from_slice(b"d");

        "ip".bencode(buf);
        peer.ip.to_string().as_str().bencode(buf);

        "peer id".bencode(buf);
        peer.peer_id.as_slice().bencode(buf);

        "port".bencode(buf);
        (peer.port as i64).bencode(buf);

        buf.extend_from_slice(b"e");
    }

    buf.extend_from_slice(b"e");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    fn create_test_peer_ipv4(ip: Ipv4Addr, port: u16) -> Peer {
        Peer::new(
            1,
            1,
            [0u8; 20],
            IpAddr::V4(ip),
            port,
            0,
            0,
            0,
            0,
            String::new(),
        )
    }

    fn create_test_peer_ipv6(ip: Ipv6Addr, port: u16) -> Peer {
        Peer::new(
            1,
            1,
            [0u8; 20],
            IpAddr::V6(ip),
            port,
            0,
            0,
            0,
            0,
            String::new(),
        )
    }

    #[test]
    fn test_build_announce_response_compact() {
        let peers = vec![
            create_test_peer_ipv4(Ipv4Addr::new(192, 168, 1, 1), 6881),
            create_test_peer_ipv4(Ipv4Addr::new(10, 0, 0, 1), 51413),
        ];

        let response = build_announce_response(&peers, 5, 3, true);
        let response_str = String::from_utf8_lossy(&response);

        // Check that response is a valid bencode dictionary
        assert!(response_str.starts_with('d'));
        assert!(response_str.ends_with('e'));

        // Check for required keys
        assert!(response_str.contains("complete"));
        assert!(response_str.contains("incomplete"));
        assert!(response_str.contains("interval"));
        assert!(response_str.contains("min interval"));
        assert!(response_str.contains("peers"));
        assert!(response_str.contains("peers6"));
    }

    #[test]
    fn test_build_announce_response_dict() {
        let peers = vec![create_test_peer_ipv4(Ipv4Addr::new(192, 168, 1, 1), 6881)];

        let response = build_announce_response(&peers, 5, 3, false);
        let response_str = String::from_utf8_lossy(&response);

        // Check that response is a valid bencode dictionary
        assert!(response_str.starts_with('d'));
        assert!(response_str.ends_with('e'));

        // Check for required keys
        assert!(response_str.contains("peers"));
        assert!(response_str.contains("ip"));
        assert!(response_str.contains("port"));
        assert!(response_str.contains("peer id"));
    }

    #[test]
    fn test_encode_compact_peers_ipv4() {
        let peers = vec![
            create_test_peer_ipv4(Ipv4Addr::new(192, 168, 1, 1), 6881),
            create_test_peer_ipv4(Ipv4Addr::new(10, 0, 0, 1), 51413),
        ];

        let mut buf = Vec::new();
        encode_compact_peers(&peers, &mut buf);

        // Should be: "12:" followed by 12 bytes (2 peers * 6 bytes each)
        assert_eq!(&buf[0..3], b"12:");
        assert_eq!(buf.len(), 3 + 12); // "12:" + 12 bytes

        // First peer: 192.168.1.1:6881
        assert_eq!(&buf[3..7], &[192, 168, 1, 1]);
        assert_eq!(&buf[7..9], &6881u16.to_be_bytes());

        // Second peer: 10.0.0.1:51413
        assert_eq!(&buf[9..13], &[10, 0, 0, 1]);
        assert_eq!(&buf[13..15], &51413u16.to_be_bytes());
    }

    #[test]
    fn test_encode_compact_peers_ipv6() {
        let peers = vec![create_test_peer_ipv6(
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
            6881,
        )];

        let mut buf = Vec::new();
        encode_compact_peers_ipv6(&peers, &mut buf);

        // Should be: "18:" followed by 18 bytes (1 peer * 18 bytes)
        assert_eq!(&buf[0..3], b"18:");
        assert_eq!(buf.len(), 3 + 18); // "18:" + 18 bytes

        // Check IP address (16 bytes)
        let expected_ip = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1).octets();
        assert_eq!(&buf[3..19], &expected_ip);

        // Check port (2 bytes)
        assert_eq!(&buf[19..21], &6881u16.to_be_bytes());
    }

    #[test]
    fn test_encode_compact_peers_empty() {
        let peers: Vec<Peer> = vec![];

        let mut buf = Vec::new();
        encode_compact_peers(&peers, &mut buf);

        // Should be empty byte string
        assert_eq!(buf, b"0:");
    }

    #[test]
    fn test_encode_compact_peers_mixed() {
        let peers = vec![
            create_test_peer_ipv4(Ipv4Addr::new(192, 168, 1, 1), 6881),
            create_test_peer_ipv6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1), 6882),
        ];

        // IPv4 encoding should only include IPv4 peers
        let mut buf = Vec::new();
        encode_compact_peers(&peers, &mut buf);
        assert_eq!(&buf[0..2], b"6:"); // Only 1 IPv4 peer = 6 bytes

        // IPv6 encoding should only include IPv6 peers
        let mut buf = Vec::new();
        encode_compact_peers_ipv6(&peers, &mut buf);
        assert_eq!(&buf[0..3], b"18:"); // Only 1 IPv6 peer = 18 bytes
    }

    #[test]
    fn test_encode_dict_peers() {
        let mut peer = create_test_peer_ipv4(Ipv4Addr::new(192, 168, 1, 1), 6881);
        peer.peer_id = [1u8; 20];

        let peers = vec![peer];

        let mut buf = Vec::new();
        encode_dict_peers(&peers, &mut buf);

        let result = String::from_utf8_lossy(&buf);

        // Should be a list containing a dictionary
        assert!(result.starts_with('l'));
        assert!(result.ends_with('e'));
        assert!(result.contains("d")); // Dictionary start
        assert!(result.contains("2:ip"));
        assert!(result.contains("7:peer id"));
        assert!(result.contains("4:port"));
    }
}
