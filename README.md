# nerdtime

Terminal-native time tracking for developers. Offline-first CLI, optional cloud sync.

[License: AGPL-3.0-only](./LICENSE)

## Project status

| Product | Price | Status |
|---|---|---|
| CLI `nerd` | Free (AGPL) | ✅ Released |
| Cloud sync | $10/mo via [nerdtime.app](https://nerdtime.app) | ✅ Built |
| Self-hosted backend | Free (AGPL) | ✅ Built |
| [TUI](./spec/nerdtime-tui-plan.md) | Free (AGPL) | 📝 Planned |
| [Neovim plugin](./plugin/nerdtime-nvim/) | Free | 📝 Planned |
| [VS Code extension](./plugin/nerdtime-vscode/) | Free | 📝 Planned |
| [MCP server](./spec/nerdtime-mcp-server.md) | Free | 📝 Planned |
| [Tauri mobile app](./spec/tauri-mobile-app-plan.md) | iOS / Android / macOS / Linux | $10/mo | 📝 Planned |

## Quick start

```sh
# Build the CLI
cargo build --release -p nerd

# Start tracking
./target/release/nerd start my-project

# Stop
./target/release/nerd stop

# Show status
./target/release/nerd status

# Sync to backend
./target/release/nerd sync
```

Data is stored in `~/.config/nerdtime/data.db` (SQLite). Config is in `~/.config/nerdtime/config.toml`.

## Architecture

```
nerd/                 CLI client (Rust, clap + rusqlite + reqwest)
nerdtime-core/        Shared session / sync types (serde + chrono + uuid)
nerdtime-api/         Loco SaaS backend (Axum + SeaORM + PostgreSQL)
nerdtime-api/migration/ Database migrations
plugin/nerdtime-nvim/  Neovim plugin (Lua)
plugin/nerdtime-vscode/ VS Code extension (TypeScript)
spec/                  Implementation plans
```

## Target platforms

- **Desktop CLI**: Linux, macOS
- **Mobile**: iOS, Android (via Tauri)
- **Desktop app**: macOS, Linux (via Tauri)
- Not planned: Windows

## Pricing model

- **CLI + TUI + MCP server**: Free (AGPL). Your data stays local.
- **Cloud sync** ($10/mo): Sync sessions across devices, access history from anywhere.
- **Self-host**: Free (AGPL). Run your own backend — all features unlocked.
- **Mobile app**: Included with cloud sync subscription.

## Building from source

### CLI

```sh
cargo build --release -p nerd
```

### Backend (requires PostgreSQL + Redis)

```sh
make db-dev                    # start PostgreSQL
cargo run -p nerdtime-api      # dev server on port 5150
```

### Plugins

See `plugin/nerdtime-nvim/README.md` and `plugin/nerdtime-vscode/` for setup.

## Contributing

See [AGENTS.md](AGENTS.md) for architecture, conventions, and development workflow.

## License

AGPL-3.0-only. See [LICENSE](./LICENSE).
