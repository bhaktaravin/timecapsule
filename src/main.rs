mod config;
mod crypto;
mod db;
mod email;
mod error;
mod jobs;
mod models;
mod routes;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use tower_http::{
    cors::CorsLayer,
    limit::RequestBodyLimitLayer,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::Config;
use db::AppState;
use email::EmailConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "timecapsule=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env()?;
    let email = Arc::new(EmailConfig::from_env()?);
    if email.is_some() {
        tracing::info!("email notifications enabled");
    } else {
        tracing::info!(
            "email notifications disabled (set MAILTRAP_API_TOKEN or SMTP_HOST to enable)"
        );
    }

    let state = AppState::new(&config, email).await?;

    jobs::spawn_unlock_scheduler(state.clone());

    let app = Router::new()
        .merge(routes::router())
        .layer(RequestBodyLimitLayer::new(32 * 1024))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::new(config.host.parse()?, config.port);
    tracing::info!(%addr, "timecapsule listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
