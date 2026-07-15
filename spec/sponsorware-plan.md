# [ARCHIVED] nerdtime Sponsorware — Implementation Plan

> **Deprecated in favor of Open Core + SaaS model.** CLI is free (AGPL). Cloud sync + mobile app is $10/mo via Stripe billing. No license keys, no sponsor gating in the CLI. This plan is kept for reference only.

## Overview

A sponsorware model where the core CLI is free (AGPL) and advanced features require a GitHub Sponsorship license key. A separate hosted SaaS tier (Stripe billing) is available for users who want a managed backend without self-hosting.

## Two-phase rollout

### Phase 1: Private repo (now)
- Repo stays private
- No license gating — team/collaborators have repo access
- Build all planned features (TUI, MCP server, Tauri app)
- License key system is implemented but not enforced yet
- Stripe SaaS code is kept but dormant (no keys configured)

### Phase 2: Public sponsorware (future)
- Core CLI + TUI goes public under AGPL
- Sponsor features require a license key (sync, MCP server, Tauri app)
- Stripe billing activated for hosted SaaS (nerdtime-api deployed on a server)
- License key verification enforced in CLI + MCP server

## Feature tiers

| Tier | Features | License | Price |
|------|----------|---------|-------|
| **Free** | CLI (start/stop/status/log), TUI, local SQLite | AGPL | Free |
| **Sponsor** | Cloud sync, MCP server, Tauri desktop app | AGPL + sponsor key | GitHub Sponsorship |
| **SaaS** | Hosted backend, team features, no self-host | Proprietary (Stripe) | $4/mo (existing billing) |

## License key system

### Architecture

```
┌─────────────────────────────────────────────────────┐
│                    Keygen / Server                    │
│  Generates HMAC-SHA256 keys signed with a secret      │
│  Stores: key_hash, tier, github_user, created_at      │
└──────────────┬──────────────────────────────────────┘
               │
               │ key distributed via GitHub Sponsors
               │ (email, webhook, or manual)
               ▼
┌─────────────────────────────────────────────────────┐
│              nerd CLI / MCP Server                    │
│  Verifies key offline using embedded public key       │
│  Key stored in ~/.config/nerdtime/config.toml         │
│  `nerd login --key <key>` to activate                 │
└─────────────────────────────────────────────────────┘
```

### How keys work

1. **Key generation**: `HMAC-SHA256(sponsor_secret, "sponsor:" + github_user + ":" + tier + ":" + expires)`
2. **Key format**: `nt_sponsor_<base64-encoded-hmac-and-metadata>` — a single string
3. **Verification**: CLI decodes the key, recomputes HMAC with embedded public key, checks expiry
4. **Storage**: Key saved to `~/.config/nerdtime/config.toml` as `sponsor_key`
5. **Offline**: Verification is purely local — no phone-home required

### Files to create

| File | Purpose |
|------|---------|
| `nerd/src/sponsor.rs` | Key verification logic (`is_sponsor()`, `verify_key()`, `get_tier()`) |
| `tools/nt-keygen/src/main.rs` | Standalone key generator binary (run by you to issue keys) |
| `tools/nt-keygen/Cargo.toml` | Deps: `hmac`, `sha2`, `hex`, `clap`, `serde_json`, `base64` |
| `nerdtime-api/src/controllers/sponsor.rs` | Webhook endpoint for GitHub Sponsors events |

### Key verification (`sponsor.rs`)

```rust
pub struct SponsorKey {
    pub github_user: String,
    pub tier: SponsorTier,  // Sponsor | Lifetime
    pub expires_at: Option<chrono::DateTime<Utc>>,
    pub signature: Vec<u8>,
}

pub enum SponsorTier {
    Sponsor,
    Lifetime,
}

/// Embedded public key (actually a shared HMAC secret, compiled into the binary)
/// This lets us verify offline. The secret is the same one used to generate keys.
/// In a public repo, this would be replaced with an asymmetric key (Ed25519).
const SPONSOR_SECRET: &[u8] = include_bytes!("../sponsor_secret.bin");

pub fn verify_key(key_str: &str) -> Result<SponsorKey, SponsorError> {
    // Decode base64 key
    // Extract HMAC + metadata
    // Verify HMAC against SPONSOR_SECRET
    // Check expiry
    // Return SponsorKey
}

pub fn is_sponsor(config: &Config) -> bool {
    match &config.sponsor_key {
        Some(key) => verify_key(key).is_ok(),
        None => false,
    }
}
```

### What's gated

In the CLI and MCP server, add a check before sponsor features:

```rust
// In sync.rs (CLI)
pub fn sync_sessions(conn: &Connection) -> Result<()> {
    let cfg = config::load()?;
    if !sponsor::is_sponsor(&cfg) {
        anyhow::bail!("Cloud sync requires a sponsor key.\nGet one at https://github.com/sponsors/nerdtime");
    }
    // ... existing sync logic ...
}
```

| Feature | Gate | Location |
|---------|------|----------|
| Cloud sync | `is_sponsor()` check | `nerd/src/db.rs:sync_sessions` |
| MCP server | `is_sponsor()` check | `nerdtime-mcp/src/main.rs` on startup |
| Tauri desktop app | License key prompt on first launch | `nerdtime-tauri/src-tauri/` |
| Backend API sync/sessions/stats | No change — existing Stripe gating | `nerdtime-api/src/controllers/sync.rs` |

### `nerd login --key` flow

```bash
$ nerd login --key nt_sponsor_abc123def456
✓ Sponsor key activated — tier: Sponsor
```

## GitHub Sponsors integration

### Manual (MVP)

1. Sponsor signs up via GitHub Sponsors
2. GitHub notifies you (email or dashboard)
3. You run `nt-keygen --user <github_user> --tier sponsor` and send them the key
4. User runs `nerd login --key <key>`

### Automated (later)

1. GitHub Sponsors webhook → `POST /api/sponsor/webhook` on nerdtime-api
2. Backend generates key, stores in DB, returns in webhook response
3. GitHub delivers the key to the sponsor via the "thank you" message

Webhook handler (`nerdtime-api/src/controllers/sponsor.rs`):

```rust
pub async fn sponsor_webhook(
    State(ctx): State<AppContext>,
    body: String,
) -> Result<Response> {
    // Verify GitHub webhook signature HMAC
    // Parse event type (sponsorship.created, sponsorship.cancelled, etc.)
    // Generate license key
    // Store in sponsors table
    // Return key in response
}
```

## Key generator tool

A small CLI tool in `tools/nt-keygen/` for generating keys:

```bash
$ nt-keygen --user cypher --tier sponsor --expires 2027-07-14
nt_sponsor_abc123def456...

$ nt-keygen --user cypher --tier lifetime
nt_sponsor_def789ghi012...
```

## What changes vs stays

### Stays (no changes needed)
- **AGPL headers** on all files — core is still open source
- **`nerdtime-api` billing code** — Stripe SaaS path remains dormant
- **Existing subscription gating** — still works for SaaS deployment

### Changes needed

| File | Change |
|------|--------|
| `nerd/Cargo.toml` | Add `hmac`, `sha2`, `hex`, `base64` deps (for key verification) |
| `nerd/src/sponsor.rs` | New: key verification logic |
| `nerd/src/main.rs` | Add `--key` flag to `Login` command |
| `nerd/src/config.rs` | Add `sponsor_key: Option<String>` field |
| `nerd/src/db.rs` | Add `is_sponsor()` check before sync |
| `nerdtime-mcp/Cargo.toml` | Add `hmac`, `sha2`, `hex`, `base64` deps |
| `nerdtime-mcp/src/main.rs` | Add key check on startup |
| `tools/nt-keygen/Cargo.toml` | New: keygen tool |
| `tools/nt-keygen/src/main.rs` | New: keygen tool |
| `Cargo.toml` (workspace) | Add `tools/nt-keygen` member |
| `.gitignore` | Add `sponsor_secret.bin` |

### Not yet needed (Phase 2)
- GitHub Sponsors webhook on backend
- Automated key delivery
- Public repo split (free vs sponsor features in separate directories or crates)
- Asymmetric Ed25519 keys (for public repo; HMAC secret-in-binary is fine for private)

## Sponsor secret management

- Single shared HMAC secret: `sponsor_secret.bin` (32 bytes, generated once)
- Same secret used by `nt-keygen` (to sign) and the CLI/MCP server (to verify)
- **Not committed to git** — added to `.gitignore`
- In Phase 2 (public repo): replace with Ed25519 keypair (public key in repo, private key held by you)

```bash
# Generate the secret
openssl rand -out nerd/src/sponsor_secret.bin 32
cp nerd/src/sponsor_secret.bin nerdtime-mcp/src/sponsor_secret.bin
```

## Estimate

| Step | Time |
|------|------|
| `sponsor.rs` key verification module | 1 hr |
| `nt-keygen` tool | 1 hr |
| `nerd login --key` integration | 30 min |
| Gating sync behind sponsor check | 30 min |
| Sponsor secret generation + CI/Script | 30 min |
| **Total** | **~3.5 hrs** |
