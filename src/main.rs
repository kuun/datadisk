// Allow dead code for reserved/future-use structures
#![allow(dead_code)]

use std::env;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

mod config;
mod db;
mod entity;
mod error;
mod handlers;
mod middleware;
mod permission;
mod routes;
mod state;
mod task;
mod ws;

use config::Config;
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|arg| arg == "-help" || arg == "--help") {
        println!("Usage: datadisk [OPTIONS]");
        println!("Options:");
        println!("  -config <path>  Path to configuration file (default: ./etc/datadisk.toml)");
        println!("  -help, --help   Print this help message");
        return Ok(());
    }

    // Parse command line arguments
    let config_path = args
        .iter()
        .skip_while(|arg| arg.as_str() != "-config")
        .nth(1)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "./etc/datadisk.toml".to_string());

    // Load configuration first (before logging init)
    let config = Config::load(&config_path).unwrap_or_else(|e| {
        eprintln!("Could not load config file: {}, using defaults", e);
        Config::default()
    });

    // Initialize logging
    // Priority: RUST_LOG env var > config file > default "info"
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log.level));

    fmt::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("Starting Datadisk server...");
    info!("Loading configuration from: {}", config_path);

    // Initialize database connection only if system is initialized
    let (db, perm_enforcer) = if config.initialized {
        let db_conn = db::init_database(&config.database).await.map_err(|e| {
            tracing::error!("Database initialization failed: {}", e);
            anyhow::anyhow!("Database initialization failed: {}", e)
        })?;

        // Initialize audit log service
        handlers::audit::service::init(db_conn.clone());
        info!("Audit log service initialized");

        // Initialize permission enforcer
        let enforcer = permission::PermissionEnforcer::new(
            db_conn.clone(),
            config.casbin_conf.to_str().unwrap_or("./etc/casbin_model.conf"),
        ).await.map_err(|e| {
            tracing::error!("Permission enforcer initialization failed: {}", e);
            anyhow::anyhow!("Permission enforcer initialization failed: {}", e)
        })?;
        info!("Permission enforcer initialized");

        (Some(db_conn), Some(enforcer))
    } else {
        info!("System not initialized, skipping database connection. Please complete setup.");
        (None, None)
    };

    // Create application state
    let state = AppState::new(db, perm_enforcer, config.clone());

    // Create router
    let app = routes::create_router(state);

    // Parse address
    let addr: SocketAddr = config.addr.parse().unwrap_or_else(|_| {
        tracing::warn!("Invalid address '{}', using default 0.0.0.0:8080", config.addr);
        "0.0.0.0:8080".parse().unwrap()
    });

    info!("Server listening on {}", addr);

    // Start server
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
