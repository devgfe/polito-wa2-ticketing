mod auth;
mod common;
mod config;
mod error;
mod events;
mod venues;

use std::sync::Arc;

use anyhow::Context;
use axum::{
    Router,
    extract::FromRef,
    extract::State,
    http::{HeaderValue, StatusCode},
    routing::get,
};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::events::{service::EventService, store::PgEventRepository};
use crate::{
    common::Pagination,
    venues::{service::VenueService, store::PgVenueRepository},
};

// ---------- Stato condiviso ----------

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub pagination: Pagination,
    pub events: EventService,
    pub venues: VenueService,
}

impl FromRef<AppState> for EventService {
    fn from_ref(state: &AppState) -> Self {
        state.events.clone()
    }
}

impl FromRef<AppState> for Pagination {
    fn from_ref(state: &AppState) -> Self {
        state.pagination
    }
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

// ---------- Avvio ----------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env()?;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .context("cannot connect to the database")?;

    sqlx::migrate!()
        .run(&pool)
        .await
        .context("cannot run the migrations")?;
    tracing::info!("migrations applied");

    let state = AppState {
        pool: pool.clone(),
        pagination: Pagination {
            default: config.page_size_default,
            max: config.page_size_max,
        },
        events: EventService::new(Arc::new(PgEventRepository::new(pool.clone()))),
        venues: VenueService::new(Arc::new(PgVenueRepository::new(pool.clone()))),
    };

    let app = Router::new()
        .route("/health", get(health))
        .merge(events::handler::router())
        .merge(venues::handler::router())
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors(&config)?);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", config.port)).await?;
    tracing::info!(port = config.port, "catalogue listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

// 200 se il database risponde, 503 altrimenti.
async fn health(State(pool): State<PgPool>) -> StatusCode {
    match sqlx::query(" SELECT 1").execute(&pool).await {
        Ok(_) => StatusCode::OK,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}

// Permette al frontend (CORS_ORIGINS, es. http://localhost:5173) di chiamare l'API dal browser.
fn cors(config: &Config) -> anyhow::Result<CorsLayer> {
    let origins = config
        .cors_origins
        .iter()
        .map(|o| HeaderValue::from_str(o).with_context(|| format!("invalid CORS origin: {o}")))
        .collect::<anyhow::Result<Vec<_>>>()?;
    Ok(CorsLayer::new()
        .allow_origin(origins)
        .allow_methods(Any)
        .allow_headers(Any))
}

// Ctrl+C (o lo stop di Docker): finisce le richieste in corso e poi si ferma.
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutting down");
}
