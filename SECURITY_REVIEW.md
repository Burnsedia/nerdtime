# Security Review — nerdtime

Generated from source audit on 2026-07-16. Covers `nerd/`, `nerdtime-db/`, `nerdtime-api/`, and config files.

---

## Critical

| # | Area | Finding | File:Line |
|---|------|---------|-----------|
| C1 | **Command injection** | TUI launches editor via `sh -c "$EDITOR {id}"`. `$EDITOR` can contain shell metacharacters (`vim; curl ... \| sh`). Use `std::env::var("EDITOR")` + direct `Command::new()` instead. | `nerd/src/tui/app.rs:527-530` |
| C2 | **SQL injection** (fragile) | `list_tasks`, `edit_task`, `label_summary` use string interpolation with `format!("... '{}'", val.replace('\'', "''"))` instead of parameterized bindings. Technically safe in SQLite but inconsistent with rest of codebase — a maintenance risk that will regress. | `nerdtime-db/src/tasks.rs:82-96, 151-216`, `nerdtime-db/src/stats.rs:153-158` |
| C3 | **JWT token stored world-readable** | Config file at `~/.config/nerdtime/config.toml` written with default umask (typically `0644`). JWT token readable by any process on the system. No `set_permissions()` call, no keyring integration. | `nerd/src/config.rs:49-56` |
| C4 | **Hardcoded JWT secrets (dev)** | `config/development.yaml:107` and `docker-compose.dev.yml:19` contain literal JWT signing key `WqOAD0KPFoE8YgKw7Ok1`. Anyone with repo access can forge JWTs for dev/staging instances. | `nerdtime-api/config/development.yaml:107`, `docker-compose.dev.yml:19` |

## High

| # | Area | Finding | File:Line |
|---|------|---------|-----------|
| H1 | **Auth — No rate limiting** | No rate limiting on `/api/auth/login`, `/register`, `/forgot`, `/reset`, or `/magic-link`. Unlimited brute-force or password-reset abuse. | `controllers/auth.rs:47,99,121,138,182,231` |
| H2 | **Race — TOCTOU in subscription creation** | `find_or_create` checks for existing subscription then inserts — no unique constraint on `user_id` in the migrations. Two concurrent signups create duplicate subscription rows. | `models/subscriptions.rs:111-124`, `migration/m20260103_000001_subscriptions.rs:10-36` |
| H3 | **Race — TOCTOU in session upsert** | `upsert_sync` finds then inserts — no explicit PK on `id` in sessions table. Concurrent syncs could create duplicate rows. | `models/sessions.rs:20-53` |
| H4 | **Secrets in process memory** | Stripe keys, JWT secrets, user passwords held in plain `String` — not zeroed on drop, not using `secrecy`/`zeroize`. | Multiple: `nerd/src/auth.rs:49-50`, `nerd/src/main.rs:470,490,493,916`, `nerd/src/config.rs:9` |

## Medium

| # | Area | Finding | File:Line |
|---|------|---------|-----------|
| M1 | **No CORS configuration** | CORS middleware disabled in all config files. A browser-based frontend would be blocked from making API requests. If enabled, default config is `*` for origins/headers/methods. | `config/*.yaml` (all) |
| M2 | **No security headers** | No `Strict-Transport-Security`, `X-Content-Type-Options`, `X-Frame-Options`, `CSP` headers configured anywhere. | `src/app.rs:53-58` |
| M3 | **Input validation — Register/Login** | `RegisterParams` and `LoginParams` have no validation on `password` field. Empty or single-character passwords accepted. No password complexity requirements. | `models/users.rs:15-25` |
| M4 | **Input validation — SyncPayload** | `project_name`, `branch_name`, `commit_hash`, `description` are arbitrary unbounded strings with no length/character restrictions. Could be used for storage amplification. | `controllers/sync.rs:27-44` |
| M5 | **Stripe errors leak to caller** | Stripe API error messages (with request IDs, internal details) returned verbatim in HTTP responses. | `controllers/billing.rs:80,224` |
| M6 | **Subscription gating blocks free tier** | `tier != "free" && is_active()` — free-tier users always get `Unauthorized`. Error message is misleading ("active subscription required" when they have one). | `controllers/sync.rs:11-25` |
| M7 | **Email verification not required** | `can_login_without_verify` — users can log in without verifying email. Reduces value of email verification. | `controllers/auth.rs:138-160` |
| M8 | **No cargo-audit/deny in CI** | No dependency vulnerability scanning in CI. | `.github/workflows/ci.yml` |

## Low

| # | Area | Finding | File:Line |
|---|------|---------|-----------|
| L1 | **Login endpoint leaks registered emails** | Returns `401` for non-existent emails vs timing-safe `200` for forgot-password. | `controllers/auth.rs:139-145` |
| L2 | **Webhook error leaks verification details** | `"webhook verification failed: {reason}"` returned to caller — attacker learns why their forged request was rejected. | `controllers/billing.rs:105` |
| L3 | **JWT algorithm confusion not explicitly prevented** | `Validation::new(HS512)` but `alg` header is not explicitly restricted — algorithm confusion attack is theoretically possible. | `loco-rs jwt.rs:102-111` |
| L4 | **No password reset token expiration** | Password reset token is UUID v4 — no TTL/expiry enforced. | `models/users.rs:303` |
| L5 | **ListParams limit unbounded** | No upper bound on `?limit=` — attacker could request `?limit=999999999999` causing memory exhaustion. | `controllers/sync.rs:85-89` |
| L6 | **Config file path leaked in errors** | Absolute path to config file included in user-facing error messages. | `nerd/src/config.rs:43,45` |
| L7 | **Server response body leaked in errors** | Raw backend error responses included in CLI error messages. | `nerd/src/auth.rs:67-68,100,104` |
| L8 | **Editor spawn fire-and-forget** | `.spawn()` result discarded — no feedback on failure, no wait, no refresh. | `nerd/src/tui/app.rs:527-531` |
| L9 | **TOCTOU in DB path** | Race window between `create_dir_all` and `Connection::open` — exploitable only by local attacker with session access. | `nerdtime-db/src/connection.rs:10-15` |

---

## Summary

| Severity | Count |
|----------|-------|
| **Critical** | 4 |
| **High** | 4 |
| **Medium** | 8 |
| **Low** | 9 |
| **Total** | 25 |
| **No finding** (confirmed clean) | 8 areas (git calls, gh calls, path traversal, TOML deserialization, git hook, SQLi via SeaORM, webhook HMAC, dependency versions) |

## Priority Remediation Order

1. **C1** — TUI shell injection (`$EDITOR` → direct exec)
2. **C4** — Hardcoded JWT secrets (env var references)
3. **C2** — SQL interpolation (migrate to `params![]`)
4. **C3** — Token world-readable (set `0o600` on config file)
5. **H1** — No rate limiting (add middleware)
6. **H2/H3** — TOCTOU races (add unique constraints, use transactions)
7. **H4** — Memory scrubbing (add `secrecy`/`zeroize`)
8. **M1-M8** — CORS, headers, input validation, error handling
