use std::time::{SystemTime, UNIX_EPOCH};

pub fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time is before Unix epoch")
        .as_secs() as i64
}

pub fn current_timestamp_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time is before Unix epoch")
        .as_millis() as i64
}

pub fn elapsed_seconds(start: i64, end: i64) -> i64 {
    end - start
}


pub fn is_expired(timestamp: i64, timeout: i64, current_time: i64) -> bool {
    elapsed_seconds(timestamp, current_time) > timeout
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_timestamp() {
        let ts = current_timestamp();
        // Should be a reasonable timestamp (after 2020-01-01)
        assert!(ts > 1577836800);
        // Should be before 2100-01-01
        assert!(ts < 4102444800);
    }

    #[test]
    fn test_current_timestamp_millis() {
        let ts_millis = current_timestamp_millis();
        let ts_secs = current_timestamp();
        
        // Milliseconds should be roughly 1000x seconds
        let diff = (ts_millis / 1000 - ts_secs).abs();
        assert!(diff <= 1); // Allow 1 second difference due to timing
    }

    #[test]
    fn test_elapsed_seconds() {
        assert_eq!(elapsed_seconds(100, 150), 50);
        assert_eq!(elapsed_seconds(1000, 1000), 0);
        assert_eq!(elapsed_seconds(200, 100), -100);
    }

    #[test]
    fn test_is_expired() {
        let current = 1000;
        
        // Not expired: timestamp is recent
        assert!(!is_expired(950, 100, current));
        
        // Expired: timestamp is old
        assert!(is_expired(800, 100, current));
        
        // Edge case: exactly at timeout
        assert!(!is_expired(900, 100, current));
        
        // Edge case: just over timeout
        assert!(is_expired(899, 100, current));
    }

    #[test]
    fn test_is_expired_peer_timeout() {
        // Simulate peer timeout scenario
        let peer_timeout = 3600; // 1 hour
        let current_time = current_timestamp();
        
        // Peer announced 30 minutes ago - not expired
        let recent_announce = current_time - 1800;
        assert!(!is_expired(recent_announce, peer_timeout, current_time));
        
        // Peer announced 2 hours ago - expired
        let old_announce = current_time - 7200;
        assert!(is_expired(old_announce, peer_timeout, current_time));
    }
}
