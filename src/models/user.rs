#[derive(Clone, Debug)]
pub struct User {
    /// User ID
    pub id: u32,
    /// 32-byte passkey (32 hex characters as bytes)
    pub passkey: [u8; 32],
    /// User class/level
    pub class: u8,
    /// Whether the user account is active
    pub is_active: bool,
}

impl User {
    pub fn new(id: u32, passkey: [u8; 32], class: u8, is_active: bool) -> Self {
        Self {
            id,
            passkey,
            class,
            is_active,
        }
    }
}
