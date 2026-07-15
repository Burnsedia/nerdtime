# GitHub OAuth — Implementation Plan

## Files to create (4)

| File | Purpose |
|------|---------|
| `nerdtime-api/src/initializers/oauth2.rs` | OAuth2 initializer reads GitHub config from YAML, registers `OAuth2ClientStore` |
| `nerdtime-api/src/controllers/oauth2.rs` | Two routes: `GET /api/oauth2/github` (redirect to GitHub) and `GET /api/oauth2/github/callback` (handle code → token → user lookup/create → JWT) |
| `nerdtime-api/src/models/o_auth2_sessions.rs` | Model for `o_auth2_sessions` table (links OAuth session to user) |
| `nerdtime-api/src/views/auth.rs` | `LoginResponse` view struct (JWT + user info) — may already exist from Loco template |

## Files to modify (8)

| File | Change |
|------|--------|
| `nerdtime-api/Cargo.toml` | Add `loco-oauth2 = { version = "0.5", features = ["axum_session"] }` |
| `nerdtime-api/migration/Cargo.toml` | Add `loco-oauth2 = { workspace = true }` |
| `nerdtime-api/migration/src/lib.rs` | Register `migration::m20240101_000000_oauth2_sessions::Migration` |
| `nerdtime-api/src/app.rs` | Add `initializers::oauth2::OAuth2StoreInitializer` to `initializers()` vec |
| `nerdtime-api/src/initializers/mod.rs` | Add `pub mod oauth2;` |
| `nerdtime-api/src/controllers/mod.rs` | Add `pub mod oauth2;` |
| `nerdtime-api/src/models/mod.rs` | Add `pub mod o_auth2_sessions;` |
| `nerdtime-api/config/development.yaml` | Add `initializers.oauth2` block with GitHub client_id, client_secret, redirect_uri |

## Dependencies

- `loco-oauth2` (0.5, feature `axum_session`) — community crate, 22 stars, MIT license
- Brings in `oauth2` crate (generic OAuth2 client), `axum_session` for session storage
- Requires `axum_session` SQLite feature in migration's Cargo.toml (or PostgreSQL — need to check compat with our SeaORM PG setup)

## Migration

`loco-oauth2` provides `migration::m20240101_000000_oauth2_sessions::Migration` — creates `o_auth2_sessions` table:

| Column | Type | Notes |
|--------|------|-------|
| `id` | i32 (PK) | Auto |
| `session_id` | String | OAuth2 session identifier |
| `user_id` | i32 | FK → users.id |
| `expires_at` | Timestamp | Session expiry |
| `created_at` | Timestamp | Auto |
| `updated_at` | Timestamp | Auto |

## Flow

1. User clicks "Login with GitHub" → frontend calls `GET /api/oauth2/github`
2. Backend generates OAuth2 authorization URL with CSRF token, stores session, redirects to GitHub
3. User authorizes on GitHub.com → GitHub redirects to `GET /api/oauth2/github/callback?code=...&state=...`
4. Backend verifies CSRF token, exchanges code for access token, fetches user profile from GitHub API
5. Backend looks up user by GitHub email (or creates new user), generates JWT
6. Returns `LoginResponse` with JWT (same shape as email/password login)

## YAML config shape

```yaml
initializers:
  oauth2:
    authorization_code:
      - client_identifier: github
        client_id: {{ get_env(name="GITHUB_CLIENT_ID") }}
        client_secret: {{ get_env(name="GITHUB_CLIENT_SECRET") }}
        redirect_uri: "http://localhost:5150/api/oauth2/github/callback"
        auth_url: "https://github.com/login/oauth/authorize"
        token_url: "https://github.com/login/oauth/access_token"
        user_info_url: "https://api.github.com/user"
        scopes: ["read:user", "user:email"]
        token_response_key: access_token
```

## Env vars to add

- `GITHUB_CLIENT_ID`
- `GITHUB_CLIENT_SECRET`

Add both to `.env.example` and `config/production.yaml`.

## Quirks / Risks

- `loco-oauth2` v0.5 had a security advisory for docs prior to v0.4.1 — ensure we pin `>=0.4.1`
- The crate's Google boilerplate won't work directly for GitHub; GitHub uses different endpoints and returns a different user profile shape. We'll need to write the GitHub callback handler manually (the crate provides the building blocks but not provider-specific handlers for GitHub).
- `axum_session` uses SQLite by default — will need to configure it for PostgreSQL since our backend is PG-only
- Session table migration from the crate uses `user_id INTEGER` — our `users` PK is also `id i32`, so FK should match
- The `token_response_key` for GitHub's token endpoint returns JSON `{ access_token: "...", token_type: "...", scope: "..." }` — need to set this to `access_token`
- GitHub's user info endpoint returns `{ login, id, avatar_url, email, name, ... }` — we need to extract the verified email. If the user hasn't made their email public, we may need to call the `/user/emails` endpoint separately.
- For `axum_session`, we need the `sqlx-postgres` feature enabled, not the default `sqlx-sqlite`
