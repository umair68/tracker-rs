use crate::core::error::AntiCheatError;
use tracing::warn;
pub fn check_ghost_seeder(
    user_id: u32,
    torrent_id: u32,
    is_seeder: bool,
    uploaded: u64,
    min_upload: u64,
    is_completed_event: bool,
) -> Result<(), AntiCheatError> {
    // Skip check if not a seeder
    if !is_seeder {
        return Ok(());
    }
    
    // Skip check if this is a completed event (peer just finished downloading)
    if is_completed_event {
        return Ok(());
    }
    
    // Check if uploaded amount is suspiciously low
    if uploaded < min_upload {
        warn!(
            user_id = user_id,
            torrent_id = torrent_id,
            uploaded = uploaded,
            min_upload = min_upload,
            severity = "medium",
            "Ghost seeder detected: seeder has uploaded less than minimum threshold"
        );
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ghost_seeder_normal_seeder() {
        // Seeder with 10 MB uploaded (above 1 MB threshold)
        let result = check_ghost_seeder(
            1,
            1,
            true,
            10_000_000,
            1_048_576, // 1 MB
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_ghost_seeder_low_upload() {
        // Seeder with only 100 KB uploaded (below 1 MB threshold)
        // This should log a warning but not fail
        let result = check_ghost_seeder(
            1,
            1,
            true,
            100_000,
            1_048_576, // 1 MB
            false,
        );
        // Ghost seeder check logs warnings but doesn't fail
        assert!(result.is_ok());
    }

    #[test]
    fn test_ghost_seeder_not_seeder() {
        // Leecher should skip the check
        let result = check_ghost_seeder(
            1,
            1,
            false,
            100_000,
            1_048_576,
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_ghost_seeder_completed_event() {
        // Seeder with low upload but completed event should skip check
        let result = check_ghost_seeder(
            1,
            1,
            true,
            100_000,
            1_048_576,
            true, // completed event
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_ghost_seeder_zero_upload() {
        // Seeder with zero upload (suspicious)
        let result = check_ghost_seeder(
            1,
            1,
            true,
            0,
            1_048_576,
            false,
        );
        // Should log warning but not fail
        assert!(result.is_ok());
    }

    #[test]
    fn test_ghost_seeder_exactly_at_threshold() {
        // Seeder with exactly the minimum upload
        let result = check_ghost_seeder(
            1,
            1,
            true,
            1_048_576,
            1_048_576,
            false,
        );
        assert!(result.is_ok());
    }
}
