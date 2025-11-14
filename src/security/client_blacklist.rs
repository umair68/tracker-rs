use dashmap::DashSet;

/// Client blacklist for banning malicious BitTorrent clients
///
/// Client matching is performed using substring matching against User-Agent headers.
#[derive(Debug, Default)]
pub struct ClientBlacklist {
    clients: DashSet<String>,
}

impl ClientBlacklist {
    pub fn new() -> Self {
        Self {
            clients: DashSet::new(),
        }
    }

    pub fn with_banned_clients(clients: &[String]) -> Self {
        let blacklist = Self::new();
        
        for client in clients {
            blacklist.ban(client.clone());
        }
        
        tracing::info!(count = blacklist.len(), "Initialized client blacklist with banned clients");
        blacklist
    }

    /// Ban a BitTorrent client
    /// 
    /// Adds the client string to the blacklist.
    /// If the client is already banned, this is a no-op.
    pub fn ban(&self, client: String) {
        self.clients.insert(client);
    }

    /// Unban a BitTorrent client
    /// 
    /// Removes the client string from the blacklist.
    /// If the client is not banned, this is a no-op.
    pub fn unban(&self, client: &str) {
        self.clients.remove(client);
    }

    /// Check if a BitTorrent client is banned
    /// 
    /// Performs substring matching against all banned client strings.
    /// Returns true if the user_agent contains any banned client string.
    /// 
    /// # Arguments
    /// * `user_agent` - The User-Agent header from the BitTorrent client
    pub fn is_banned(&self, user_agent: &str) -> bool {
        self.clients.iter().any(|entry| {
            user_agent.contains(entry.key().as_str())
        })
    }

    /// List all banned client strings
    /// 
    /// Returns a vector of all client strings in the blacklist.
    /// The order is not guaranteed.
    pub fn list(&self) -> Vec<String> {
        self.clients.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Get the total number of banned clients
    pub fn len(&self) -> usize {
        self.clients.len()
    }

    pub fn is_empty(&self) -> bool {
        self.clients.is_empty()
    }

    pub fn clear(&self) {
        self.clients.clear();
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ban_client() {
        let blacklist = ClientBlacklist::new();
        
        blacklist.ban("BadClient".to_string());
        assert!(blacklist.is_banned("BadClient/1.0"));
    }

    #[test]
    fn test_unban_client() {
        let blacklist = ClientBlacklist::new();
        
        blacklist.ban("BadClient".to_string());
        assert!(blacklist.is_banned("BadClient/1.0"));
        
        blacklist.unban("BadClient");
        assert!(!blacklist.is_banned("BadClient/1.0"));
    }

    #[test]
    fn test_is_banned_not_in_list() {
        let blacklist = ClientBlacklist::new();
        
        assert!(!blacklist.is_banned("GoodClient/1.0"));
    }

    #[test]
    fn test_is_banned_substring_match() {
        let blacklist = ClientBlacklist::new();
        
        blacklist.ban("BadClient".to_string());
        
        // Should match any user agent containing "BadClient"
        assert!(blacklist.is_banned("BadClient/1.0"));
        assert!(blacklist.is_banned("BadClient/2.0"));
        assert!(blacklist.is_banned("Mozilla/5.0 BadClient"));
        assert!(blacklist.is_banned("BadClientPro"));
        
        // Should not match different clients
        assert!(!blacklist.is_banned("GoodClient/1.0"));
        assert!(!blacklist.is_banned("qBittorrent/4.5.0"));
    }

    #[test]
    fn test_list_clients() {
        let blacklist = ClientBlacklist::new();
        
        blacklist.ban("BadClient1".to_string());
        blacklist.ban("BadClient2".to_string());
        
        let list = blacklist.list();
        assert_eq!(list.len(), 2);
        assert!(list.contains(&"BadClient1".to_string()));
        assert!(list.contains(&"BadClient2".to_string()));
    }

    #[test]
    fn test_len_and_is_empty() {
        let blacklist = ClientBlacklist::new();
        assert!(blacklist.is_empty());
        assert_eq!(blacklist.len(), 0);
        
        blacklist.ban("BadClient".to_string());
        assert!(!blacklist.is_empty());
        assert_eq!(blacklist.len(), 1);
        
        blacklist.ban("AnotherBadClient".to_string());
        assert_eq!(blacklist.len(), 2);
    }

    #[test]
    fn test_clear() {
        let blacklist = ClientBlacklist::new();
        
        blacklist.ban("BadClient1".to_string());
        blacklist.ban("BadClient2".to_string());
        assert_eq!(blacklist.len(), 2);
        
        blacklist.clear();
        assert!(blacklist.is_empty());
        assert!(!blacklist.is_banned("BadClient1/1.0"));
        assert!(!blacklist.is_banned("BadClient2/1.0"));
    }

    #[test]
    fn test_ban_duplicate() {
        let blacklist = ClientBlacklist::new();
        
        blacklist.ban("BadClient".to_string());
        blacklist.ban("BadClient".to_string());
        
        assert_eq!(blacklist.len(), 1);
        assert!(blacklist.is_banned("BadClient/1.0"));
    }

    #[test]
    fn test_unban_not_banned() {
        let blacklist = ClientBlacklist::new();
        
        // Should not panic
        blacklist.unban("NotBannedClient");
        assert!(!blacklist.is_banned("NotBannedClient/1.0"));
    }

    #[test]
    fn test_with_banned_clients() {
        let clients = vec![
            "BadClient1".to_string(),
            "BadClient2".to_string(),
            "MaliciousClient".to_string(),
        ];
        
        let blacklist = ClientBlacklist::with_banned_clients(&clients);
        
        assert_eq!(blacklist.len(), 3);
        assert!(blacklist.is_banned("BadClient1/1.0"));
        assert!(blacklist.is_banned("BadClient2/2.0"));
        assert!(blacklist.is_banned("MaliciousClient"));
    }

    #[test]
    fn test_with_banned_clients_empty() {
        let clients: Vec<String> = vec![];
        let blacklist = ClientBlacklist::with_banned_clients(&clients);
        
        assert!(blacklist.is_empty());
    }

    #[test]
    fn test_case_sensitive_matching() {
        let blacklist = ClientBlacklist::new();
        
        blacklist.ban("BadClient".to_string());
        
        // Substring matching is case-sensitive
        assert!(blacklist.is_banned("BadClient/1.0"));
        assert!(!blacklist.is_banned("badclient/1.0"));
        assert!(!blacklist.is_banned("BADCLIENT/1.0"));
    }

    #[test]
    fn test_multiple_banned_clients() {
        let blacklist = ClientBlacklist::new();
        
        blacklist.ban("BadClient".to_string());
        blacklist.ban("MaliciousBot".to_string());
        blacklist.ban("FakeSeeder".to_string());
        
        assert!(blacklist.is_banned("BadClient/1.0"));
        assert!(blacklist.is_banned("MaliciousBot"));
        assert!(blacklist.is_banned("FakeSeeder v2"));
        assert!(!blacklist.is_banned("qBittorrent/4.5.0"));
    }

    #[test]
    fn test_partial_match() {
        let blacklist = ClientBlacklist::new();
        
        blacklist.ban("Thunder".to_string());
        
        // Should match any user agent containing "Thunder"
        assert!(blacklist.is_banned("Thunder"));
        assert!(blacklist.is_banned("Xunlei Thunder"));
        assert!(blacklist.is_banned("Thunder/5.9.0"));
        assert!(!blacklist.is_banned("qBittorrent"));
    }
}
