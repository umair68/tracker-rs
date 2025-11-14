use crate::core::error::AntiCheatError;
use tracing::warn;

/// Check for extreme upload/download ratios
pub fn check_ratio(
    user_id: u32,
    torrent_id: u32,
    uploaded: u64,
    downloaded: u64,
    max_ratio: f64,
) -> Result<(), AntiCheatError> {
    if downloaded == 0 {
        return Ok(());
    }
    
    // Calculate ratio (uploaded / downloaded)
    let ratio = uploaded as f64 / downloaded as f64;
    
    // Check if ratio exceeds maximum
    if ratio > max_ratio {
        warn!(
            user_id = user_id,
            torrent_id = torrent_id,
            ratio = ratio,
            max_ratio = max_ratio,
            uploaded = uploaded,
            downloaded = downloaded,
            severity = "medium",
            "Suspicious ratio detected: upload/download ratio exceeds maximum realistic ratio"
        );
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ratio_check_normal_ratio() {
        // Ratio of 2.0 (uploaded 2x downloaded)
        let result = check_ratio(
            1,
            1,
            2_000_000,
            1_000_000,
            1000.0, // max ratio
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_ratio_check_excessive_ratio() {
        // Ratio of 2000.0 (uploaded 2000x downloaded, exceeds 1000.0 limit)
        // This should log a warning but not fail
        let result = check_ratio(
            1,
            1,
            2_000_000_000,
            1_000_000,
            1000.0, // max ratio
        );
        // Ratio check logs warnings but doesn't fail
        assert!(result.is_ok());
    }

    #[test]
    fn test_ratio_check_zero_downloaded() {
        // Zero downloaded should skip the check
        let result = check_ratio(
            1,
            1,
            1_000_000,
            0,
            1000.0,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_ratio_check_equal_amounts() {
        // Ratio of 1.0 (equal upload and download)
        let result = check_ratio(
            1,
            1,
            1_000_000,
            1_000_000,
            1000.0,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_ratio_check_low_ratio() {
        // Ratio of 0.5 (downloaded more than uploaded)
        let result = check_ratio(
            1,
            1,
            500_000,
            1_000_000,
            1000.0,
        );
        assert!(result.is_ok());
    }
}
