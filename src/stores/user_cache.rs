use crate::models::user::User;
use dashmap::DashMap;
use std::sync::Arc;

/// In-memory cache for user data
pub struct UserCache {
    users: DashMap<[u8; 32], Arc<User>>,
}

impl UserCache {
    /// Create a new UserCache instance
    pub fn new() -> Self {
        Self {
            users: DashMap::new(),
        }
    }


    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            users: DashMap::with_capacity(capacity),
        }
    }

    /// Add a user to the cache
    /// If a user with the same passkey already exists, it will be replaced
    pub fn add_user(&self, user: User) {
        let passkey = user.passkey;
        self.users.insert(passkey, Arc::new(user));
    }

    /// Remove a user from the cache by passkey
    /// Returns the removed user if it existed
    pub fn remove_user(&self, passkey: [u8; 32]) -> Option<Arc<User>> {
        self.users.remove(&passkey).map(|(_, user)| user)
    }

    /// Get a user from the cache by passkey
    /// Returns a clone of the user if found
    pub fn get_user(&self, passkey: [u8; 32]) -> Option<Arc<User>> {
        self.users.get(&passkey).map(|entry| Arc::clone(entry.value()))
    }

    /// Get a user from the cache by user ID
    /// Returns a clone of the user if found
    /// Note: This is a linear search and should be used sparingly
    pub fn get_user_by_id(&self, user_id: u32) -> Option<Arc<User>> {
        self.users
            .iter()
            .find(|entry| entry.value().id == user_id)
            .map(|entry| Arc::clone(entry.value()))
    }


    pub fn clear(&self) {
        self.users.clear();
    }


    pub fn len(&self) -> usize {
        self.users.len()
    }


    pub fn is_empty(&self) -> bool {
        self.users.is_empty()
    }
}

impl Default for UserCache {
    fn default() -> Self {
        Self::new()
    }
}
