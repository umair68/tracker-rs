use crate::utils::hex::url_decode;
use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use std::net::IpAddr;

/// Query parameters for announce requests
#[derive(Debug, Deserialize)]
pub struct AnnounceParams {
    /// User's 32-character hexadecimal passkey
    pub passkey: String,
    
    /// URL-encoded 20-byte info_hash
    pub info_hash: String,
    
    /// URL-encoded 20-byte peer_id
    pub peer_id: String,
    
    /// Port number (1-65535)
    pub port: u16,
    
    /// Bytes uploaded
    pub uploaded: u64,
    
    /// Bytes downloaded
    pub downloaded: u64,
    
    /// Bytes left to download
    pub left: u64,
    
    /// Event: "started", "stopped", "completed", or empty
    #[serde(default)]
    pub event: String,
    
    /// Number of peers wanted (0-200, default 50)
    #[serde(default = "default_numwant")]
    pub numwant: u32,
    
    /// Compact mode (0 or 1, default 1)
    #[serde(default = "default_compact")]
    pub compact: u8,
    
    /// Optional IP address override
    pub ip: Option<String>,
}

fn default_numwant() -> u32 {
    50
}

fn default_compact() -> u8 {
    1
}

#[derive(Debug)]
pub struct ValidatedAnnounceParams {
    pub passkey: [u8; 32],
    pub info_hash: [u8; 20],
    pub peer_id: [u8; 20],
    pub port: u16,
    pub uploaded: u64,
    pub downloaded: u64,
    pub left: u64,
    pub event: Option<AnnounceEvent>,
    pub numwant: u32,
    pub compact: bool,
    pub ip: Option<IpAddr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnounceEvent {
    Started,
    Stopped,
    Completed,
}

impl AnnounceParams {
    /// Check if request has suspicious headers that indicate it's not a real torrent client
    /// This should be called by the handler with the actual HTTP headers
    pub fn has_suspicious_headers(headers: &[(String, String)]) -> bool {
        // Real torrent clients don't send Want-Digest header
        // Presence of this header indicates a fake client
        headers.iter().any(|(name, _)| {
            name.eq_ignore_ascii_case("want-digest")
        })
    }
    
    pub fn validate(self) -> Result<ValidatedAnnounceParams> {
        // Validate passkey (32 hex characters)
        let passkey = self.validate_passkey()
            .context("Invalid passkey")?;
        
        // Validate info_hash (20 bytes)
        let info_hash = self.validate_info_hash()
            .context("Invalid info_hash")?;
        
        // Validate peer_id (20 bytes)
        let peer_id = self.validate_peer_id()
            .context("Invalid peer_id")?;
        
        // Validate port (1-65535, already enforced by u16 type, but check for 0)
        let port = self.validate_port()
            .context("Invalid port")?;
        
        // Validate numwant (0-200)
        let numwant = self.validate_numwant()
            .context("Invalid numwant")?;
        
        // Validate event
        let event = self.validate_event()
            .context("Invalid event")?;
        
        // Validate compact
        let compact = self.compact == 1;
        
        // Validate IP if provided
        let ip = if let Some(ip_str) = self.ip {
            Some(ip_str.parse::<IpAddr>()
                .context("Invalid IP address")?)
        } else {
            None
        };
        
        Ok(ValidatedAnnounceParams {
            passkey,
            info_hash,
            peer_id,
            port,
            uploaded: self.uploaded,
            downloaded: self.downloaded,
            left: self.left,
            event,
            numwant,
            compact,
            ip,
        })
    }
    
    fn validate_passkey(&self) -> Result<[u8; 32]> {
        let bytes = self.passkey.as_bytes();
        
        if bytes.len() != 32 {
            bail!("Passkey must be exactly 32 characters");
        }
        
        if !bytes.iter().all(|&b| b.is_ascii_alphanumeric()) {
            bail!("Passkey must contain only alphanumeric characters");
        }
        
        let mut passkey = [0u8; 32];
        passkey.copy_from_slice(bytes);
        
        Ok(passkey)
    }
    
    fn validate_info_hash(&self) -> Result<[u8; 20]> {
        let bytes = url_decode(&self.info_hash)
            .context("Failed to URL decode info_hash")?;
        
        if bytes.len() != 20 {
            bail!("Info hash must be exactly 20 bytes");
        }
        
        bytes.try_into()
            .map_err(|_| anyhow!("Failed to convert info_hash to fixed array"))
    }
    
    fn validate_peer_id(&self) -> Result<[u8; 20]> {
        let bytes = url_decode(&self.peer_id)
            .context("Failed to URL decode peer_id")?;
        
        if bytes.len() != 20 {
            bail!("Peer ID must be exactly 20 bytes");
        }
        
        bytes.try_into()
            .map_err(|_| anyhow!("Failed to convert peer_id to fixed array"))
    }
    
    /// Validate port is in range 1-65535 and not blacklisted
    fn validate_port(&self) -> Result<u16> {
        if self.port == 0 {
            bail!("Port must be between 1 and 65535");
        }
        
        // Blacklisted ports - commonly used by P2P software or have security concerns
        // taken from unit3d tracker thx (https://github.com/HDInnovations/UNIT3D/blob/f3fc849198ce5d4313cb9931ac3ca2be4ae541e9/app/Http/Controllers/AnnounceController.php#L51)
        const BLACKLISTED_PORTS: &[u16] = &[
            // HTTP - port used for web traffic
            8080, 8081,
            // Kazaa - peer-to-peer file sharing, some known vulnerabilities
            1214,
            // Microsoft WBT Server, used for Windows Remote Desktop
            3389,
            // eDonkey 2000 P2P file sharing service
            4662,
            // Gnutella (FrostWire, Limewire, Shareaza, etc.), BearShare
            6346, 6347,
            // Port used by p2p software, such as WinMX, Napster
            6699,
        ];
        
        if BLACKLISTED_PORTS.contains(&self.port) {
            bail!("Port is blacklisted");
        }
        
        Ok(self.port)
    }
    

    fn validate_numwant(&self) -> Result<u32> {
        if self.numwant > 200 {
            bail!("Numwant must be between 0 and 200");
        }
        
        Ok(self.numwant)
    }
    

    fn validate_event(&self) -> Result<Option<AnnounceEvent>> {
        if self.event.is_empty() {
            return Ok(None);
        }
        
        match self.event.as_str() {
            "started" => Ok(Some(AnnounceEvent::Started)),
            "stopped" => Ok(Some(AnnounceEvent::Stopped)),
            "completed" => Ok(Some(AnnounceEvent::Completed)),
            _ => bail!("Event must be 'started', 'stopped', 'completed', or empty"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_passkey_valid() {
        let params = AnnounceParams {
            passkey: "abcdef0123456789abcdef0123456789".to_string(), // 32 chars
            info_hash: "%12%34%56%78%9a%bc%de%f0%11%22%33%44%55%66%77%88%99%aa%bb%cc".to_string(),
            peer_id: "%12%34%56%78%9a%bc%de%f0%11%22%33%44%55%66%77%88%99%aa%bb%cc".to_string(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: 1000,
            event: "started".to_string(),
            numwant: 50,
            compact: 1,
            ip: None,
        };
        
        let result = params.validate_passkey();
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_passkey_too_short() {
        let params = AnnounceParams {
            passkey: "0123456789abcdef".to_string(), // Only 16 chars
            info_hash: "".to_string(),
            peer_id: "".to_string(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: 0,
            event: "".to_string(),
            numwant: 50,
            compact: 1,
            ip: None,
        };
        
        let result = params.validate_passkey();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_passkey_invalid_chars() {
        let params = AnnounceParams {
            passkey: "0123456789abcdef!@#$%^&*()12".to_string(), // 32 chars but with special chars
            info_hash: "".to_string(),
            peer_id: "".to_string(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: 0,
            event: "".to_string(),
            numwant: 50,
            compact: 1,
            ip: None,
        };
        
        let result = params.validate_passkey();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_info_hash_valid() {
        let params = AnnounceParams {
            passkey: "".to_string(),
            info_hash: "%12%34%56%78%9a%bc%de%f0%11%22%33%44%55%66%77%88%99%aa%bb%cc".to_string(),
            peer_id: "".to_string(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: 0,
            event: "".to_string(),
            numwant: 50,
            compact: 1,
            ip: None,
        };
        
        let result = params.validate_info_hash();
        assert!(result.is_ok());
        let hash = result.unwrap();
        assert_eq!(hash.len(), 20);
    }

    #[test]
    fn test_validate_info_hash_wrong_length() {
        let params = AnnounceParams {
            passkey: "".to_string(),
            info_hash: "%12%34%56%78".to_string(),
            peer_id: "".to_string(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: 0,
            event: "".to_string(),
            numwant: 50,
            compact: 1,
            ip: None,
        };
        
        let result = params.validate_info_hash();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_peer_id_valid() {
        let params = AnnounceParams {
            passkey: "".to_string(),
            info_hash: "".to_string(),
            peer_id: "%12%34%56%78%9a%bc%de%f0%11%22%33%44%55%66%77%88%99%aa%bb%cc".to_string(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: 0,
            event: "".to_string(),
            numwant: 50,
            compact: 1,
            ip: None,
        };
        
        let result = params.validate_peer_id();
        assert!(result.is_ok());
        let peer_id = result.unwrap();
        assert_eq!(peer_id.len(), 20);
    }

    #[test]
    fn test_validate_port_valid() {
        let params = AnnounceParams {
            passkey: "".to_string(),
            info_hash: "".to_string(),
            peer_id: "".to_string(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: 0,
            event: "".to_string(),
            numwant: 50,
            compact: 1,
            ip: None,
        };
        
        let result = params.validate_port();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 6881);
    }

    #[test]
    fn test_validate_port_zero() {
        let params = AnnounceParams {
            passkey: "".to_string(),
            info_hash: "".to_string(),
            peer_id: "".to_string(),
            port: 0,
            uploaded: 0,
            downloaded: 0,
            left: 0,
            event: "".to_string(),
            numwant: 50,
            compact: 1,
            ip: None,
        };
        
        let result = params.validate_port();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_port_blacklisted() {
        // Test a few blacklisted ports
        let blacklisted = vec![8080, 1214, 3389, 4662, 6346, 6699];
        
        for port in blacklisted {
            let params = AnnounceParams {
                passkey: "".to_string(),
                info_hash: "".to_string(),
                peer_id: "".to_string(),
                port,
                uploaded: 0,
                downloaded: 0,
                left: 0,
                event: "".to_string(),
                numwant: 50,
                compact: 1,
                ip: None,
            };
            
            let result = params.validate_port();
            assert!(result.is_err(), "Port {} should be blacklisted", port);
        }
    }

    #[test]
    fn test_validate_numwant_valid() {
        let params = AnnounceParams {
            passkey: "".to_string(),
            info_hash: "".to_string(),
            peer_id: "".to_string(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: 0,
            event: "".to_string(),
            numwant: 50,
            compact: 1,
            ip: None,
        };
        
        let result = params.validate_numwant();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 50);
    }

    #[test]
    fn test_validate_numwant_max() {
        let params = AnnounceParams {
            passkey: "".to_string(),
            info_hash: "".to_string(),
            peer_id: "".to_string(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: 0,
            event: "".to_string(),
            numwant: 200,
            compact: 1,
            ip: None,
        };
        
        let result = params.validate_numwant();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 200);
    }

    #[test]
    fn test_validate_numwant_too_high() {
        let params = AnnounceParams {
            passkey: "".to_string(),
            info_hash: "".to_string(),
            peer_id: "".to_string(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: 0,
            event: "".to_string(),
            numwant: 201,
            compact: 1,
            ip: None,
        };
        
        let result = params.validate_numwant();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_event_started() {
        let params = AnnounceParams {
            passkey: "".to_string(),
            info_hash: "".to_string(),
            peer_id: "".to_string(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: 0,
            event: "started".to_string(),
            numwant: 50,
            compact: 1,
            ip: None,
        };
        
        let result = params.validate_event();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(AnnounceEvent::Started));
    }

    #[test]
    fn test_validate_event_stopped() {
        let params = AnnounceParams {
            passkey: "".to_string(),
            info_hash: "".to_string(),
            peer_id: "".to_string(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: 0,
            event: "stopped".to_string(),
            numwant: 50,
            compact: 1,
            ip: None,
        };
        
        let result = params.validate_event();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(AnnounceEvent::Stopped));
    }

    #[test]
    fn test_validate_event_completed() {
        let params = AnnounceParams {
            passkey: "".to_string(),
            info_hash: "".to_string(),
            peer_id: "".to_string(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: 0,
            event: "completed".to_string(),
            numwant: 50,
            compact: 1,
            ip: None,
        };
        
        let result = params.validate_event();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(AnnounceEvent::Completed));
    }

    #[test]
    fn test_validate_event_empty() {
        let params = AnnounceParams {
            passkey: "".to_string(),
            info_hash: "".to_string(),
            peer_id: "".to_string(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: 0,
            event: "".to_string(),
            numwant: 50,
            compact: 1,
            ip: None,
        };
        
        let result = params.validate_event();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_validate_event_invalid() {
        let params = AnnounceParams {
            passkey: "".to_string(),
            info_hash: "".to_string(),
            peer_id: "".to_string(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: 0,
            event: "invalid".to_string(),
            numwant: 50,
            compact: 1,
            ip: None,
        };
        
        let result = params.validate_event();
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_full_params() {
        let params = AnnounceParams {
            passkey: "abcdef0123456789abcdef0123456789".to_string(), // 32 chars
            info_hash: "%12%34%56%78%9a%bc%de%f0%11%22%33%44%55%66%77%88%99%aa%bb%cc".to_string(),
            peer_id: "%12%34%56%78%9a%bc%de%f0%11%22%33%44%55%66%77%88%99%aa%bb%cc".to_string(),
            port: 6881,
            uploaded: 1024,
            downloaded: 2048,
            left: 1000000,
            event: "started".to_string(),
            numwant: 50,
            compact: 1,
            ip: Some("192.168.1.1".to_string()),
        };
        
        let result = params.validate();
        assert!(result.is_ok());
        
        let validated = result.unwrap();
        assert_eq!(validated.port, 6881);
        assert_eq!(validated.uploaded, 1024);
        assert_eq!(validated.downloaded, 2048);
        assert_eq!(validated.left, 1000000);
        assert_eq!(validated.event, Some(AnnounceEvent::Started));
        assert_eq!(validated.numwant, 50);
        assert_eq!(validated.compact, true);
        assert!(validated.ip.is_some());
    }
}

    #[test]
    fn test_has_suspicious_headers_with_want_digest() {
        let headers = vec![
            ("User-Agent".to_string(), "BitTorrent/7.10.5".to_string()),
            ("Want-Digest".to_string(), "sha-256".to_string()),
        ];
        
        assert!(AnnounceParams::has_suspicious_headers(&headers));
    }

    #[test]
    fn test_has_suspicious_headers_case_insensitive() {
        let headers = vec![
            ("User-Agent".to_string(), "BitTorrent/7.10.5".to_string()),
            ("want-digest".to_string(), "sha-256".to_string()),
        ];
        
        assert!(AnnounceParams::has_suspicious_headers(&headers));
    }

    #[test]
    fn test_has_suspicious_headers_clean() {
        let headers = vec![
            ("User-Agent".to_string(), "BitTorrent/7.10.5".to_string()),
            ("Accept".to_string(), "*/*".to_string()),
        ];
        
        assert!(!AnnounceParams::has_suspicious_headers(&headers));
    }
