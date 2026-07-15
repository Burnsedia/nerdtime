# Tauri Mobile App — Implementation Plan

## Architecture

```
┌──────────────────────────────────────────────────┐
│                  nerdtime-tauri                   │
│  ┌────────────────┐      ┌────────────────────┐  │
│  │  Vue 3 + Vite   │invoke│   Tauri Rust Backend │  │
│  │  + Tailwind     │◄────►│   (commands.rs)      │  │
│  │  (src/)         │      │         │            │  │
│  └────────────────┘      └─────────│────────────┘  │
└─────────────────────────────────────│───────────────┘
                                      │
            ┌─────────────────────────│───────────────┐
            │              ┌──────────▼──────────┐    │
            │              │     nerdtime-db      │    │
            │              │  (shared SQLite ops)  │    │
            │              │  start / stop / list  │    │
            │              │  status / mark_synced │    │
            │              └──────────┬──────────┘    │
            │              ┌──────────▼──────────┐    │
            │              │    nerdtime-core     │    │
            │              │  Session / SyncPayload│   │
            │              └─────────────────────┘    │
            │                                         │
            │         Also consumed by:               │
            │    ┌────────────────────┐               │
            │    │  nerd CLI (nerd/)  │               │
            │    │  (blocking sync)    │               │
            │    └────────────────────┘               │
            └─────────────────────────────────────────┘
```

## New workspace members

| Crate | Path | Purpose |
|-------|------|---------|
| `nerdtime-db` | `nerdtime-db/` | Shared SQLite operations (extracted from CLI's `db.rs`) |
| `nerdtime-tauri` | `nerdtime-tauri/` | Tauri v2 app (Vue + Rust backend) |

## Phase 1: Extract `nerdtime-db` shared crate

### Files to create (3)

| File | Purpose |
|------|---------|
| `nerdtime-db/Cargo.toml` | Deps: `rusqlite` (0.32 bundled), `nerdtime-core`, `chrono`, `uuid`, `dirs`, `anyhow` |
| `nerdtime-db/src/lib.rs` | Re-export all public API |
| `nerdtime-db/src/sessions.rs` | `get_connection`, `ensure_schema`, `start_session`, `stop_session`, `show_status`, `list_sessions`, `get_unsynced_sessions`, `mark_synced`, `stats_by_project` |

### Files to modify (2)

| File | Change |
|------|--------|
| `nerd/src/Cargo.toml` | Replace local `db.rs` code with `nerdtime-db` dependency; remove `rusqlite`, `dirs`, `anyhow` (inherited from `nerdtime-db`) |
| `nerd/src/main.rs` | Update imports to use `nerdtime_db::*` instead of `db::*` |

### DB API surface

```rust
// All functions take &rusqlite::Connection (sync, no async)
pub fn get_connection() -> Result<(Connection, PathBuf), anyhow::Error>;
pub fn start_session(conn: &Connection, project: &str, desc: Option<&str>) -> Result<()>;
pub fn stop_session(conn: &Connection) -> Result<Option<Duration>>;
pub fn show_status(conn: &Connection) -> Result<Option<Session>>;
pub fn list_sessions(conn: &Connection, project: Option<&str>, limit: usize) -> Result<Vec<Session>>;
pub fn get_unsynced_sessions(conn: &Connection) -> Result<Vec<Session>>;
pub fn mark_synced(conn: &Connection, ids: &[Uuid]) -> Result<()>;
pub fn stats_by_project(conn: &Connection) -> Result<Vec<ProjectStat>>;
```

### What about sync?

The CLI's sync logic (`sync_sessions` in `db.rs`) sends HTTP requests. That's app-layer, not DB-layer. Keep it in the CLI's `main.rs` / move to a `nerdtime-db::sync` module behind a `blocking` feature flag.

**Decision**: Keep sync logic in the CLI binary. The Tauri app will have its own async sync logic. `nerdtime-db` is pure DB operations only.

## Phase 2: Create `nerdtime-tauri` app

### Frontend (Vue 3 + Vite + Tailwind)

#### Files to create (many)

| File | Purpose |
|------|---------|
| `nerdtime-tauri/index.html` | Vite entry HTML |
| `nerdtime-tauri/package.json` | Deps: `vue`, `vue-router`, `@tauri-apps/api`, `tailwindcss`, `vite`, `@vitejs/plugin-vue`, `postcss`, `autoprefixer` |
| `nerdtime-tauri/vite.config.js` | Vite config with Tauri dev/prod adjustments |
| `nerdtime-tauri/tailwind.config.js` | Tailwind content paths |
| `nerdtime-tauri/postcss.config.js` | PostCSS with Tailwind plugin |
| `nerdtime-tauri/src/main.js` | Vue app bootstrap + router |
| `nerdtime-tauri/src/App.vue` | Root layout with nav shell |
| `nerdtime-tauri/src/router.js` | Vue Router config |
| `nerdtime-tauri/src/assets/main.css` | `@tailwind` directives |
| `nerdtime-tauri/src/views/Dashboard.vue` | Active timer + quick controls |
| `nerdtime-tauri/src/views/Sessions.vue` | Session list with filters |
| `nerdtime-tauri/src/views/Stats.vue` | Time per project |
| `nerdtime-tauri/src/views/Settings.vue` | API URL, token, billing info |
| `nerdtime-tauri/src/components/TimerCard.vue` | Live elapsed timer + start/stop |
| `nerdtime-tauri/src/components/SessionTable.vue` | Sortable/filterable session rows |
| `nerdtime-tauri/src/components/SyncStatus.vue` | Last sync indicator + sync button |
| `nerdtime-tauri/src/lib/tauri.js` | Thin wrappers around `invoke()` calls |

#### Pages

| Route | View | Purpose |
|-------|------|---------|
| `/` | Dashboard | Active timer, today's total, quick start/stop |
| `/sessions` | Sessions | Filter by project, date; inline edit description |
| `/stats` | Stats | Bar chart of time per project (this week/month/all) |
| `/settings` | Settings | API URL, token (saved to keyring), billing info, about |

#### Vue → Rust communication

Every page calls Tauri `invoke()` commands. No direct HTTP to the backend — the Rust side handles all DB + API logic.

```js
// src/lib/tauri.js
import { invoke } from '@tauri-apps/api/core';

export const startSession = (project, desc) => invoke('start_session', { project, desc });
export const stopSession = () => invoke('stop_session');
export const getStatus = () => invoke('get_status');
export const listSessions = (project, limit) => invoke('list_sessions', { project, limit });
export const getStats = () => invoke('get_stats');
export const syncSessions = () => invoke('sync_sessions');
export const getConfig = () => invoke('get_config');
export const setConfig = (url, token) => invoke('set_config', { url, token });
```

### Rust Backend (Tauri commands)

#### Files to create (5)

| File | Purpose |
|------|---------|
| `nerdtime-tauri/src-tauri/Cargo.toml` | Deps: `tauri` (v2), `tauri-build`, `nerdtime-db`, `nerdtime-core`, `reqwest` (non-blocking), `serde`, `serde_json`, `tokio` |
| `nerdtime-tauri/src-tauri/src/main.rs` | `fn main()` → `tauri::Builder` |
| `nerdtime-tauri/src-tauri/src/lib.rs` | Tauri setup, command registration, app data dir initialization |
| `nerdtime-tauri/src-tauri/src/commands.rs` | All `#[tauri::command]` handlers |
| `nerdtime-tauri/src-tauri/src/sync.rs` | Async API sync logic (reqwest to backend) |

#### Tauri commands

```rust
#[tauri::command]
fn start_session(project: String, desc: Option<String>) -> Result<(), String>;

#[tauri::command]
fn stop_session() -> Result<Option<DurationResponse>, String>;

#[tauri::command]
fn get_status(state: State<AppState>) -> Result<Option<SessionResponse>, String>;

#[tauri::command]
fn list_sessions(project: Option<String>, limit: Option<usize>) -> Result<Vec<SessionResponse>, String>;

#[tauri::command]
fn get_stats(state: State<AppState>) -> Result<Vec<ProjectStatResponse>, String>;

#[tauri::command]
async fn sync_sessions(state: State<'_, AppState>) -> Result<SyncResult, String>;

#[tauri::command]
fn get_config(state: State<AppState>) -> Result<ConfigResponse, String>;

#[tauri::command]
fn set_config(state: State<AppState>, url: String, token: Option<String>) -> Result<(), String>;
```

#### State management

```rust
struct AppState {
    db: Mutex<Connection>,  // rusqlite connection (sync, wrapped in Mutex for Tauri)
    config: Mutex<Config>,  // api_url + token
}
```

DB calls in commands block briefly — acceptable for SQLite (sub-ms). For sync, the command is `async` and uses async reqwest.

#### Config storage

Use Tauri's built-in app data dir (cross-platform) instead of `~/.config/nerdtime/config.toml`:
- Windows: `%APPDATA%/nerdtime/config.json`
- macOS: `~/Library/Application Support/nerdtime/config.json`
- Linux: `~/.local/share/nerdtime/config.json`
- iOS: App sandbox
- Android: App internal storage

This avoids file conflicts with the CLI; the Tauri app has its own config.

#### Database

Shared with the CLI on **desktop only** (same path `~/.config/nerdtime/data.db`). On mobile, stored in the app sandbox.

### Tauri config (`tauri.conf.json`)

```
appName: nerdtime
identifier: dev.nerdtime.app
build:
  frontendDist: ../dist     # Vite build output
  devUrl: http://localhost:1420
  beforeDevCommand: npm run dev
  beforeBuildCommand: npm run build
app:
  windows:
    - title: nerdtime
      width: 400
      height: 700           # Mobile-form-factor window on desktop
security:
  csp: default-src 'self'; connect-src 'self' https://api.stripe.com
```

### Platform-specific setup

#### macOS + iOS

- Requires macOS development machine with Xcode
- iOS signing needs an Apple Developer account
- `Info.plist` configures camera/microphone usage (not needed for v1)

#### Android

- Requires Android Studio + SDK (API 26+)
- `AndroidManifest.xml`: internet permission (for API sync)
- Hardware keyboard support for quick project entry

#### Linux

- System deps: `webkit2gtk-4.1`, `libappindicator3`, etc.
- Bundled via AppImage or Flatpak

### Reusing DB schema

The `nerdtime-db` crate creates the same `sessions` table as the CLI:
```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    project_name TEXT NOT NULL,
    branch_name TEXT,
    commit_hash TEXT,
    description TEXT,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    is_synced INTEGER NOT NULL DEFAULT 0
);
```

On desktop, the Tauri app and CLI share the same SQLite file at `~/.config/nerdtime/data.db`. On mobile, it's sandboxed.

## Phase 3: Update `nerd` CLI

- Swap `nerd/src/db.rs` for `nerdtime-db` dependency
- Keep sync logic in CLI (blocking reqwest)
- Minimal diff — just change imports and Cargo.toml

## What the app can do at launch (MVP features)

| Feature | Implementation |
|---------|---------------|
| Start/stop timer from mobile | Tauri `invoke` → `nerdtime-db::start_session` |
| Live elapsed timer display | `invoke('stop_session')` → return duration; frontend poll/update |
| Session history with filters | `invoke('list_sessions')` → Vue table with project filter |
| Stats per project | `invoke('get_stats')` → simple bar chart (CSS or canvas) |
| Sync to backend | `invoke('sync_sessions')` → async reqwest POST to API |
| Login/settings | Store API URL + token in Tauri app config |
| Dark mode | Tailwind `dark:` variant, system preference detection |

## Future features (not in v1)

- Push notifications (via Tauri plugins)
- iOS widget / Android home screen widget showing active timer
- Apple Watch companion app
- Background time tracking (keeps running if app is killed)
- Biometric auth (Face ID / fingerprint to lock the app)
- Cloud backup of SQLite (iCloud / Google Drive)

## Project cost estimate

| Phase | Files | Estimated effort |
|-------|-------|------------------|
| 1. Extract `nerdtime-db` | 3 create, 2 modify | 1-2 hours |
| 2a. Scaffold Tauri + Vue project | `cargo tauri init`, `npm create vue` | 30 min |
| 2b. Rust commands + sync | 5 create | 2-3 hours |
| 2c. Vue views + components | ~12 create | 4-6 hours |
| 2d. Tailwind styling + polish | Iterate on all views | 2-4 hours |
| 2e. Platform testing (iOS/Android/Mac/Linux) | Config + build fixes | 2-4 hours |
| 3. Update CLI to use `nerdtime-db` | 2 modify | 30 min |
| **Total** | **~25 files** | **~12-18 hours** |
