# Axon Desktop

Axon Desktop is the native desktop client, built with Tauri 2 (Rust host) and the shared React/Vite workspace.

## Development

Prerequisites: Node.js 20+, Rust stable, and the platform's Tauri prerequisites.

```sh
cd frontend/apps/desktop
npm install
npm run dev
```

Tauri starts the shared web application and embeds the Axon Axum API in the desktop process. The local API binds to `127.0.0.1:8080`; it is started before the webview loads and shut down gracefully when the desktop application exits.

## Build installers

```sh
cd frontend/apps/desktop
npm install
npm run build
```

Tauri produces native bundles for the build host. Windows builds require the Microsoft C++ Build Tools and WebView2; macOS builds require Xcode command-line tools; Linux builds require the Tauri WebKitGTK system dependencies.

## Security boundary

The embedded API binds only to loopback and allows browser requests only from the Tauri application origins and local Vite development origins. A second process already using port 8080 causes a clear startup error rather than silently connecting to an unrelated service.

The current API still uses process-local development storage and has no authentication or workspace authorization. No filesystem, shell, process, or credential-vault capability is exposed to the webview. Do not use this build with sensitive or production data. Durable storage, verified identity, route-level authorization, approved local file access, credential storage, tray integration, notifications, and signed updates remain explicit follow-on milestones.
