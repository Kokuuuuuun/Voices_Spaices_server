use axum::Router;
use socketioxide::SocketIo;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::FmtSubscriber;
use std::net::SocketAddr;

mod state;
mod handlers;
mod types;
mod db;
mod api;
mod auth;

use state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting VoiceSpaces Rust Server...");

    // Setup State
    // HuggingFace provides persistent storage at /data
    // Use in-memory as fallback if /data doesn't exist
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        if std::path::Path::new("/data").exists() {
            "sqlite:/data/voicespaces.db?mode=rwc".to_string()
        } else {
            "sqlite::memory:".to_string()
        }
    });
    info!("Using database: {}", database_url);
    let state = AppState::new(&database_url).await?;

    // Setup Socket.IO
    let (layer, io) = SocketIo::builder()
        .with_state(state.clone())
        .build_layer();

    io.ns("/", handlers::on_connect);

    // Setup Axum Router
    let app = Router::new()
        .route("/api/register", axum::routing::post(auth::register))
        .route("/api/login", axum::routing::post(auth::login))
        .route("/api/rooms", axum::routing::get(api::list_rooms))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(CorsLayer::permissive()) // Allow all CORS for dev
                .layer(layer)
        );

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "7860".to_string())
        .parse()
        .unwrap_or(7860);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
