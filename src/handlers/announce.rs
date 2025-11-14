use crate::anti_cheat::{announce_interval, duplicate_peer, ghost_seeder, ratio_check, speed_check};
use crate::bencode::response::build_announce_response;
use crate::core::error::AnnounceError;
use crate::core::state::AppState;
use crate::models::peer::Peer;
use crate::utils::time::current_timestamp;
use crate::validation::params::{AnnounceEvent, AnnounceParams};
use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::Response,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{debug, info, instrument, warn};

/// Main announce handler
/// 
/// Processes BitTorrent announce requests from clients.
///
/// # Flow
/// 1. Parse and validate query parameters
/// 2. Extract IP address and User-Agent
/// 3. Authenticate user (check passkey)
/// 4. Authorize torrent (check info_hash)
/// 5. Check IP blacklist
/// 6. Check client blacklist
/// 7. Check rate limit
/// 8. Run anti-cheat checks (log warnings, don't block)
/// 9. Handle lifecycle events (started, stopped, completed)
/// 10. Update peer in peer store
/// 11. Get peer list
/// 12. Build and return bencode response
#[instrument(skip(state, headers, raw_query))]
pub async fn announce_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::RawQuery(raw_query): axum::extract::RawQuery,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Response, AnnounceError> {
    let query_str = raw_query.ok_or_else(|| {
        warn!("Missing query string - browser access");
        state.metrics.increment_failed();
        AnnounceError::BrowserAccess
    })?;
    
    let mut passkey = "";
    let mut info_hash = "";
    let mut peer_id = "";
    let mut port = 0u16;
    let mut uploaded = 0u64;
    let mut downloaded = 0u64;
    let mut left = 0u64;
    let mut event = "";
    let mut numwant = 50u32;
    let mut compact = 1u8;
    let mut ip: Option<&str> = None;
    
    for pair in query_str.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            match key {
                "passkey" => passkey = value,
                "info_hash" => info_hash = value,
                "peer_id" => peer_id = value,
                "port" => port = value.parse().unwrap_or(0),
                "uploaded" => uploaded = value.parse().unwrap_or(0),
                "downloaded" => downloaded = value.parse().unwrap_or(0),
                "left" => left = value.parse().unwrap_or(0),
                "event" => event = value,
                "numwant" => numwant = value.parse().unwrap_or(50),
                "compact" => compact = value.parse().unwrap_or(1),
                "ip" => ip = Some(value),
                _ => {}
            }
        }
    }

    if !passkey.is_empty() && info_hash.is_empty() && peer_id.is_empty() {
        warn!("Browser access detected: only passkey provided");
        state.metrics.increment_failed();
        return Err(AnnounceError::BrowserAccess);
    }

    let params = AnnounceParams {
        passkey: passkey.to_string(),
        info_hash: info_hash.to_string(),
        peer_id: peer_id.to_string(),
        port,
        uploaded,
        downloaded,
        left,
        event: event.to_string(),
        numwant,
        compact,
        ip: ip.map(|s| s.to_string()),
    };
    debug!("Processing announce request");

    state.metrics.increment_announces();

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("Unknown")
        .to_string();

    let header_list: Vec<(String, String)> = headers
        .iter()
        .map(|(name, value)| {
            (
                name.as_str().to_string(),
                value.to_str().unwrap_or("").to_string(),
            )
        })
        .collect();

    if AnnounceParams::has_suspicious_headers(&header_list) {
        warn!(
            user_agent = %user_agent,
            "Suspicious client detected: fake client headers"
        );
        state.metrics.increment_blocked();
        return Err(AnnounceError::SuspiciousClient);
    }

    let validated = params.validate().map_err(|e| {
        warn!(error = %e, "Parameter validation failed");
        state.metrics.increment_failed();
        AnnounceError::InvalidParameter("Invalid announce parameters".to_string())
    })?;

    let ip = validated.ip.unwrap_or(addr.ip());

    debug!(
        ip = %ip,
        port = validated.port,
        uploaded = validated.uploaded,
        downloaded = validated.downloaded,
        left = validated.left,
        event = ?validated.event,
        "Validated announce parameters"
    );

    let user = state
        .user_cache
        .get_user(validated.passkey)
        .ok_or_else(|| {
            warn!(passkey = ?validated.passkey, "Invalid passkey");
            state.metrics.increment_failed();
            AnnounceError::InvalidPasskey
        })?;

    if !user.is_active {
        warn!(user_id = user.id, "User account is disabled");
        state.metrics.increment_failed();
        return Err(AnnounceError::UserDisabled);
    }

    info!(user_id = user.id, "User authenticated");

    let torrent = state
        .torrent_cache
        .get_torrent(validated.info_hash)
        .ok_or_else(|| {
            warn!(info_hash = ?validated.info_hash, "Torrent not registered");
            state.metrics.increment_failed();
            AnnounceError::TorrentNotFound
        })?;

    if !torrent.is_active {
        warn!(torrent_id = torrent.id, "Torrent is not active");
        state.metrics.increment_failed();
        return Err(AnnounceError::TorrentInactive);
    }

    debug!(torrent_id = torrent.id, "Torrent authorized");

    if state.ip_blacklist.is_banned(ip) {
        warn!(ip = %ip, "IP address is banned");
        state.metrics.increment_blocked();
        return Err(AnnounceError::IpBanned);
    }

    if state.client_blacklist.is_banned(&user_agent) {
        warn!(user_agent = %user_agent, "Client is banned");
        state.metrics.increment_blocked();
        return Err(AnnounceError::ClientBanned);
    }

    let current_time = current_timestamp();
    if !state.rate_limiter.check_and_increment(ip, current_time) {
        warn!(ip = %ip, "Rate limit exceeded");
        state.metrics.increment_blocked();
        return Err(AnnounceError::RateLimitExceeded);
    }

    let existing_peer = state
        .peer_store
        .get_peers(validated.info_hash, 1, validated.peer_id)
        .into_iter()
        .find(|p| p.user_id == user.id);

    let last_announce = existing_peer.as_ref().map(|p| p.last_announce);
    if let Err(e) = announce_interval::check_announce_interval(
        user.id,
        torrent.id,
        last_announce,
        current_time,
        state.config.performance.min_announce_interval,
    ) {
        warn!(
            user_id = user.id,
            torrent_id = torrent.id,
            error = %e,
            "Announce interval check failed"
        );
    }

    if let Err(e) = duplicate_peer::check_duplicate_peer(
        &state.peer_store,
        user.id,
        torrent.id,
        state.config.anti_cheat.max_ips_per_user,
    ) {
        warn!(
            user_id = user.id,
            torrent_id = torrent.id,
            error = %e,
            "Duplicate peer check failed"
        );
    }

    if let Some(ref old_peer) = existing_peer {
        let elapsed = current_time - old_peer.last_announce;
        if let Err(e) = speed_check::check_speed(
            user.id,
            torrent.id,
            old_peer.uploaded,
            validated.uploaded,
            old_peer.downloaded,
            validated.downloaded,
            elapsed,
            state.config.anti_cheat.max_upload_speed,
        ) {
            warn!(
                user_id = user.id,
                torrent_id = torrent.id,
                error = %e,
                "Speed check failed"
            );
        }
    }

    if let Err(e) = ratio_check::check_ratio(
        user.id,
        torrent.id,
        validated.uploaded,
        validated.downloaded,
        state.config.anti_cheat.max_ratio,
    ) {
        warn!(
            user_id = user.id,
            torrent_id = torrent.id,
            error = %e,
            "Ratio check failed"
        );
    }

    let is_seeder = validated.left == 0;
    let is_completed_event = validated.event == Some(AnnounceEvent::Completed);
    if let Err(e) = ghost_seeder::check_ghost_seeder(
        user.id,
        torrent.id,
        is_seeder,
        validated.uploaded,
        state.config.anti_cheat.min_seeder_upload,
        is_completed_event,
    ) {
        warn!(
            user_id = user.id,
            torrent_id = torrent.id,
            error = %e,
            "Ghost seeder check failed"
        );
    }

    match validated.event {
        Some(AnnounceEvent::Stopped) => {
            if let Err(e) = state.peer_store.remove_peer(validated.info_hash, validated.peer_id) {
                warn!(
                    user_id = user.id,
                    torrent_id = torrent.id,
                    error = %e,
                    "Failed to remove peer"
                );
            } else {
                info!(
                    user_id = user.id,
                    torrent_id = torrent.id,
                    "Peer stopped and removed"
                );
            }

            let (seeders, leechers) = state.peer_store.get_stats(validated.info_hash);
            let response = build_announce_response(&[], seeders, leechers, validated.compact);

            state.metrics.increment_successful();
            return Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/plain")
                .body(response.into())
                .unwrap());
        }
        Some(AnnounceEvent::Started) => {
            info!(
                user_id = user.id,
                torrent_id = torrent.id,
                "Peer started"
            );
        }
        Some(AnnounceEvent::Completed) => {
            info!(
                user_id = user.id,
                torrent_id = torrent.id,
                "Peer completed download"
            );
        }
        None => {}
    }

    let peer = Peer::new(
        user.id,
        torrent.id,
        validated.peer_id,
        ip,
        validated.port,
        validated.uploaded,
        validated.downloaded,
        validated.left,
        current_time,
        user_agent.clone(),
    );

    if existing_peer.is_some() {
        state
            .peer_store
            .update_peer(validated.info_hash, validated.peer_id, peer)
            .map_err(|e| {
                warn!(error = %e, "Failed to update peer");
                state.metrics.increment_failed();
                AnnounceError::InternalError(e)
            })?;
        debug!(user_id = user.id, torrent_id = torrent.id, "Peer updated");
    } else {
        state
            .peer_store
            .add_peer(validated.info_hash, peer)
            .map_err(|e| {
                warn!(error = %e, "Failed to add peer");
                state.metrics.increment_failed();
                AnnounceError::InternalError(e)
            })?;
        info!(user_id = user.id, torrent_id = torrent.id, "Peer added");
    }

    let peers = state.peer_store.get_peers(
        validated.info_hash,
        validated.numwant,
        validated.peer_id,
    );

    let (seeders, leechers) = state.peer_store.get_stats(validated.info_hash);

    debug!(
        seeders = seeders,
        leechers = leechers,
        peers_returned = peers.len(),
        "Building announce response"
    );

    let response = build_announce_response(&peers, seeders, leechers, validated.compact);

    state.metrics.increment_successful();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/plain")
        .body(response.into())
        .unwrap())
}
