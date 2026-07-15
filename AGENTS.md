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
cargo run -p nerd -- heatmap [--days N] [--project P]      # week x hour contribution grid
cargo run -p nerd -- insights [--days N] [--project P]     # productivity patterns
cargo run -p nerd -- devlog new                            # interactive devlog entry
cargo run -p nerd -- devlog list [--limit N]               # list entries
cargo run -p nerd -- devlog query <text> [--tags T]        # search entries
cargo run -p nerd -- devlog edit <id>                      # edit entry in $EDITOR
cargo run -p nerd -- devlog generate                       # regenerate DEVLOG.md
cargo run -p nerd -- devlog show <id>                      # view single entry
cargo run -p nerd -- devlog cache-commit                   # cache commit (post-commit hook)
cargo run -p nerd -- task add <project> <title> [--q1/--q2/--q3/--q4] [--estimate 2h]  # create task
cargo run -p nerd -- task list [project]                   # list tasks
cargo run -p nerd -- task matrix [--project P]             # Eisenhower quadrant view
cargo run -p nerd -- task complete <id>                    # complete task
cargo run -p nerd -- task cancel <id>                      # cancel task
cargo run -p nerd -- task edit <id> [--title ...] [--estimate ...] [--q1]  # edit
cargo run -p nerd -- start <project> [--task T] [--estimate 2h] [--label L]  # start with task/labels
cargo run -p nerd -- estimate [task-id] [--project P]     # estimate vs actual
cargo run -p nerd -- summary [--days N] [--project P] [--label L]  # label aggregation
cargo run -p nerd -- what-should-i-work-on [--time 2h] [--energy high]  # advisor

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

# Billing env vars
BILLING_ENABLED=false
STRIPE_SECRET_KEY=
STRIPE_WEBHOOK_SECRET=
STRIPE_PRICE_ID=
```

## Architecture

- **CLI**: blocks on sync via `reqwest::blocking`; data in `~/.config/nerdtime/data.db` (SQLite); config in `~/.config/nerdtime/config.toml` (TOML with `api_url` + `token`)
- **CLI auto-detects** git branch and commit hash on `start` via `git branch --show-current` / `git rev-parse HEAD`
- **Backend binary**: `nerdtime_api-cli` (underscore, not hyphen)
- **Migrations**: auto-run on server start (`auto_migrate: true`). New migration files go in `nerdtime-api/migration/src/` with name `mYYYYMMDD_000000_name.rs` and must be registered in `lib.rs`
- **SeaORM entities** in `src/models/_entities/` (generated-style), companion models with business logic in `src/models/`
- **Controllers** export `pub fn routes() -> Routes` and are registered in `src/app.rs`
- **Auth**: JWT via `loco_rs::auth::jwt`; all API endpoints except `/api/health` and `/api/billing/webhook` require `auth::JWT` extractor
- **New SeaORM entities** need an `impl ActiveModelBehavior for ActiveModel` (can be empty) — required by `DeriveEntityModel` derive macro
- **Error helpers**: `Error::string(&format!(...))` for string errors. `Error::InternalServerError` is a unit variant (no payload). `bad_request()` / `unauthorized()` return `Result<Response>` — do not wrap in `.map_err()`

### Billing & Subscription Gating

- `settings.billing.enabled` toggle (`BILLING_ENABLED` env var); `false` = all features free, skip gating
- `BillingSettings::from_settings(&ctx.config.settings)` loads from config YAML `settings.billing.*` block
- `require_subscription()` helper in sync controller — no-op when billing disabled
- `subscriptions` table: `user_id` (FK), `stripe_customer_id`, `stripe_subscription_id`, `status`, `tier`, `current_period_end`
- `Model::is_active()` returns `true` for `free`, `active`, `trialing` statuses
- `find_or_create()` auto-creates a `free` tier row on first access

### SaaS / Self-Host Deployment

Same binary serves both models — no compile-time feature flags:

- **Multi-tenant SaaS**: every query filters by `user_id` (JWT claim), full Stripe billing gating. `BILLING_ENABLED=true` enables Stripe.
- **Single-tenant self-host**: `BILLING_ENABLED=false` (default) — all features free. `docker-compose.yml` for PostgreSQL + API behind Traefik with auto-TLS.
- CLI is offline-first; backend is optional for self-host.
- Per-user data isolation via `user_id` FK on `sessions` + `subscriptions` tables.

### MCP Server (Planned)

An MCP (Model Context Protocol) server exposing nerdtime CLI commands as tools is planned. It does not exist yet.

Proposed tools: `start_tracking`, `stop_tracking`, `get_status`, `list_sessions`, `get_stats`, `sync`.

Would use the same backing store (`~/.config/nerdtime/data.db`, SQLite) as the CLI. Could be a standalone binary or a `nerd mcp` subcommand.

## API Endpoints

| Endpoint | Method | Auth | Purpose |
|---|---|---|---|
| `/api/health` | GET | No | Health check |
| `/api/auth/register` | POST | No | User registration |
| `/api/auth/login` | POST | No | JWT login |
| `/api/auth/current` | GET | JWT | Current user profile |
| `/api/sync` | POST | JWT | Batch upsert sessions |
| `/api/sessions` | GET | JWT | List sessions (`?project=`, `?limit=`) |
| `/api/stats` | GET | JWT | Aggregate time per project |
| `/api/billing/checkout` | POST | JWT | Stripe Checkout Session |
| `/api/billing/webhook` | POST | No | Stripe webhook receiver (HMAC-signed) |
| `/api/billing/portal` | GET | JWT | Stripe Customer Portal redirect |
| `/api/billing/info` | GET | JWT | Current subscription tier/status |

Stripe integration uses raw `reqwest` calls to Stripe REST API (not the `stripe` crate). Webhook signature verification uses `hmac` + `sha2` + `hex`.

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

## Devlog Post-Commit Hook

To enable auto-caching of commit metadata (shown in `nerd devlog new` prompts):

```sh
git config core.hooksPath .githooks
```

Run once per clone. After this, every `git commit` calls `nerd devlog cache-commit` automatically.

## Quirks

- `include_dir!` **must** use `$CARGO_MANIFEST_DIR/` prefix (e.g. `include_dir!("$CARGO_MANIFEST_DIR/src/mailers/auth/welcome")`) — the macro resolves relative to CWD, not crate root
- `.rustfmt.toml` has `max_width = 100`
- `rusqlite` uses `bundled` feature (0.32.x) to stay compatible with `sqlx-sqlite`'s `libsqlite3-sys` v0.30 — do not upgrade independently
- The API defaults to port **5150**, binding `localhost` (dev) / `0.0.0.0` (prod)
- `DATABASE_URL` env var overrides the dev default `postgres://loco:loco@localhost:5432/nerdtime-api_development`
- CI requires both **PostgreSQL** and **Redis** services
- Snapshot tests use `insta`; fixtures in `src/fixtures/`
- When adding routes, add controller module to `src/controllers/mod.rs` + register `routes()` in `src/app.rs`
