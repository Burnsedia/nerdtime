# nerdtime Roadmap

Product phases for nerdtime, the quantified-self system for developers.

## Legend

| Icon | Meaning |
|---|---|
| ✅ | Shipped |
| 🛠 | In progress |
| 📝 | Spec complete, not started |
| 💡 | Idea / not spec'd |

## Phase 0: Foundation ✅

*The offline-first CLI with cloud sync.*

- [x] `nerd start/stop/status/log` — core tracking loop
- [x] `nerd sync` — offline queue → cloud
- [x] `nerd login/config` — headless auth
- [x] Git auto-detection (branch + commit)
- [x] Backend auth (register/login/JWT)
- [x] Cloud sync API + session listing
- [x] Stats aggregation per project
- [x] Stripe billing (checkout, portal, webhook, info)
- [x] Subscription gating (free tier vs pro)
- [x] Self-host deployment (docker-compose)

## Phase 1: Quantified Self MVP

*The full dev intelligence loop: track → log → prioritize → visualize.*

### Time & heatmap

- [ ] `nerd heatmap` — GitHub-style terminal contribution grid (weekday × hour)
- [ ] `nerd insights` — per-project breakdown by weekday/hour, trend analysis

### Tasks & Eisenhower Matrix

- [ ] `nerd task add/list/edit/complete/cancel` — task CRUD
- [ ] Eisenhower Matrix (urgency 1-5, importance 1-5, 4 quadrants)
- [ ] `nerd task matrix` — quadrant view
- [ ] `nerd what-should-i-work-on` — deterministic analysis paralysis helper

### DEVLOG

- [ ] `nerd devlog new` — interactive session logging with commit auto-capture
- [ ] `nerd devlog query` — full-text search via SQLite FTS5
- [ ] `nerd devlog list` — recent entries
- [ ] `nerd devlog generate` — render DEVLOG.md from SQLite
- [ ] `.githooks/post-commit` — auto-cache commit data

### MCP server

- [ ] `nerdtime-mcp` binary — stdio MCP server for AI coding agents
- [ ] Session tools (start, stop, status, list, stats, sync)
- [ ] Task tools (create, list, matrix, complete, edit)
- [ ] Devlog tools (log_session, query, get_decisions)
- [ ] `what_should_i_work_on` tool

### Ship

- [ ] Stripe SDK migration (typed API, drop raw reqwest)
- [ ] Landing page at nerdtime.app with heatmap hero image
- [ ] Interactive CLI auth (`nerd login` prompt, `nerd signup`, `nerd logout`)
- [ ] Install script + GitHub release binaries
- [ ] Production deployment (backend + DB + Redis)
- [ ] Upgrade messaging: $10/mo for cloud sync

## Phase 2: Editor ecosystem

*Meet developers in their editor.*

- [ ] Neovim plugin — `:NerdtimeStart`, `:NerdtimeStop`, statusline
- [ ] VS Code extension — status bar, start/stop from command palette
- [ ] `nerd tui` — Ratatui terminal UI for browsing sessions
- [ ] `nerd sync --auto` — periodic background sync

## Phase 3: Mobile + GitHub

*Extend to mobile devices and GitHub integration.*

- [ ] Tauri mobile app (iOS + Android) — view sessions, start/stop
- [ ] GitHub Issues sync (link, import, close tasks from issues)
- [ ] GitHub OAuth — login via GitHub instead of email/password
- [ ] SVG heatmap export (for desktop/mobile app)

## Phase 4: Scale

*Team features and deeper intelligence.*

- [ ] Team workspaces (maybe — wait for demand)

## Never

- Windows support
- Source-available licensing
- Sponsorship / donation model
