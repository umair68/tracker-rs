use dashmap::DashSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

#[derive(Debug, Default)]
pub struct IpBlacklist {
    ipv4: DashSet<Ipv4Addr>,
    ipv6: DashSet<Ipv6Addr>,
}

impl IpBlacklist {
    pub fn new() -> Self {
        Self {
            ipv4: DashSet::new(),
            ipv6: DashSet::new(),
        }
    }


    pub fn with_banned_ips(ips: &[String]) -> Self {
        let blacklist = Self::new();
        
        for ip_str in ips {
            match ip_str.parse::<IpAddr>() {
                Ok(ip) => blacklist.ban(ip),
                Err(e) => {
                    tracing::warn!(ip = %ip_str, error = %e, "Failed to parse IP address in config");
                }
            }
        }
        
        tracing::info!(count = blacklist.len(), "Initialized IP blacklist with banned IPs");
        blacklist
    }


    pub fn ban(&self, ip: IpAddr) {
        match ip {
            IpAddr::V4(ipv4) => {
                self.ipv4.insert(ipv4);
            }
            IpAddr::V6(ipv6) => {
                self.ipv6.insert(ipv6);
            }
        }
    }


    pub fn unban(&self, ip: IpAddr) {
        match ip {
            IpAddr::V4(ipv4) => {
                self.ipv4.remove(&ipv4);
            }
            IpAddr::V6(ipv6) => {
                self.ipv6.remove(&ipv6);
            }
        }
    }


    pub fn is_banned(&self, ip: IpAddr) -> bool {
        match ip {
            IpAddr::V4(ipv4) => self.ipv4.contains(&ipv4),
            IpAddr::V6(ipv6) => self.ipv6.contains(&ipv6),
        }
    }


    pub fn list_ipv4(&self) -> Vec<Ipv4Addr> {
        self.ipv4.iter().map(|entry| *entry.key()).collect()
    }


    pub fn list_ipv6(&self) -> Vec<Ipv6Addr> {
        self.ipv6.iter().map(|entry| *entry.key()).collect()
    }

    pub fn len(&self) -> usize {
        self.ipv4.len() + self.ipv6.len()
    }


    pub fn is_empty(&self) -> bool {
        self.ipv4.is_empty() && self.ipv6.is_empty()
    }


    pub fn clear(&self) {
        self.ipv4.clear();
        self.ipv6.clear();
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ban_ipv4() {
        let blacklist = IpBlacklist::new();
        let ip = "192.168.1.1".parse::<Ipv4Addr>().unwrap();

        blacklist.ban(IpAddr::V4(ip));
        assert!(blacklist.is_banned(IpAddr::V4(ip)));
    }

    #[test]
    fn test_ban_ipv6() {
        let blacklist = IpBlacklist::new();
        let ip = "2001:db8::1".parse::<Ipv6Addr>().unwrap();

        blacklist.ban(IpAddr::V6(ip));
        assert!(blacklist.is_banned(IpAddr::V6(ip)));
    }

    #[test]
    fn test_unban_ipv4() {
        let blacklist = IpBlacklist::new();
        let ip = "192.168.1.1".parse::<Ipv4Addr>().unwrap();

        blacklist.ban(IpAddr::V4(ip));
        assert!(blacklist.is_banned(IpAddr::V4(ip)));

        blacklist.unban(IpAddr::V4(ip));
        assert!(!blacklist.is_banned(IpAddr::V4(ip)));
    }

    #[test]
    fn test_unban_ipv6() {
        let blacklist = IpBlacklist::new();
        let ip = "2001:db8::1".parse::<Ipv6Addr>().unwrap();

        blacklist.ban(IpAddr::V6(ip));
        assert!(blacklist.is_banned(IpAddr::V6(ip)));

        blacklist.unban(IpAddr::V6(ip));
        assert!(!blacklist.is_banned(IpAddr::V6(ip)));
    }

    #[test]
    fn test_is_banned_not_in_list() {
        let blacklist = IpBlacklist::new();
        let ip = "192.168.1.1".parse::<Ipv4Addr>().unwrap();

        assert!(!blacklist.is_banned(IpAddr::V4(ip)));
    }

    #[test]
    fn test_list_ipv4() {
        let blacklist = IpBlacklist::new();
        let ip1 = "192.168.1.1".parse::<Ipv4Addr>().unwrap();
        let ip2 = "10.0.0.1".parse::<Ipv4Addr>().unwrap();

        blacklist.ban(IpAddr::V4(ip1));
        blacklist.ban(IpAddr::V4(ip2));

        let list = blacklist.list_ipv4();
        assert_eq!(list.len(), 2);
        assert!(list.contains(&ip1));
        assert!(list.contains(&ip2));
    }

    #[test]
    fn test_list_ipv6() {
        let blacklist = IpBlacklist::new();
        let ip1 = "2001:db8::1".parse::<Ipv6Addr>().unwrap();
        let ip2 = "2001:db8::2".parse::<Ipv6Addr>().unwrap();

        blacklist.ban(IpAddr::V6(ip1));
        blacklist.ban(IpAddr::V6(ip2));

        let list = blacklist.list_ipv6();
        assert_eq!(list.len(), 2);
        assert!(list.contains(&ip1));
        assert!(list.contains(&ip2));
    }

    #[test]
    fn test_len_and_is_empty() {
        let blacklist = IpBlacklist::new();
        assert!(blacklist.is_empty());
        assert_eq!(blacklist.len(), 0);

        let ipv4 = "192.168.1.1".parse::<Ipv4Addr>().unwrap();
        let ipv6 = "2001:db8::1".parse::<Ipv6Addr>().unwrap();

        blacklist.ban(IpAddr::V4(ipv4));
        assert!(!blacklist.is_empty());
        assert_eq!(blacklist.len(), 1);

        blacklist.ban(IpAddr::V6(ipv6));
        assert_eq!(blacklist.len(), 2);
    }

    #[test]
    fn test_clear() {
        let blacklist = IpBlacklist::new();
        let ipv4 = "192.168.1.1".parse::<Ipv4Addr>().unwrap();
        let ipv6 = "2001:db8::1".parse::<Ipv6Addr>().unwrap();

        blacklist.ban(IpAddr::V4(ipv4));
        blacklist.ban(IpAddr::V6(ipv6));
        assert_eq!(blacklist.len(), 2);

        blacklist.clear();
        assert!(blacklist.is_empty());
        assert!(!blacklist.is_banned(IpAddr::V4(ipv4)));
        assert!(!blacklist.is_banned(IpAddr::V6(ipv6)));
    }

    #[test]
    fn test_ban_duplicate() {
        let blacklist = IpBlacklist::new();
        let ip = "192.168.1.1".parse::<Ipv4Addr>().unwrap();

        blacklist.ban(IpAddr::V4(ip));
        blacklist.ban(IpAddr::V4(ip));

        assert_eq!(blacklist.len(), 1);
        assert!(blacklist.is_banned(IpAddr::V4(ip)));
    }

    #[test]
    fn test_unban_not_banned() {
        let blacklist = IpBlacklist::new();
        let ip = "192.168.1.1".parse::<Ipv4Addr>().unwrap();

        // Should not panic
        blacklist.unban(IpAddr::V4(ip));
        assert!(!blacklist.is_banned(IpAddr::V4(ip)));
    }

    #[test]
    fn test_with_banned_ips() {
        let ips = vec![
            "192.168.1.1".to_string(),
            "10.0.0.1".to_string(),
            "2001:db8::1".to_string(),
        ];
        
        let blacklist = IpBlacklist::with_banned_ips(&ips);
        
        assert_eq!(blacklist.len(), 3);
        assert!(blacklist.is_banned("192.168.1.1".parse().unwrap()));
        assert!(blacklist.is_banned("10.0.0.1".parse().unwrap()));
        assert!(blacklist.is_banned("2001:db8::1".parse().unwrap()));
    }

    #[test]
    fn test_with_banned_ips_invalid() {
        let ips = vec![
            "192.168.1.1".to_string(),
            "invalid-ip".to_string(),
            "10.0.0.1".to_string(),
        ];
        
        let blacklist = IpBlacklist::with_banned_ips(&ips);
        
        // Should only have 2 valid IPs
        assert_eq!(blacklist.len(), 2);
        assert!(blacklist.is_banned("192.168.1.1".parse().unwrap()));
        assert!(blacklist.is_banned("10.0.0.1".parse().unwrap()));
    }

    #[test]
    fn test_with_banned_ips_empty() {
        let ips: Vec<String> = vec![];
        let blacklist = IpBlacklist::with_banned_ips(&ips);
        
        assert!(blacklist.is_empty());
    }
}
