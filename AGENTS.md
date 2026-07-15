# nerdtime.dev — Agent Guide

## Project

nerdtime is **quantified self for developers** — time tracking + commit intelligence + Eisenhower task prioritization + development logging + AI agent integration. One CLI, one SQLite database, optional cloud sync.

```
nerd/               # CLI client (clap + rusqlite + reqwest blocking)
nerdtime-core/      # shared Session / SyncPayload types (serde + chrono + uuid)
nerdtime-api/       # Loco SaaS backend (Axum + SeaORM + PostgreSQL)
nerdtime-api/migration/  # SeaORM migrations
spec/               # Implementation plans for all features
ROADMAP.md          # Product phases and priorities
DEVLOG.md           # Development history (append after every commit)
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

### Tasks & Eisenhower Matrix

- Tasks live in the CLI's SQLite (`data.db`) — not synced to backend (local-only concept)
- Each task has `urgency` (1-5) and `importance` (1-5); quadrant is computed: `(urgency > 3, importance > 3)` → Q1-Q4
- `--q1`/`--q2`/`--q3`/`--q4` shorthand flags map to (5/5, 1/5, 5/1, 1/1)
- `nerd what-should-i-work-on` uses a deterministic decision tree (time, energy, blockers) — no LLM
- See `spec/nerdtime-tasks.md`

### DEVLOG

- `nerd devlog` subcommand with `new`/`edit`/`query`/`list`/`generate`
- Post-commit hook auto-caches git data; user fills narrative via interactive prompt
- DEVLOG.md is generated from SQLite — never edited manually
- See `spec/nerdtime-devlog.md`

### SaaS / Self-Host Deployment

Same binary serves both models — no compile-time feature flags:

- **Multi-tenant SaaS**: every query filters by `user_id` (JWT claim), full Stripe billing gating. `BILLING_ENABLED=true` enables Stripe.
- **Single-tenant self-host**: `BILLING_ENABLED=false` (default) — all features free. `docker-compose.yml` for PostgreSQL + API behind Traefik with auto-TLS.
- CLI is offline-first; backend is optional for self-host.
- Per-user data isolation via `user_id` FK on `sessions` + `subscriptions` tables.

### MCP Server (Planned)

MCP server exposing nerdtime as tools for AI agents. 15 tools planned: sessions (6), tasks (5), devlog (3), what_should_i_work_on (1). See `spec/nerdtime-mcp-server.md`.

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
- **Log every commit in DEVLOG.md** — after each commit (or batch of related commits), append a brief entry to `DEVLOG.md` describing what changed and why. Stage, commit, and push separately.

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
