use axum::{routing::get, Router};
use dotenvy::dotenv;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use tokio::signal;
use tracing::{info, warn};

mod db;
mod handlers;
mod protocols;

pub struct AppState {
    pub pool: Pool<Sqlite>,
}

#[tokio::main]
async fn main() {
    // load environment variables from .env file
    if dotenv().is_err() {
        warn!("No .env file found");
    }

    // initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // initialize database
    let pool = db::initialize_db().await;
    let shared_state = Arc::new(AppState { pool });

    //initialize average message service
    tokio::spawn(protocols::avg_msg_service(shared_state.clone()));

    // initialize router
    let app = Router::new()
        .route("/", get(handlers::health_handler))
        .route("/ws", get(handlers::handler))
        .with_state(shared_state.clone());

    info!("Starting the cloud server...");
    // start server
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

// Graceful shutdown
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutting down...");
}
