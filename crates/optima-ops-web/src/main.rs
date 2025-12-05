//! Optima Ops Web Dashboard
//!
//! A web-based dashboard for monitoring Optima services health and infrastructure.

use anyhow::Result;
use axum::Router;
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

mod routes;
mod state;

use state::AppState;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    // Load core configuration
    let config = optima_ops_core::AppConfig::load()?;
    tracing::info!("Loaded configuration for environment: {}", config.get_environment());

    // Create application state
    let state = AppState::new(config);

    // Create router
    let app = Router::new()
        .merge(routes::create_router())
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state);

    // Start server
    let addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
