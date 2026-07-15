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
