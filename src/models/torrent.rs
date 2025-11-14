#[derive(Clone, Debug)]
pub struct Torrent {
    /// Torrent ID
    pub id: u32,
    /// 20-byte SHA-1 info hash
    pub info_hash: [u8; 20],
    /// Whether this torrent is freeleech (downloads don't count toward ratio)
    pub is_freeleech: bool,
    /// Whether this torrent is active
    pub is_active: bool,
}

impl Torrent {
    pub fn new(id: u32, info_hash: [u8; 20], is_freeleech: bool, is_active: bool) -> Self {
        Self {
            id,
            info_hash,
            is_freeleech,
            is_active,
        }
    }
}
