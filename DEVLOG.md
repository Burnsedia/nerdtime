# nerdtime.dev — Development Log

## 2026-07-14: Initial Build Session

### Project Inception

Started with a PDF specification describing a full-stack Rust time tracker: a lightning-fast CLI writing to local SQLite, paired with a multi-tenant Loco web framework backend on PostgreSQL behind Traefik.

PDF was extracted via `pdftotext` and analyzed. The spec outlined two main components:

- **`nerd` CLI** — `clap` + `rusqlite`, offline-first session storage
- **Loco Backend** — Axum + SeaORM + PostgreSQL, JWT auth, sync endpoint

### Tooling

- Rust 1.97.0 (Arch Linux)
- `loco-cli` was deprecated mid-session; migrated to `loco` v0.16.3
- Cargo workspace with 4 members: `nerd`, `nerdtime-core`, `nerdtime-api`, `nerdtime-api/migration`

### Workspace & Core Types

Created `nerdtime-core` as a shared types crate holding `Session` and `SyncPayload` structs. Both serialize via serde — JSON for the API, string columns in SQLite.

**Quirk discovered:** The workspace caused a `libsqlite3-sys` conflict between `rusqlite` (v0.34, bundled feature → libsqlite3-sys v0.32) and `sqlx-sqlite` (transitive from loco-rs → libsqlite3-sys v0.30). Both link the same native library. Fixed by pinning `rusqlite` to 0.32 with `bundled` (uses libsqlite3-sys 0.30, matching sqlx-sqlite). Also dropped `sqlx-sqlite` feature from `sea-orm` since the backend uses PostgreSQL only.

### CLI Client (`nerd`)

Built with clap derive. Seven commands:

| Command | Purpose |
|---------|---------|
| `start` | Insert session with UUID + ISO timestamp, auto-detect git branch/commit |
| `stop` | Set `ended_at`, print duration |
| `status` | Show active session with elapsed time |
| `log` | List sessions with optional `--project` and `--limit` |
| `sync` | Batch POST unsynced sessions to API |
| `login` | Store JWT + API URL in config |
| `config` | Show current configuration |

Data lives in `~/.config/nerdtime/data.db` (SQLite). Config in `~/.config/nerdtime/config.toml` (TOML).

Git detection runs `git branch --show-current` and `git rev-parse HEAD` on every `start` — fails silently if not in a git repo.

### Loco Backend (`nerdtime-api`)

Generated with `loco new --template saas --db postgres`. The SaaS starter includes a `users` table and JWT authentication out of the box.

**Migration:** Added `m20260101_000001_sessions` with `id` (UUID PK), `user_id` (FK to users), `project_name`, `branch_name`, `commit_hash`, `description`, `started_at`, `ended_at`.

**Controllers:**

| Endpoint | Method | Auth | Purpose |
|----------|--------|------|---------|
| `/api/health` | GET | No | Health check |
| `/api/auth/register` | POST | No | User registration |
| `/api/auth/login` | POST | No | JWT login |
| `/api/auth/current` | GET | JWT | Current user profile |
| `/api/sync` | POST | JWT | Batch upsert sessions |
| `/api/sessions` | GET | JWT | List sessions (optional `?project=`, `?limit=`) |
| `/api/stats` | GET | JWT | Aggregate time per project |

**Routes registered in `src/app.rs`** via the `routes()` pattern — each controller exports `pub fn routes() -> Routes`.

**Quirks:**
- The binary is named `nerdtime_api-cli` (underscore, not hyphen)
- Backend binary is at `src/bin/main.rs`, entrypoint calls `cli::main::<App, Migrator>`
- `include_dir!` macro resolves relative to CWD, not crate root — all mailer paths must use `$CARGO_MANIFEST_DIR/` prefix
- `.rustfmt.toml` sets `max_width = 100`
- Migrations auto-run on server start (`auto_migrate: true`)
- Port defaults to 5150 in dev, 3000 in production

### Error Handling Patterns

Used `return unauthorized("msg")` pattern from Loco's controller helpers (returns `Err(Error::Unauthorized(...))`). No `internal_server` helper exists — use `Error::InternalServerError` directly.

The `ActiveModelBehavior::before_save` is async in SeaORM 1.1, requiring `#[async_trait::async_trait]` impl.

### Deployment

**Docker:** Multi-stage Dockerfile (builder → slim runtime). Production docker-compose with Traefik + PostgreSQL + API, automatic TLS via Let's Encrypt.

**Development:** Dev docker-compose with PostgreSQL + API hot-reload via `cargo-watch`. Makefile wrapping common commands:

```
make build-cli       cargo build --release -p nerd
make build-api       cargo build --release -p nerdtime-api
make dev-api         cd nerdtime-api && cargo start
make db-dev          docker compose -f docker-compose.dev.yml up -d postgres
make test            cargo test --workspace
```

### Git Conventions

Established conventions during the session:

- **Branch per feature** — kebab-case with prefix (`feat/`, `fix/`, `refactor/`, `chore/`, `docs/`)
- **Logical commits** — never mix unrelated changes
- **Conventional commit messages** — `feat:`, `fix:`, `chore:`, `docs:`, `refactor:`
- **No force-push** or amending pushed commits
- **Review before commit** — `git status`, `git diff`, `git log --oneline -5`

### Documentation

Created `AGENTS.md` as a compact instruction file for future OpenCode sessions covering commands, architecture, git workflow, licensing, and framework quirks.

Regular READMEs written for:
- Root — project overview, architecture diagram, quick start
- `nerd/` — CLI user guide with all command examples
- `nerdtime-api/` — API reference, endpoints, env vars, deployment
- `nerdtime-core/` — shared types documentation

### Licensing

Project licensed under **AGPL-3.0-only**. Applied across all 39 `.rs` files (`// SPDX-License-Identifier: AGPL-3.0-only` header) and all 5 `Cargo.toml` manifests.

### Chat Decisions & Architectural Tradeoffs

#### GitHub OAuth (Planned)

Discussed adding GitHub OAuth login. Research found:

- Loco 0.16 has **no built-in OAuth2** support — only JWT auth
- The `loco-oauth2` crate (v0.5) is the recommended approach, implementing the OAuth2 initializer pattern Loco's config system supports (commented-out `initializers.oauth2` block in `development.yaml`)
- Decision: use `loco-oauth2` with `axum_session` backend, JWT cookie callback strategy (redirect browser with cookie set, matching web dashboard UX)
- Would require: 4 new files (2 initializers, 1 controller, 1 migration), 8 modified files, `GITHUB_CLIENT_ID`/`GITHUB_CLIENT_SECRET` env vars

#### SaaS vs Self-Host

Analyzed how the current architecture supports both models:

- **Multi-tenant SaaS** works out of the box — every session has `user_id`, queries always filter by user, built-in JWT auth
- **Single-tenant self-host** also works — CLI stores configurable API URL, Docker compose for PostgreSQL + API, offline-first CLI needs no backend at all
- Key insight: the **same binary** serves both models, no vendor lock-in
- Planned improvements: `nerd init` interactive setup, health check before sync, simplified self-host compose

#### Documentation Philosophy for AGENTS.md

Every line must answer: "Would an agent likely miss this without help?" Excluded: generic advice, obvious language conventions, exhaustive file trees, speculative claims. Included: exact commands, framework quirks, version locks, CI requirements, route registration pattern.

---

*Session ended with project on `master` branch, 17 commits across 2 branches, full workspace compiling cleanly, AGPL-3.0-only license applied.*

---

## 2026-07-14: Open Core + Paid Sync Billing

### Business Model Decision

Chose **Open Core + Paid Sync** (Model 2 of the spec): the CLI is free and AGPL-licensed; cloud sync is the paid feature at ~$4/mo via Stripe subscriptions. Self-host deployments get everything free by setting `BILLING_ENABLED=false`.

### Subscriptions Table & Migration

Added `m20260103_000001_subscriptions` migration creating the `subscriptions` table:

| Column | Type | Purpose |
|--------|------|---------|
| `id` | PK (auto) | Primary key |
| `user_id` | UUID (FK → users) | Owner |
| `stripe_customer_id` | String? | Stripe customer reference |
| `stripe_subscription_id` | String? | Stripe subscription reference |
| `status` | String | `active`, `trialing`, `past_due`, `canceled`, `incomplete` |
| `tier` | String | `free` or `pro` |
| `current_period_end` | TimestampTz? | Subscription period end |

SeaORM entity in `_entities/subscriptions.rs` with `Relation::User` (belongs_to). Companion model in `models/subscriptions.rs`:

- `BillingSettings` — deserialized from `config.settings.billing.*`; provides `from_settings()` and `Default` (billing off)
- `Model::find_or_create()` — auto-creates `free` tier row on first access
- `Model::is_active()` — `active`, `trialing`, or `free` counts as active
- `Model::update_stripe()` — webhook handler that sets customer/subscription IDs and upgrades to `pro`
- `Model::set_tier()` — manual tier change (e.g., subscription canceled → revert to `free`)

**Quirk:** New SeaORM entities require `impl ActiveModelBehavior for ActiveModel` (can be empty body) — `DeriveEntityModel` derive macro enforces it.

### Billing Controller

`src/controllers/billing.rs` with 4 endpoints:

| Endpoint | Auth | Purpose |
|----------|------|---------|
| `POST /api/billing/checkout` | JWT | Creates Stripe Checkout Session, returns URL for redirect |
| `POST /api/billing/webhook` | None (HMAC) | Stripe event receiver — verifies HMAC-SHA256 signature, handles `checkout.session.completed`, `customer.subscription.*` |
| `GET /api/billing/portal` | JWT | Creates Stripe Customer Portal session, returns redirect URL |
| `GET /api/billing/info` | JWT | Returns current tier, status, `is_active` for the user |

**Stripe integration approach:** Uses raw `reqwest` calls to Stripe REST API (not the `stripe` crate). Reason: simpler dependency management, more transparent error handling, full control over API version (`2025-02-24.acacia`).

**Webhook security:** HMAC-SHA256 signature verification via `hmac` + `sha2` + `hex` crates (not the `stripe` crate). Parses `stripe-signature` header for `t=` timestamp and `v1=` signature, computes expected HMAC, compares.

**Quirk:** `Error::InternalServerError` is a **unit variant** (no data payload). For string errors, use `Error::string(&format!(...))`. The `bad_request()` and `unauthorized()` helpers return `Result<Response>` directly — do not wrap in `.map_err()`.

### Subscription Gating

`src/controllers/sync.rs` now has a `require_subscription()` helper called at the start of `sync_sessions()`, `list_sessions()`, and `get_stats()`:

1. Loads `BillingSettings` from config
2. If `enabled == false`, returns `Ok(())` (skip gating)
3. If enabled, looks up user's subscription via `find_or_create()`
4. Checks `is_active()` — returns `Unauthorized` if not active

This means `health` and all `auth` endpoints remain free when billing is on. The `/api/billing/webhook` endpoint is also free (unauthenticated, HMAC-signed).

### SaaS vs Self-Host Architecture

Documented in AGENTS.md:

- **Same binary** (`nerdtime-api`) serves both models — no compile-time feature flags
- **Multi-tenant SaaS**: `BILLING_ENABLED=true`, JWT per-user data isolation via `user_id` FK on all tables, full Stripe integration
- **Single-tenant self-host**: `BILLING_ENABLED=false` (default), docker-compose with PostgreSQL + API + Traefik, CLI offline-first needs no backend
- CLI stores configurable `api_url` — no hardcoded endpoint

### MCP Server (Planned)

An MCP (Model Context Protocol) server was designed but not yet implemented. Proposed tools matching CLI commands:

- `start_tracking` (project, description?)
- `stop_tracking` → returns duration
- `get_status` → active session with elapsed time
- `list_sessions` (project?, limit=10)
- `get_stats` → aggregate time per project
- `sync` → push unsynced sessions to backend

Would use the same SQLite backing store (`~/.config/nerdtime/data.db`) as the CLI. Could ship as a standalone binary or a new `nerd mcp` subcommand.

### Compile Errors Encountered

During development of `billing.rs`, three errors needed fixing:

1. **`Error::string(&format!(...))`** — The `&` is required because `Error::string` takes `&str`, not `String` (format!() returns String).
2. **`serde_json::map_err(|_| bad_request(...))?`** — `bad_request()` returns `Result<Response>`, so `.map_err()` inside a `Result::map_err` wraps it as `Result<Response, Error>`, which `?` can't convert. Fixed by using `Error::string(...)` directly.
3. **Handler trait not satisfied** — `webhook` handler with `(State, Bytes, HeaderMap)` extractor tuple wasn't recognized. Switched from `Bytes` to `String` body extractor, which resolved it.

### Files Changed

```
NEW: nerdtime-api/migration/src/m20260103_000001_subscriptions.rs
NEW: nerdtime-api/src/models/_entities/subscriptions.rs
NEW: nerdtime-api/src/models/subscriptions.rs
NEW: nerdtime-api/src/controllers/billing.rs
MOD: nerdtime-api/Cargo.toml                     (+reqwest, hmac, sha2, hex)
MOD: nerdtime-api/config/development.yaml         (+settings.billing block)
MOD: nerdtime-api/config/production.yaml          (+settings.billing block)
MOD: nerdtime-api/migration/src/lib.rs             (+subscriptions migration)
MOD: nerdtime-api/src/app.rs                       (+billing routes)
MOD: nerdtime-api/src/controllers/mod.rs            (+billing module)
MOD: nerdtime-api/src/controllers/sync.rs           (+subscription gating)
MOD: nerdtime-api/src/models/_entities/mod.rs       (+subscriptions entity)
MOD: nerdtime-api/src/models/_entities/prelude.rs   (+Subscriptions re-export)
MOD: nerdtime-api/src/models/mod.rs                 (+subscriptions module)
MOD: AGENTS.md                                      (comprehensive update)
MOD: DEVLOG.md                                      (this entry)
```

*Session ended with workspace compiling cleanly (`cargo build --workspace`), clippy clean (4 pre-existing warnings), rustfmt clean. 17 existing commits + uncommitted changes above.*

### MVP Gap Fixes

After reviewing what was actually needed to ship the MVP, three issues were fixed:

1. **Wrong webhook lookup for non-checkout events** — `customer.subscription.updated` and `.created` events (which carry a `stripe_customer_id` like `"cus_xxx"` but no `client_reference_id`) were calling `users::Model::find_by_pid()` which expects a UUID. This silently failed for every non-checkout Stripe event. Fixed by adding `subscriptions::Model::find_by_stripe_customer_id()` that queries the subscriptions table directly, and using it in the webhook handler.

2. **`customer.subscription.deleted` was a no-op** — The handler existed but only had a TODO comment. Now it looks up the subscription by `stripe_customer_id` and calls `set_tier(db, user_id, "free", "canceled")` to downgrade the user.

3. **Missing index on `stripe_customer_id`** — The migration had no index on the column used for webhook lookups. Added `idx_subscriptions_stripe_customer_id` index via SeaORM's `Index::create()` API in the existing migration.

**Files changed:** `src/models/subscriptions.rs` (new method), `src/controllers/billing.rs` (webhook fix), `migration/src/m20260103_000001_subscriptions.rs` (index).

### Critical Gating Bug Fix

`require_subscription()` in `sync.rs` was calling `sub.is_active()`, which returns `true` for `tier == "free"` (by design — free tier is always "active"). Combined with `find_or_create()` auto-creating a free row for every user, this meant **billing gating was completely non-functional** — every user passed the check regardless of billing status.

Fix: changed the condition to `sub.tier != "free" && sub.is_active()`. When billing is enabled, free-tier users are now correctly rejected and must upgrade to proceed.

### CLI Sync UX Improvement

Previously, sync failures showed a cryptic raw status code (`"Sync failed with status: 403"`). Now 401/403 responses print a user-friendly message with a link to the upgrade page:
```
Sync rejected (403). An active subscription is required. Visit https://nerdtime.dev/settings to upgrade.
```

---

*Session ended with workspace compiling cleanly (`cargo build --workspace`), fmt clean, clippy clean (4 pre-existing warnings). 17 existing commits + all changes above.*
