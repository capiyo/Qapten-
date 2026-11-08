#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    net::TcpListener as StdTcpListener,
    sync::Mutex,
};
use tauri::{Manager, RunEvent};
use tokio::sync::oneshot;

struct ApiShutdown(Mutex<Option<oneshot::Sender<()>>>);

fn main() {
    let application = tauri::Builder::default()
        .setup(|app| {
            // Reserve the loopback port synchronously before the webview loads.
            // This prevents the UI from racing the embedded API startup.
            let listener = StdTcpListener::bind(("127.0.0.1", 8080)).map_err(|error| {
                Box::new(std::io::Error::new(
                    error.kind(),
                    format!(
                        "Axon could not start its local API on 127.0.0.1:8080: {error}. Close the process using that port before opening Axon."
                    ),
                )) as Box<dyn std::error::Error>
            })?;
            listener.set_nonblocking(true)?;

            let (shutdown_tx, shutdown_rx) = oneshot::channel();
            app.manage(ApiShutdown(Mutex::new(Some(shutdown_tx))));

            tauri::async_runtime::spawn(async move {
                let listener = tokio::net::TcpListener::from_std(listener)
                    .expect("the Axon API listener must be created inside the Tauri runtime");
                if let Err(error) = axum::serve(
                    listener,
                    axon_api::app(axon_api::AppState::default()),
                )
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                {
                    eprintln!("Axon local API stopped unexpectedly: {error}");
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to initialize Axon desktop");

    application.run(|handle, event| {
        if let RunEvent::Exit = event {
            if let Some(state) = handle.try_state::<ApiShutdown>() {
                if let Ok(mut sender) = state.0.lock() {
                    if let Some(sender) = sender.take() {
                        let _ = sender.send(());
                    }
                }
            }
        }
    });
}
