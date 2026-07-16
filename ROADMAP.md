# nerdtime Roadmap

Product phases for nerdtime, the terminal-native quantified-self system for developers.

## Legend

| Icon | Meaning |
|---|---|
| ✅ | Shipped |
| 🛠 | In progress |
| 📝 | Spec complete, not started |
| 💡 | Idea / not spec'd |

## Phase 0: Core MVP ✅

*Time tracking foundation. Offline CLI, cloud sync, payment gating.*

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

## Phase 1: Quantified Self

*The features that make nerdtime a personal analytics system, not just a timer.*

- [x] `nerd heatmap` — terminal heatmap (GitHub-style contribution grid)
- [x] `nerd insights` — per-project breakdown by weekday/hour
- [x] `nerd devlog new` — structured session logging CLI
- [x] `nerd devlog query` — search past context/decisions
- [x] `nerd devlog generate` — auto-render DEVLOG.md from SQLite
- [x] Post-commit hook — auto-cache commit data for devlog
- [x] `nerd task add/list/edit/complete/cancel` — task CRUD
- [x] `nerd task matrix` — Eisenhower Matrix view (Q1-Q4)
- [x] `nerd what-should-i-work-on` — deterministic decision tree advisor
- [x] `nerd summary` — aggregate by label/project
- [x] `nerd estimate` — estimate vs actual time
- [x] Labels (JSON array, cross-cutting projects and tasks)
- [x] GitHub Issues sync (link, import, close)

## Phase 2: Launch polish

*Ship-ready quality. Production deployment and onboarding.*

- [x] Stripe SDK migration (typed API, drop raw reqwest)
- [x] `nerd login` interactive prompt (`rpassword`)
- [x] `nerd signup` / `nerd logout`
- [ ] Landing page at nerdtime.app
- [ ] Install script + GitHub release binaries
- [ ] Production deployment (backend + DB + Redis)
- [ ] Basic error reporting / monitoring
- [ ] `nerd sync --auto` — periodic background sync

## Phase 3: Ecosystem

*Meet developers where they work — in their editor and terminal.*

- [x] MCP server — 16 files, 12 tools for AI agents (tracking, tasks, devlog, advisor)
- [ ] Neovim plugin — `:NerdtimeStart`, `:NerdtimeStop`, statusline
- [ ] VS Code extension — status bar, start/stop from command palette
- [ ] `nerd tui` — Ratatui terminal UI for browsing sessions

## Phase 4: Mobile + AI

*Extend to mobile and deepen AI integration.*

- [ ] Tauri mobile app (iOS + Android) — view sessions, start/stop, SVG heatmaps
- [ ] Desktop app (macOS + Linux) — SVG heatmaps, full TUI (covered by Tauri)
- [ ] GitHub OAuth — login via GitHub instead of email/password
- [ ] Team workspaces (maybe — wait for demand)

## Never

- Windows support
- Source-available licensing
- Sponsorship / donation model
