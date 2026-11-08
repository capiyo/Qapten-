# Axon Desktop

Axon Desktop is the native desktop client, built with Tauri 2 (Rust host) and the shared React/Vite web application. It is a first-class client, not a separate product or a fork of the web UI.

## Development

Prerequisites: Node.js 20+, Rust stable, and the platform's Tauri prerequisites.

```sh
cd frontend/apps/web
npm install
cd ../desktop
npm install
npm run dev
```

The Tauri shell starts the shared web application in its native window. The API remains the shared Axon backend at `127.0.0.1:8080`.

## Build installers

```sh
cd frontend/apps/web && npm install
cd ../desktop && npm install && npm run build
```

Tauri produces native bundles for the build host. Windows builds require the Microsoft C++ Build Tools and WebView2; macOS builds require Xcode command-line tools; Linux builds require the Tauri WebKitGTK system dependencies.

## Security boundary

The initial shell deliberately exposes no filesystem, shell, process, or unrestricted network capabilities to the webview. Add a narrowly scoped Rust command and capability only alongside a user-facing permission flow and tests. Local file access, credential storage, background tasks, tray integration, notifications, and signed updates are planned desktop milestones, not yet implemented.
