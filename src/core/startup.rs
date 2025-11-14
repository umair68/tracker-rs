
use anyhow::{Context, Result};
use tracing::{info, warn};

use crate::api::client::ApiClient;
use crate::models::{torrent::Torrent, user::User};
use crate::core::state::AppState;
use crate::wal::wal::WalOperation;
// this runs at boot time 
pub fn apply_wal_operations(state: &AppState, operations: &[WalOperation]) -> Result<()> {
    for op in operations {
        match op {
            WalOperation::AddTorrent { id, info_hash, freeleech } => {
                let torrent = Torrent::new(*id, *info_hash, *freeleech, true);
                state.torrent_cache.add_torrent(torrent);
            }
            WalOperation::RemoveTorrent { info_hash } => {
                state.torrent_cache.remove_torrent(*info_hash);
            }
            WalOperation::AddUser { id, passkey, class } => {
                let user = User::new(*id, *passkey, *class, true);
                state.user_cache.add_user(user);
            }
            WalOperation::RemoveUser { passkey } => {
                state.user_cache.remove_user(*passkey);
            }
        }
    }
    Ok(())
}

pub async fn populate_from_api(state: &AppState, api_client: &ApiClient) -> Result<()> {
    let api_data = api_client.fetch_data().await
        .context("Failed to fetch data from external API")?;
    
    info!(
        torrents = api_data.torrents.len(),
        users = api_data.users.len(),
        "Data fetched from external API"
    );
    
    for api_torrent in api_data.torrents {
        match hex::decode(&api_torrent.info_hash) {
            Ok(hash_bytes) if hash_bytes.len() == 20 => {
                let mut info_hash = [0u8; 20];
                info_hash.copy_from_slice(&hash_bytes);
                
                let torrent = Torrent::new(
                    api_torrent.id,
                    info_hash,
                    api_torrent.is_freeleech,
                    true, // Assume active from API
                );
                
                state.torrent_cache.add_torrent(torrent);
            }
            Ok(_) => {
                warn!(
                    torrent_id = api_torrent.id,
                    info_hash = %api_torrent.info_hash,
                    "Invalid info_hash length, skipping torrent"
                );
            }
            Err(e) => {
                warn!(
                    torrent_id = api_torrent.id,
                    info_hash = %api_torrent.info_hash,
                    error = %e,
                    "Failed to decode info_hash, skipping torrent"
                );
            }
        }
    }
    
    for api_user in api_data.users {
        // Passkeys are 32-character alphanumeric strings, store as bytes directly
        if api_user.passkey.len() == 32 {
            let passkey_bytes = api_user.passkey.as_bytes();
            let mut passkey = [0u8; 32];
            passkey.copy_from_slice(passkey_bytes);
            
            // User is active if they can download and are not security locked
            let is_active = api_user.can_download && !api_user.security_locked;
            
            let user = User::new(
                api_user.id,
                passkey,
                api_user.user_class_id,
                is_active,
            );
            
            state.user_cache.add_user(user);
        } else {
            warn!(
                user_id = api_user.id,
                passkey = %api_user.passkey,
                passkey_len = api_user.passkey.len(),
                "Invalid passkey length (expected 32), skipping user"
            );
        }
    }
    
    info!(
        users_cached = state.user_cache.len(),
        torrents_cached = state.torrent_cache.len(),
        "Caches populated from external API"
    );
    
    Ok(())
}
