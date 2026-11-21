use axon_api::{app, memory::MemoryRepository, AppState};
use std::{env, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "axon_api=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let host = env::var("AXON_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
    let port = env::var("AXON_PORT")
        .unwrap_or_else(|_| "8080".to_owned())
        .parse::<u16>()?;
    let address: SocketAddr = format!("{host}:{port}").parse()?;

    if !address.ip().is_loopback()
        && env::var("AXON_ALLOW_INSECURE_REMOTE").as_deref() != Ok("true")
    {
        return Err(
            "Refusing non-loopback bind before authentication is implemented. Set AXON_ALLOW_INSECURE_REMOTE=true only for an isolated development environment."
                .into(),
        );
    }

    let listener = TcpListener::bind(address).await?;
    let mut state = AppState::default();
    match env::var("MONGODB_URI") {
        Ok(uri) if !uri.trim().is_empty() => {
            let database = env::var("MONGODB_DATABASE").unwrap_or_else(|_| "axon".to_owned());
            let repository = MemoryRepository::connect(&uri, &database).await?;
            state.memory = Some(Arc::new(repository));
            tracing::info!(database = %database, "MongoDB-backed durable memory enabled");
        }
        _ if env::var("AXON_ENV").as_deref() == Ok("production") => {
            return Err("MONGODB_URI is required when AXON_ENV=production".into());
        }
        _ => tracing::warn!("MONGODB_URI not configured; durable memory endpoints are disabled"),
    }

    tracing::info!(
        %address,
        "Axon API listening (loopback-only development mode; authentication is not implemented)"
    );
    axum::serve(listener, app(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
