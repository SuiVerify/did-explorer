mod types;
mod redis_subscriber;
mod api_handlers;

use axum::{
    routing::get,
    Router,
};
use tower_http::cors::{CorsLayer, Any};
use tokio::sync::broadcast;
use sqlx::postgres::PgPoolOptions;
use tracing::{info, error};
use anyhow::Result;
use std::net::SocketAddr;

use crate::types::DIDClaimedEvent;
use crate::redis_subscriber::RedisSubscriber;
use crate::api_handlers::{sse_events, get_recent_events};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Load environment variables
    dotenvy::dotenv().ok();
    
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let redis_url = std::env::var("REDIS_URL")
        .expect("REDIS_URL must be set");
    
    info!("Starting SSE API Server");
    
    // Create PostgreSQL connection pool for REST queries
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    
    info!("Connected to PostgreSQL");
    
    // Create broadcast channel for SSE
    let (tx, _rx) = broadcast::channel::<DIDClaimedEvent>(100);
    
    // Start Redis subscriber
    let subscriber = RedisSubscriber::new(tx.clone());
    tokio::spawn(async move {
        if let Err(e) = subscriber.start(redis_url).await {
            error!("Redis subscriber error: {}", e);
        }
    });
    
    // Configure CORS to allow frontend requests
    let cors = CorsLayer::new()
        .allow_origin(Any)  // Allow all origins (can be restricted to specific origins)
        .allow_methods(Any)  // Allow all methods
        .allow_headers(Any); // Allow all headers
    
    // Build API routes
    let app = Router::new()
        // SSE endpoint for real-time updates
        .route("/api/sse/events", get(sse_events).with_state(tx.clone()))
        // REST endpoint for historical queries
        .route("/api/events", get(get_recent_events).with_state(db_pool))
        // Add CORS layer
        .layer(cors);
    
    // Start server
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    info!("API server listening on http://{}", addr);
    info!("SSE endpoint: http://{}/api/sse/events", addr);
    info!("REST endpoint: http://{}/api/events", addr);
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}