use crate::core::error::AntiCheatError;
use tracing::warn;

pub fn check_speed(
    user_id: u32,
    torrent_id: u32,
    old_uploaded: u64,
    new_uploaded: u64,
    old_downloaded: u64,
    new_downloaded: u64,
    elapsed: i64,
    max_speed: f64,
) -> Result<(), AntiCheatError> {
    // Skip check if this is the first announce (elapsed would be 0 or very small)
    if elapsed <= 0 {
        return Ok(());
    }
    
    let elapsed_f64 = elapsed as f64;
    
    // Calculate upload speed (bytes per second)
    let upload_delta = new_uploaded.saturating_sub(old_uploaded);
    let upload_speed = upload_delta as f64 / elapsed_f64;
    
    // Calculate download speed (bytes per second)
    let download_delta = new_downloaded.saturating_sub(old_downloaded);
    let download_speed = download_delta as f64 / elapsed_f64;
    
    // Check upload speed
    if upload_speed > max_speed {
        let speed_mbps = upload_speed / 1_000_000.0;
        let max_mbps = max_speed / 1_000_000.0;
        
        warn!(
            user_id = user_id,
            torrent_id = torrent_id,
            upload_speed_mbps = speed_mbps,
            max_speed_mbps = max_mbps,
            elapsed_seconds = elapsed,
            upload_delta = upload_delta,
            severity = "high",
            "Suspicious upload speed detected: exceeds maximum realistic speed"
        );
    }
    
    // Check download speed
    if download_speed > max_speed {
        let speed_mbps = download_speed / 1_000_000.0;
        let max_mbps = max_speed / 1_000_000.0;
        
        warn!(
            user_id = user_id,
            torrent_id = torrent_id,
            download_speed_mbps = speed_mbps,
            max_speed_mbps = max_mbps,
            elapsed_seconds = elapsed,
            download_delta = download_delta,
            severity = "high",
            "Suspicious download speed detected: exceeds maximum realistic speed"
        );
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_speed_check_normal_speed() {
        // 10 MB uploaded in 10 seconds = 1 MB/s
        let result = check_speed(
            1,
            1,
            0,
            10_000_000,
            0,
            5_000_000,
            10,
            100_000_000.0, // 100 MB/s max
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_speed_check_excessive_upload() {
        // 1 GB uploaded in 1 second = 1 GB/s (exceeds 100 MB/s limit)
        // This should log a warning but not fail
        let result = check_speed(
            1,
            1,
            0,
            1_000_000_000,
            0,
            0,
            1,
            100_000_000.0, // 100 MB/s max
        );
        // Speed check logs warnings but doesn't fail
        assert!(result.is_ok());
    }

    #[test]
    fn test_speed_check_excessive_download() {
        // 1 GB downloaded in 1 second = 1 GB/s (exceeds 100 MB/s limit)
        let result = check_speed(
            1,
            1,
            0,
            0,
            0,
            1_000_000_000,
            1,
            100_000_000.0, // 100 MB/s max
        );
        // Speed check logs warnings but doesn't fail
        assert!(result.is_ok());
    }

    #[test]
    fn test_speed_check_first_announce() {
        // First announce (elapsed = 0) should be skipped
        let result = check_speed(
            1,
            1,
            0,
            1_000_000_000,
            0,
            1_000_000_000,
            0,
            100_000_000.0,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_speed_check_no_change() {
        // No upload or download change
        let result = check_speed(
            1,
            1,
            1000,
            1000,
            500,
            500,
            10,
            100_000_000.0,
        );
        assert!(result.is_ok());
    }
}
