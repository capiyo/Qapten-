use axon_api::{app, auth::FirebaseVerifier, storage::SqliteStorage, AppState};
use std::{env, net::SocketAddr};
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

    let allow_remote = matches!(env::var("AXON_ALLOW_REMOTE_BIND").as_deref(), Ok("true"));
    let tls_terminated = matches!(env::var("AXON_TLS_TERMINATED").as_deref(), Ok("true"));
    if !address.ip().is_loopback() && !(allow_remote && tls_terminated) {
        return Err("Remote binding requires AXON_ALLOW_REMOTE_BIND=true and AXON_TLS_TERMINATED=true behind a trusted HTTPS proxy.".into());
    }

    let database_url =
        env::var("AXON_DATABASE_URL").unwrap_or_else(|_| "sqlite://axon.db".to_owned());
    let storage = SqliteStorage::connect(&database_url).await?;
    let project_id = env::var("FIREBASE_PROJECT_ID")?;
    let verifier = FirebaseVerifier::new(project_id)?;
    let simulator_enabled = matches!(env::var("AXON_ENV").as_deref(), Ok("development"))
        && matches!(env::var("AXON_ENABLE_SIMULATOR").as_deref(), Ok("true"));
    let state = AppState::with_storage_and_auth(storage.shared(), verifier, simulator_enabled).await?;

    let listener = TcpListener::bind(address).await?;
    tracing::info!(
        %address,
        storage = "sqlite",
        "Axon API listening with Firebase authentication and SQLite storage"
    );
    axum::serve(listener, app(state).into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
