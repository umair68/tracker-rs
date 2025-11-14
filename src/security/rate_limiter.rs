use dashmap::DashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicI64, AtomicU32, Ordering};


pub struct RateLimiter {
    requests: DashMap<IpAddr, (AtomicU32, AtomicI64)>,
    max_requests_per_minute: u32,
}

impl RateLimiter {
    pub fn new(max_requests_per_minute: u32) -> Self {
        Self {
            requests: DashMap::new(),
            max_requests_per_minute,
        }
    }

    pub fn check_and_increment(&self, ip: IpAddr, current_time: i64) -> bool {
        let entry = self.requests.entry(ip).or_insert_with(|| {
            (AtomicU32::new(0), AtomicI64::new(current_time))
        });

        let (count, window_start) = entry.value();
        let window_start_time = window_start.load(Ordering::Relaxed);
        
        if current_time - window_start_time >= 60 {
            window_start.store(current_time, Ordering::Relaxed);
            count.store(1, Ordering::Relaxed);
            return true;
        }

        let current_count = count.fetch_add(1, Ordering::Relaxed) + 1;

        current_count <= self.max_requests_per_minute
    }

    pub fn cleanup_old_entries(&self, current_time: i64) {
        self.requests.retain(|_, (_, window_start)| {
            current_time - window_start.load(Ordering::Relaxed) < 60
        });
    }


    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.requests.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_rate_limiter_allows_first_request() {
        let limiter = RateLimiter::new(10);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let current_time = 1000;

        assert!(limiter.check_and_increment(ip, current_time));
    }

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::new(5);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let current_time = 1000;

        // First 5 requests should be allowed
        for _ in 0..5 {
            assert!(limiter.check_and_increment(ip, current_time));
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(5);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let current_time = 1000;

        // First 5 requests should be allowed
        for _ in 0..5 {
            assert!(limiter.check_and_increment(ip, current_time));
        }

        // 6th request should be blocked
        assert!(!limiter.check_and_increment(ip, current_time));
    }

    #[test]
    fn test_rate_limiter_resets_after_window() {
        let limiter = RateLimiter::new(5);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let current_time = 1000;

        // Use up the limit
        for _ in 0..5 {
            assert!(limiter.check_and_increment(ip, current_time));
        }

        // Should be blocked
        assert!(!limiter.check_and_increment(ip, current_time));

        // After 60 seconds, window should reset
        let new_time = current_time + 60;
        assert!(limiter.check_and_increment(ip, new_time));
    }

    #[test]
    fn test_rate_limiter_different_ips() {
        let limiter = RateLimiter::new(5);
        let ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));
        let current_time = 1000;

        // Use up limit for ip1
        for _ in 0..5 {
            assert!(limiter.check_and_increment(ip1, current_time));
        }
        assert!(!limiter.check_and_increment(ip1, current_time));

        // ip2 should still be allowed
        assert!(limiter.check_and_increment(ip2, current_time));
    }

    #[test]
    fn test_rate_limiter_ipv6() {
        let limiter = RateLimiter::new(10);
        let ip = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        let current_time = 1000;

        assert!(limiter.check_and_increment(ip, current_time));
    }

    #[test]
    fn test_cleanup_old_entries() {
        let limiter = RateLimiter::new(10);
        let ip1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let ip2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));
        let current_time = 1000;

        // Add entries for both IPs
        limiter.check_and_increment(ip1, current_time);
        limiter.check_and_increment(ip2, current_time + 30);

        assert_eq!(limiter.len(), 2);

        // Cleanup at time that makes ip1 old but ip2 recent
        limiter.cleanup_old_entries(current_time + 70);

        // ip1 should be removed (70 seconds old), ip2 should remain (40 seconds old)
        assert_eq!(limiter.len(), 1);
    }

    #[test]
    fn test_cleanup_removes_all_old_entries() {
        let limiter = RateLimiter::new(10);
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let current_time = 1000;

        limiter.check_and_increment(ip, current_time);
        assert_eq!(limiter.len(), 1);

        // Cleanup after window expires
        limiter.cleanup_old_entries(current_time + 100);
        assert_eq!(limiter.len(), 0);
    }

    #[test]
    fn test_atomic_operations_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let limiter = Arc::new(RateLimiter::new(100));
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let current_time = 1000;

        let mut handles = vec![];

        // Spawn multiple threads trying to increment
        for _ in 0..10 {
            let limiter_clone = Arc::clone(&limiter);
            let handle = thread::spawn(move || {
                for _ in 0..10 {
                    limiter_clone.check_and_increment(ip, current_time);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // All 100 requests should have been counted
        // The 101st request should fail
        assert!(!limiter.check_and_increment(ip, current_time));
    }

    #[test]
    fn test_is_empty() {
        let limiter = RateLimiter::new(10);
        assert!(limiter.is_empty());

        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        limiter.check_and_increment(ip, 1000);
        assert!(!limiter.is_empty());
    }
}
