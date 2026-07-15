# nerdtime.dev — Agent Guide

## Project

Two binaries, one shared types crate. CLI is offline-first; backend is a Loco API.

```
nerd/               # CLI client (clap + rusqlite + reqwest blocking)
nerdtime-core/      # shared Session / SyncPayload types (serde + chrono + uuid)
nerdtime-api/       # Loco SaaS backend (Axum + SeaORM + PostgreSQL)
nerdtime-api/migration/  # SeaORM migrations
```

## Commands

```sh
# CLI
cargo run -p nerd -- start <project>         # start tracking
cargo run -p nerd -- stop                    # stop active session
cargo run -p nerd -- status                  # show active session
cargo run -p nerd -- sync                    # push to backend
cargo run -p nerd -- login <token>           # store JWT + API URL

# Backend
cargo run -p nerdtime-api -- start           # run dev server (port 5150)
cd nerdtime-api && cargo start               # same, via .cargo/config.toml alias

# Build (release)
cargo build --release -p nerd
cargo build --release -p nerdtime-api

# Test (needs PostgreSQL running)
make db-dev              # docker compose -f docker-compose.dev.yml up -d postgres
cargo test --workspace   # CI also needs Redis

# Lint
cargo fmt --all -- --check
cargo clippy --all-features -- -D warnings
```

## Architecture

- **CLI**: blocks on sync via `reqwest::blocking`; data in `~/.config/nerdtime/data.db` (SQLite); config in `~/.config/nerdtime/config.toml` (TOML with `api_url` + `token`)
- **CLI auto-detects** git branch and commit hash on `start` via `git branch --show-current` / `git rev-parse HEAD`
- **Backend binary**: `nerdtime_api-cli` (underscore, not hyphen)
- **Migrations**: auto-run on server start (`auto_migrate: true`). New migration files go in `nerdtime-api/migration/src/` with name `mYYYYMMDD_000000_name.rs` and must be registered in `lib.rs`
- **SeaORM entities** in `src/models/_entities/` (generated-style), companion models with business logic in `src/models/`
- **Controllers** export `pub fn routes() -> Routes` and are registered in `src/app.rs`
- **Auth**: JWT via `loco_rs::auth::jwt`; all API endpoints except `/api/health` require `auth::JWT` extractor

## Git

- **Branch per feature** — create a new branch for each logical change (e.g. `feat/github-oauth`, `fix/sync-timeout`, `refactor/cli-args`). Never commit unrelated changes on the same branch.
- **Branch naming convention** — use kebab-case with conventional prefix: `feat/`, `fix/`, `refactor/`, `chore/`, `docs/`.
- **Commit logically** — never lump unrelated changes into one commit. Separate concerns (e.g. CLI changes → one commit, backend → another, Docker → another).
- **Write meaningful messages** — use conventional commits (`feat:`, `fix:`, `chore:`, `docs:`, `refactor:`) followed by a brief description of what and why.
- **Never force-push** or amend pushed commits.
- **Review before commit** — run `git status`, `git diff`, and `git log --oneline -5` first. Only stage intended files.

## Licensing

- All new source files **must** include `// SPDX-License-Identifier: AGPL-3.0-only` as the first line.
- All `Cargo.toml` manifests must have `license = "AGPL-3.0-only"`.

## Quirks

- `include_dir!` **must** use `$CARGO_MANIFEST_DIR/` prefix (e.g. `include_dir!("$CARGO_MANIFEST_DIR/src/mailers/auth/welcome")`) — the macro resolves relative to CWD, not crate root
- `.rustfmt.toml` has `max_width = 100`
- `rusqlite` uses `bundled` feature (0.32.x) to stay compatible with `sqlx-sqlite`'s `libsqlite3-sys` v0.30 — do not upgrade independently
- The API defaults to port **5150**, binding `localhost` (dev) / `0.0.0.0` (prod)
- `DATABASE_URL` env var overrides the dev default `postgres://loco:loco@localhost:5432/nerdtime-api_development`
- CI requires both **PostgreSQL** and **Redis** services
- Snapshot tests use `insta`; fixtures in `src/fixtures/`
- When adding routes, add controller module to `src/controllers/mod.rs` + register `routes()` in `src/app.rs`
