# matc-ui

Desktop GUI for the [Matter](https://csa-iot.org/developer-resource/specifications-download-request/) protocol controller library ([rust-matc](../rust-matc)).

Built with Tauri 2, Vue 3, and TypeScript.

## Features

- Discover commissionable devices via mDNS or BLE
- Commission devices by pairing code or IP address
- List commissioned devices and check reachability
- Browse full attribute tree (by endpoint and cluster)
- Send arbitrary commands to any device

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- Node.js >= 18
- Tauri CLI and platform dependencies: see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
- The `rust-matc` library checked out at `../rust-matc`

### macOS: BLE Matter commissioning

BLE commissioning requires the **Bluetooth Central Matter Client Developer Mode profile** (a `.mobileconfig` from the Apple Developer portal) to be installed on the Mac. Without it, macOS blocks the BLE Matter setup flow. Download it from your Apple Developer account and double-click to install via System Settings.

## Development

```bash
npm install
npm run tauri dev       # hot-reload frontend + debug Rust build
```

## Build

```bash
npm run tauri build
```

On macOS, to skip code signing for local builds:

```bash
APPLE_SIGNING_IDENTITY="" npm run tauri build
```

Output binary: `src-tauri/target/release/bundle/`

## Data directory

Matter state is stored in `~/Library/Application Support/com.matc.ui/matc/` and contains the fabric config, device registry, and certificates. Delete this directory to reset to factory state.

## Stack

| Layer | Technology |
|---|---|
| Desktop shell | Tauri 2 |
| Frontend | Vue 3 + TypeScript + Vite |
| UI components | Naive UI |
| State management | Pinia |
| Routing | Vue Router |
| Backend | Rust + rust-matc |
