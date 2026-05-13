# Development

## Stack

| Layer | Technology |
|---|---|
| Desktop shell | Tauri 2 |
| Frontend | Vue 3 + TypeScript + Vite |
| UI components | Naive UI |
| State management | Pinia |
| Routing | Vue Router |
| Backend | Rust + rust-matc |

## Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- Node.js >= 18
- Tauri CLI and platform dependencies: see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
- The `rust-matc` library checked out at `../rust-matc`

## Running in development

```bash
npm install
npm run tauri dev       # hot-reload frontend + debug Rust build
```

## Building a release binary

```bash
npm run tauri build
```

On macOS, to skip code signing for local builds:

```bash
APPLE_SIGNING_IDENTITY="" npm run tauri build
```

Output: `src-tauri/target/release/bundle/`

## Versioning

Version is stored in three files that must stay in sync:
- `package.json` -> `version`
- `src-tauri/tauri.conf.json` -> `version`
- `src-tauri/Cargo.toml` -> `version`

Use the bump script to update all three atomically:

```bash
npm run version:set -- 0.2.0
```

### Cutting a release

```bash
npm run version:set -- 0.2.0
git commit -am "release v0.2.0"
git tag v0.2.0
git push origin master v0.2.0
```

Pushing the tag triggers `.github/workflows/release.yml`, which builds on macOS, Linux, and Windows and creates a draft GitHub Release. Review and publish the draft on GitHub.

## Project structure

See [CLAUDE.md](CLAUDE.md) for the full project structure, Tauri command reference, and backend architecture notes.
