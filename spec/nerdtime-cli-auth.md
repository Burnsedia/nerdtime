# nerdtime CLI Auth — Implementation Plan

> **Status**: ✅ Implemented (login/signup/logout in `nerd/src/auth.rs`, interactive `rpassword` prompts)
> 
## Overview

Add interactive login/signup/logout to the `nerd` CLI. Currently the CLI accepts a pre-existing JWT via `nerd login <token>` with no way to obtain one. This plan adds prompts that call the backend's `/api/auth/login` and `/api/auth/register` endpoints directly.

## Commands

### `nerd login` (interactive)

```
$ nerd login
Email: user@example.com
Password:
✓ Logged in as user@example.com
```

Calls `POST /api/auth/login` with `{ email, password }`. On 200, saves the JWT token and user email to `config.toml`.

### `nerd login <token>` (headless, existing behavior)

```
$ nerd login eyJhbGciOiJIUzI1NiIs...
✓ Authentication saved for http://localhost:3000/api
```

Unchanged — kept for CI/headless users.

### `nerd signup`

```
$ nerd signup
Email: user@example.com
Name: Your Name
Password:
Confirm password:
✓ Registered! You're logged in.
```

Calls `POST /api/auth/register` with `{ email, password, name }`. On success, immediately logs in (or the register endpoint returns a JWT — depends on backend implementation).

### `nerd logout`

```
$ nerd logout
✓ Logged out.
```

Clears `token` and `user_email` from config.

### Password input

Uses `rpassword` crate to read password without terminal echo:

```rust
let password = rpassword::read_password_from_tty(Some("Password: "))?;
```

## Backend API contract

### `POST /api/auth/login`

Request:
```json
{ "email": "user@example.com", "password": "secret123" }
```

Response (200):
```json
{
  "token": "eyJhbGciOiJIUzI1NiIs...",
  "user": { "id": 1, "email": "user@example.com", "name": "Your Name" }
}
```

Error (401):
```json
{ "error": "Invalid email or password" }
```

### `POST /api/auth/register`

Request:
```json
{ "email": "user@example.com", "password": "secret123", "name": "Your Name" }
```

Response (200):
```json
{
  "token": "eyJhbGciOiJIUzI1NiIs...",
  "user": { "id": 1, "email": "user@example.com", "name": "Your Name" }
}
```

Error (422):
```json
{ "error": "Email already registered" }
```

## New files

```
nerd/src/
├── auth.rs        # NEW: login_interactive(), signup(), logout()
├── config.rs      # + user_email field
├── main.rs        # Change Login to accept optional token, add Signup/Logout
```

### `nerd/src/auth.rs`

```rust
use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use crate::config;

#[derive(Serialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct RegisterRequest {
    email: String,
    password: String,
    name: String,
}

#[derive(Deserialize)]
struct AuthResponse {
    token: String,
    user: UserInfo,
}

#[derive(Deserialize)]
struct UserInfo {
    #[serde(default)]
    id: i64,
    email: String,
    #[serde(default)]
    name: String,
}

pub fn login(email: &str, password: &str) -> Result<()> {
    let cfg = config::load()?;
    let url = format!("{}/auth/login", cfg.api_url.trim_end_matches('/'));

    let client = Client::new();
    let resp = client
        .post(&url)
        .json(&LoginRequest { email: email.to_string(), password: password.to_string() })
        .send()
        .context("Failed to connect to server")?;

    if resp.status().is_success() {
        let body: AuthResponse = resp.json()?;
        let mut cfg = config::load()?;
        cfg.token = Some(body.token);
        cfg.user_email = Some(body.user.email);
        config::save(&cfg)?;
        println!("✓ Logged in as {}", body.user.email);
        Ok(())
    } else if resp.status().as_u16() == 401 {
        anyhow::bail!("Invalid email or password.");
    } else {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        anyhow::bail!("Login failed ({}): {}", status, text);
    }
}

pub fn signup(email: &str, password: &str, name: &str) -> Result<()> {
    let cfg = config::load()?;
    let url = format!("{}/auth/register", cfg.api_url.trim_end_matches('/'));

    let client = Client::new();
    let resp = client
        .post(&url)
        .json(&RegisterRequest {
            email: email.to_string(),
            password: password.to_string(),
            name: name.to_string(),
        })
        .send()
        .context("Failed to connect to server")?;

    if resp.status().is_success() {
        let body: AuthResponse = resp.json()?;
        let mut cfg = config::load()?;
        cfg.token = Some(body.token);
        cfg.user_email = Some(body.user.email);
        config::save(&cfg)?;
        println!("✓ Registered! Logged in as {}", body.user.email);
        Ok(())
    } else if resp.status().as_u16() == 422 {
        let text = resp.text().unwrap_or_default();
        anyhow::bail!("Registration failed: {}", text);
    } else {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        anyhow::bail!("Registration failed ({}): {}", status, text);
    }
}

pub fn logout() -> Result<()> {
    let mut cfg = config::load()?;
    cfg.token = None;
    cfg.user_email = None;
    config::save(&cfg)?;
    println!("✓ Logged out.");
    Ok(())
}
```

## Changes to `config.rs`

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub api_url: String,
    pub token: Option<String>,
    pub user_email: Option<String>,  // NEW
}
```

## Changes to `main.rs`

```rust
enum Commands {
    Login {
        token: Option<String>,  // was: String
        #[arg(short, long)]
        url: Option<String>,
    },
    Signup,     // NEW
    Logout,     // NEW
}
```

Dispatch:

```rust
Commands::Login { token: Some(t), url } => login_headless(url.as_deref(), t),
Commands::Login { token: None, url } => login_interactive(url.as_deref()),
Commands::Signup => signup_interactive(),
Commands::Logout => auth::logout(),
```

## Edge cases

| Case | Behavior |
|---|---|
| Already logged in | `nerd login` shows "Already logged in as user@. Run `nerd logout` first." |
| Already logged in + `nerd signup` | Allowed — registers a *new* account (email may differ) |
| Not logged in + `nerd logout` | "You are not logged in." |
| Network timeout | Retry prompt or "Could not reach server at <url>. Check your connection." |
| Wrong credentials | "Invalid email or password." with no retry (user reruns command) |
| Password mismatch (signup) | "Passwords do not match." at prompt, no server call |
| Invalid email format | Basic check: must contain `@`. Server validates rest. |
| Password too short | Server returns 422, message forwarded to user |
| Backend URL not configured | Defaults to `http://localhost:3000/api` (existing behavior). User can set via `nerd login --url` or edit config. |

## Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `rpassword` | 7 | Read password without terminal echo |

```toml
# nerd/Cargo.toml
rpassword = "7"
```

## Implementation order

| Step | Files | Time |
|---|---|---|
| `auth.rs` — login(), signup(), logout() API calls | `nerd/src/auth.rs` | 1 hr |
| `config.rs` — add `user_email` field | `nerd/src/config.rs` | 5 min |
| `main.rs` — new subcommands + dispatch | `nerd/src/main.rs` | 30 min |
| Manual testing (register, login, logout, error cases) | — | 30 min |
| **Total** | | **~2 hrs** |
