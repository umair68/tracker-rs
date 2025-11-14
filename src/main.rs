mod core {
    pub mod config;
    pub mod error;
    pub mod state;
    pub mod routes;
    pub mod startup;
    pub mod tracing_init;
}

mod handlers;
mod models;
mod stores;
mod security;
mod anti_cheat;
mod bencode;
mod api;
mod wal;
mod metrics;
mod validation;
mod utils;

use anyhow::{bail, Context, Result};
use api::client::ApiClient;
use axum::serve;
use core::config::Config;
use core::state::AppState;
use core::startup::{apply_wal_operations, populate_from_api};
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, UnixListener};
use std::net::SocketAddr;
use tower::Service;
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::{info, debug, error, Level};
use wal::wal::Wal;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    
    let config_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("config.toml")
    };
    
    // Load and validate configuration
    let config = Config::from_file(&config_path)
        .context(format!(
            "Failed to load configuration from '{}'. \
            If this is your first time running the tracker, copy config.example.toml to config.toml and adjust the values.",
            config_path.display()
        ))?;
    
    // Initialize tracing/logging
    core::tracing_init::init_tracing(&config.logging);
    
    // Build Tokio runtime with configured number of threads
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.server.num_threads)
        .enable_all()
        .build()
        .context("Failed to build Tokio runtime")?;
    
    // Run the async main function
    runtime.block_on(async_main(config, config_path))
}

async fn async_main(config: Config, config_path: PathBuf) -> Result<()> {
    info!(
        config_path = %config_path.display(),
        port = ?config.server.port,
        unix_socket = ?config.server.unix_socket,
        num_threads = config.server.num_threads,
        log_level = %config.logging.level,
        log_format = %config.logging.format,
        "BitTorrent Tracker starting"
    );
    
    // Initialize WAL
    let wal_path = PathBuf::from("tracker.wal");
    let wal = Wal::new(wal_path.clone())
        .context("Failed to initialize WAL")?;
    
    info!(wal_path = %wal_path.display(), "WAL initialized");
    
    // Create application state
    let state = AppState::new(config.clone(), wal);
    
    // Replay WAL operations to restore cache state
    info!("Replaying WAL operations");
    let operations = state.wal.replay()
        .context("Failed to replay WAL")?;
    
    apply_wal_operations(&state, &operations)?;
    
    info!(
        operations_replayed = operations.len(),
        users_loaded = state.user_cache.len(),
        torrents_loaded = state.torrent_cache.len(),
        "WAL replay completed"
    );
    
    // Fetch data from external API
    info!(
        endpoint = %config.sync.data_endpoint,
        "Fetching data from external API"
    );
    
    let api_client = ApiClient::new(
        config.sync.data_endpoint.clone(),
        config.sync.api_key.clone(),
    ).context("Failed to create API client")?;
    
    match populate_from_api(&state, &api_client).await {
        Ok(_) => {
            info!("Successfully populated caches from external API");
        }
        Err(e) => {
            error!(
                error = %e,
                "Failed to fetch data from external API, continuing with WAL data only"
            );
        }
    }
    
    // Spawn background cleanup task
    spawn_cleanup_task(
        Arc::clone(&state.peer_store),
        config.performance.cleanup_interval,
        config.performance.peer_timeout,
    );
    
    info!(
        cleanup_interval_seconds = config.performance.cleanup_interval,
        peer_timeout_seconds = config.performance.peer_timeout,
        "Peer cleanup task started"
    );
    
    // Log final startup statistics
    info!(
        users = state.user_cache.len(),
        torrents = state.torrent_cache.len(),
        peers = state.peer_store.total_peers(),
        banned_ips_ipv4 = state.ip_blacklist.list_ipv4().len(),
        banned_ips_ipv6 = state.ip_blacklist.list_ipv6().len(),
        banned_clients = state.client_blacklist.list().len(),
        "BitTorrent Tracker startup complete"
    );
    
    // Build the router with middleware
    let app = core::routes::build_router(Arc::new(state))
        .layer(
            ServiceBuilder::new()
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(DefaultMakeSpan::new().level(Level::DEBUG))
                        .on_response(DefaultOnResponse::new().level(Level::DEBUG))
                )
        );
    
    // Start HTTP server(s)
    let tcp_handle = if let Some(port) = config.server.port {
        let addr = format!("0.0.0.0:{}", port);
        info!(address = %addr, "Starting TCP listener");
        
        let listener = TcpListener::bind(&addr).await
            .context(format!("Failed to bind TCP listener to {}", addr))?;
        
        info!(address = %addr, "TCP listener bound successfully");
        
        let app_clone = app.clone();
        Some(tokio::spawn(async move {
            serve(
                listener,
                app_clone.into_make_service_with_connect_info::<SocketAddr>()
            )
                .with_graceful_shutdown(shutdown_signal())
                .await
                .context("TCP server error")
        }))
    } else {
        None
    };
    
    let unix_handle = if let Some(unix_socket) = &config.server.unix_socket {
        info!(path = %unix_socket.display(), "Starting Unix socket listener");
        
        // Remove existing socket file if it exists
        if unix_socket.exists() {
            std::fs::remove_file(unix_socket)
                .context(format!("Failed to remove existing Unix socket: {}", unix_socket.display()))?;
        }
        
        let listener = UnixListener::bind(unix_socket)
            .context(format!("Failed to bind Unix socket listener to {}", unix_socket.display()))?;
        
        info!(path = %unix_socket.display(), "Unix socket listener bound successfully");
        
        let mut make_service = app.into_make_service();
        Some(tokio::spawn(async move {
            use tower::Service;
            
            loop {
                let (socket, _remote_addr) = match listener.accept().await {
                    Ok(conn) => conn,
                    Err(e) => {
                        error!(error = %e, "Failed to accept Unix socket connection");
                        continue;
                    }
                };
                
                let tower_service = match make_service.call(&socket).await {
                    Ok(svc) => svc,
                    Err(infallible) => match infallible {},
                };
                
                tokio::spawn(async move {
                    let socket = hyper_util::rt::TokioIo::new(socket);
                    
                    let hyper_service = hyper::service::service_fn(move |request: hyper::Request<hyper::body::Incoming>| {
                        tower_service.clone().call(request)
                    });
                    
                    if let Err(err) = hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new())
                        .serve_connection_with_upgrades(socket, hyper_service)
                        .await
                    {
                        error!(error = %err, "Error serving Unix socket connection");
                    }
                });
            }
        }))
    } else {
        None
    };
    
    info!("HTTP server(s) started, waiting for shutdown signal");
    
    // Wait for both servers to complete (if they exist)
    match (tcp_handle, unix_handle) {
        (Some(tcp), Some(unix)) => {
            tokio::select! {
                result = tcp => {
                    if let Err(e) = result {
                        error!(error = %e, "TCP server task failed");
                    }
                }
                result = unix => {
                    if let Err(e) = result {
                        error!(error = %e, "Unix socket server task failed");
                    }
                }
            }
        }
        (Some(tcp), None) => {
            if let Err(e) = tcp.await {
                error!(error = %e, "TCP server task failed");
            }
        }
        (None, Some(unix)) => {
            if let Err(e) = unix.await {
                error!(error = %e, "Unix socket server task failed");
            }
        }
        (None, None) => {
            error!("No listeners configured");
            bail!("No listeners configured");
        }
    }
    
    info!("Shutting down gracefully");
    
    Ok(())
}

/// Spawn a background task that periodically cleans up stale peers
fn spawn_cleanup_task(peer_store: Arc<stores::peer_store::PeerStore>, cleanup_interval: u64, peer_timeout: i64) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(cleanup_interval));
        
        loop {
            interval.tick().await;
            
            debug!("Running peer cleanup");
            let removed = peer_store.cleanup_stale_peers(peer_timeout);
            
            if removed > 0 {
                info!(
                    removed_peers = removed,
                    active_peers = peer_store.total_peers(),
                    active_torrents = peer_store.active_torrents(),
                    "Peer cleanup completed"
                );
            } else {
                debug!("Peer cleanup completed, no stale peers found");
            }
        }
    });
}

/// Wait for shutdown signal (Ctrl+C or SIGTERM)
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal");
        },
        _ = terminate => {
            info!("Received SIGTERM signal");
        },
    }
    
    info!("Shutdown signal received, starting graceful shutdown");
}
