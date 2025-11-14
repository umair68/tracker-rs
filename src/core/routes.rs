// HTTP routes configuration

use crate::core::state::AppState;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Public endpoints
        .route("/announce", get(crate::handlers::announce::announce_handler))
        .route("/health", get(crate::handlers::health::health_handler))
        
        // Admin endpoints (require API key)
        .route("/metrics", get(crate::handlers::metrics::metrics_handler))
        .route("/update", get(crate::handlers::update::update_handler))
        .route("/reload", post(crate::handlers::admin::reload_handler))
        .route("/torrent/add", get(crate::handlers::admin::torrent_add_handler))
        .route("/torrent/remove", get(crate::handlers::admin::torrent_remove_handler))
        .route("/user/add", get(crate::handlers::admin::user_add_handler))
        .route("/user/remove", get(crate::handlers::admin::user_remove_handler))
        
        // Blacklist endpoints (require API key)
        .route("/ip/ban", get(crate::handlers::blacklist::ip_ban_handler))
        .route("/ip/unban", get(crate::handlers::blacklist::ip_unban_handler))
        .route("/ip/list", get(crate::handlers::blacklist::ip_list_handler))
        .route("/client/ban", get(crate::handlers::blacklist::client_ban_handler))
        .route("/client/unban", get(crate::handlers::blacklist::client_unban_handler))
        .route("/client/list", get(crate::handlers::blacklist::client_list_handler))

        // 404 fallback for all unmatched routes
        .fallback(crate::handlers::fallback::fallback_handler)

        .with_state(state)
}
