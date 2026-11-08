use qapten_api::{app, AppState};
use std::{env, net::SocketAddr};
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "qapten_api=info,tower_http=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let host = env::var("QAPTEN_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
    let port = env::var("QAPTEN_PORT").unwrap_or_else(|_| "8080".to_owned()).parse::<u16>()?;
    let address: SocketAddr = format!("{host}:{port}").parse()?;

    if !address.ip().is_loopback() && env::var("QAPTEN_ALLOW_INSECURE_REMOTE").as_deref() != Ok("true") {
        return Err("Refusing non-loopback bind before authentication is implemented. Set QAPTEN_ALLOW_INSECURE_REMOTE=true only for an isolated development environment.".into());
    }

    let listener = TcpListener::bind(address).await?;
    tracing::info!(%address, "Qapten API listening (development in-memory store; no authentication)");
    axum::serve(listener, app(AppState::default()))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
