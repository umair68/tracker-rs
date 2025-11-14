use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub memory: MemoryConfig,
    pub performance: PerformanceConfig,
    pub sync: SyncConfig,
    pub logging: LoggingConfig,
    pub anti_cheat: AntiCheatConfig,
    #[serde(default)]
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub port: Option<u16>,
    pub unix_socket: Option<PathBuf>,
    #[serde(default = "default_num_threads")]
    pub num_threads: usize,
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MemoryConfig {
    #[serde(default = "default_peer_capacity")]
    pub peer_capacity: usize,
    #[serde(default = "default_torrent_cache_size")]
    pub torrent_cache_size: usize,
    #[serde(default = "default_user_cache_size")]
    pub user_cache_size: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PerformanceConfig {
    #[serde(default = "default_min_announce_interval")]
    pub min_announce_interval: i64,
    #[serde(default = "default_max_requests_per_minute")]
    pub max_requests_per_minute: u32,
    #[serde(default = "default_cleanup_interval")]
    pub cleanup_interval: u64,
    #[serde(default = "default_peer_timeout")]
    pub peer_timeout: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SyncConfig {
    pub data_endpoint: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_format")]
    pub format: String,
    #[allow(dead_code)]
    pub path: Option<PathBuf>,
    #[serde(default = "default_console")]
    pub console: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AntiCheatConfig {
    #[serde(default = "default_max_ips_per_user")]
    pub max_ips_per_user: u32,
    #[serde(default = "default_max_ratio")]
    pub max_ratio: f64,
    #[serde(default = "default_max_upload_speed")]
    pub max_upload_speed: f64,
    #[serde(default = "default_max_download_speed")]
    pub max_download_speed: f64,
    #[serde(default = "default_min_seeder_upload")]
    pub min_seeder_upload: u64,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SecurityConfig {
    #[serde(default)]
    pub banned_ips: Vec<String>,
    #[serde(default)]
    pub banned_clients: Vec<String>,
}

// Default value functions
fn default_num_threads() -> usize {
    num_cpus::get()
}

fn default_max_connections() -> usize {
    10000
}

fn default_peer_capacity() -> usize {
    1_000_000
}

fn default_torrent_cache_size() -> usize {
    100_000
}

fn default_user_cache_size() -> usize {
    50_000
}

fn default_min_announce_interval() -> i64 {
    900 // 15 minutes
}

fn default_max_requests_per_minute() -> u32 {
    100
}

fn default_cleanup_interval() -> u64 {
    300 // 5 minutes
}

fn default_peer_timeout() -> i64 {
    3600 // 1 hour
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "json".to_string()
}

fn default_console() -> bool {
    false
}

fn default_max_ips_per_user() -> u32 {
    3
}

fn default_max_ratio() -> f64 {
    1000.0
}

fn default_max_upload_speed() -> f64 {
    1_073_741_824.0 // 1 GB/s
}

fn default_max_download_speed() -> f64 {
    1_073_741_824.0 // 1 GB/s
}

fn default_min_seeder_upload() -> u64 {
    1_048_576 // 1 MB
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .context(format!("Failed to read config file: {}", path.display()))?;
        
        let config: Config = toml::from_str(&content)
            .context("Failed to parse config file")?;
        
        config.validate()?;
        
        Ok(config)
    }

    /// Validate configuration values
    pub fn validate(&self) -> Result<()> {
        // Validate server config
        if self.server.port.is_none() && self.server.unix_socket.is_none() {
            bail!("Either port or unix_socket must be specified in server config");
        }
        
        if let Some(port) = self.server.port {
            if port == 0 {
                bail!("Server port must be greater than 0");
            }
        }
        
        if self.server.num_threads == 0 {
            bail!("num_threads must be greater than 0");
        }
        
        if self.server.max_connections == 0 {
            bail!("max_connections must be greater than 0");
        }
        
        // Validate memory config
        if self.memory.peer_capacity == 0 {
            bail!("peer_capacity must be greater than 0");
        }
        
        if self.memory.torrent_cache_size == 0 {
            bail!("torrent_cache_size must be greater than 0");
        }
        
        if self.memory.user_cache_size == 0 {
            bail!("user_cache_size must be greater than 0");
        }
        
        // Validate performance config
        if self.performance.min_announce_interval < 0 {
            bail!("min_announce_interval must be non-negative");
        }
        
        if self.performance.max_requests_per_minute == 0 {
            bail!("max_requests_per_minute must be greater than 0");
        }
        
        if self.performance.cleanup_interval == 0 {
            bail!("cleanup_interval must be greater than 0");
        }

        if self.performance.peer_timeout < 0 {
            bail!("peer_timeout must be non-negative");
        }

        // Validate that peer_timeout is greater than cleanup_interval
        if self.performance.peer_timeout <= self.performance.cleanup_interval as i64 {
            bail!(
                "peer_timeout ({}) must be greater than cleanup_interval ({})",
                self.performance.peer_timeout,
                self.performance.cleanup_interval
            );
        }

        // Validate sync config
        if self.sync.data_endpoint.is_empty() {
            bail!("data_endpoint must not be empty");
        }
        
        if self.sync.api_key.is_empty() {
            bail!("api_key must not be empty");
        }
        
        // Validate logging config
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            bail!(
                "Invalid log level '{}'. Must be one of: trace, debug, info, warn, error",
                self.logging.level
            );
        }
        
        let valid_formats = ["json", "console"];
        if !valid_formats.contains(&self.logging.format.as_str()) {
            bail!(
                "Invalid log format '{}'. Must be one of: json, console",
                self.logging.format
            );
        }
        
        // Validate anti-cheat config
        if self.anti_cheat.max_ips_per_user == 0 {
            bail!("max_ips_per_user must be greater than 0");
        }
        
        if self.anti_cheat.max_ratio <= 0.0 {
            bail!("max_ratio must be greater than 0");
        }
        
        if self.anti_cheat.max_upload_speed <= 0.0 {
            bail!("max_upload_speed must be greater than 0");
        }
        
        if self.anti_cheat.max_download_speed <= 0.0 {
            bail!("max_download_speed must be greater than 0");
        }
        
        if self.anti_cheat.min_seeder_upload == 0 {
            bail!("min_seeder_upload must be greater than 0");
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    #[test]
    #[ignore]
    fn test_load_config_with_security() {
        let path = PathBuf::from("config.security.toml");
        let config = Config::from_file(&path).expect("Failed to load config");
        
        assert_eq!(config.security.banned_ips.len(), 4);
        assert!(config.security.banned_ips.contains(&"192.168.1.100".to_string()));
        assert!(config.security.banned_ips.contains(&"10.0.0.50".to_string()));
        assert!(config.security.banned_ips.contains(&"2001:db8::1".to_string()));
        assert!(config.security.banned_ips.contains(&"2001:db8::bad:cafe".to_string()));
        
        assert_eq!(config.security.banned_clients.len(), 2);
        assert!(config.security.banned_clients.contains(&"BadClient/1.0".to_string()));
        assert!(config.security.banned_clients.contains(&"SpamBot".to_string()));
    }

    #[test]
    #[ignore]
    fn test_parse_banned_ips() {
        let path = PathBuf::from("config.security.toml");
        let config = Config::from_file(&path).expect("Failed to load config");
        
        // Verify all IPs can be parsed
        for ip_str in &config.security.banned_ips {
            let parsed: Result<IpAddr, _> = ip_str.parse();
            assert!(parsed.is_ok(), "Failed to parse IP: {}", ip_str);
        }
    }

    #[test]
    fn test_config_with_empty_security() {
        let path = PathBuf::from("config.toml");
        let config = Config::from_file(&path).expect("Failed to load config");
        
        assert_eq!(config.security.banned_ips.len(), 0);
        assert_eq!(config.security.banned_clients.len(), 0);
    }

    #[test]
    fn test_security_config_default() {
        let security = SecurityConfig::default();
        assert!(security.banned_ips.is_empty());
        assert!(security.banned_clients.is_empty());
    }

    #[test]
    #[ignore]
    fn test_ipv4_and_ipv6_mixed() {
        let path = PathBuf::from("config.security.toml");
        let config = Config::from_file(&path).expect("Failed to load config");
        
        let mut ipv4_count = 0;
        let mut ipv6_count = 0;
        
        for ip_str in &config.security.banned_ips {
            match ip_str.parse::<IpAddr>().unwrap() {
                IpAddr::V4(_) => ipv4_count += 1,
                IpAddr::V6(_) => ipv6_count += 1,
            }
        }
        
        assert_eq!(ipv4_count, 2, "Expected 2 IPv4 addresses");
        assert_eq!(ipv6_count, 2, "Expected 2 IPv6 addresses");
    }
}
