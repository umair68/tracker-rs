use crate::core::error::AntiCheatError;
use tracing::warn;

pub fn check_announce_interval(
    user_id: u32,
    torrent_id: u32,
    last_announce: Option<i64>,
    current_time: i64,
    min_interval: i64,
) -> Result<(), AntiCheatError> {
    // Skip check if this is the first announce
    let Some(last_announce_time) = last_announce else {
        return Ok(());
    };
    
    // Calculate elapsed time since last announce
    let elapsed = current_time - last_announce_time;
    
    // Check if announce interval is too short
    if elapsed < min_interval {
        warn!(
            user_id = user_id,
            torrent_id = torrent_id,
            elapsed_seconds = elapsed,
            min_interval = min_interval,
            severity = "medium",
            "Announce interval too short: peer announced before minimum interval elapsed"
        );
        
        return Err(AntiCheatError::AnnounceIntervalTooShort {
            elapsed,
            min_interval,
        });
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_announce_interval_first_announce() {
        // First announce (no previous announce) should pass
        let result = check_announce_interval(
            1,
            1,
            None,
            1000,
            900, // 15 minutes
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_announce_interval_sufficient_time() {
        // 1000 seconds elapsed, min interval 900 seconds
        let result = check_announce_interval(
            1,
            1,
            Some(1000),
            2000,
            900,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_announce_interval_too_short() {
        // 500 seconds elapsed, min interval 900 seconds
        let result = check_announce_interval(
            1,
            1,
            Some(1000),
            1500,
            900,
        );
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Announce interval too short"));
    }

    #[test]
    fn test_announce_interval_exactly_at_minimum() {
        // Exactly at minimum interval should pass
        let result = check_announce_interval(
            1,
            1,
            Some(1000),
            1900,
            900,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_announce_interval_zero_elapsed() {
        // Zero elapsed time (same timestamp) should fail
        let result = check_announce_interval(
            1,
            1,
            Some(1000),
            1000,
            900,
        );
        assert!(result.is_err());
    }
}
