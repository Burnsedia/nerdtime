# nerdtime Roadmap

Product phases for nerdtime, the terminal-native time tracker for developers.

## Legend

| Icon | Meaning |
|---|---|
| ✅ | Shipped |
| 🛠 | In progress |
| 📝 | Spec complete, not started |
| 💡 | Idea / not spec'd |

## Phase 0: MVP ✅

*The foundational product. Offline CLI, cloud sync, payment gating.*

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

## Phase 1: Launch polish

*Ship-ready quality. Interactive auth, deployment, landing page.*

- [ ] Stripe SDK migration (typed API, drop raw reqwest)
- [ ] `nerd login` interactive prompt (`rpassword`)
- [ ] `nerd signup` / `nerd logout`
- [ ] Landing page at nerdtime.app
- [ ] Install script + GitHub release binaries
- [ ] Production deployment (backend + DB + Redis)
- [ ] Basic error reporting / monitoring

## Phase 2: Power tools

*Features that convert CLI users into paying sync users.*

- [ ] `nerd heatmap` — GitHub-style contribution heatmap
- [ ] `nerd insights` — per-project breakdown by weekday/hour
- [ ] `nerd tasks` — todo tracking alongside sessions
- [ ] `nerd estimate` — estimate vs actual time
- [ ] `nerd summary` — aggregate by label/project
- [ ] Labels (JSON array, cross-cutting)
- [ ] GitHub Issues sync (link, import, close)
- [ ] `nerd sync --auto` — periodic background sync

## Phase 3: Editor ecosystem

*Meet developers where they live.*

- [ ] Neovim plugin — `:NerdtimeStart`, `:NerdtimeStop`, statusline
- [ ] VS Code extension — status bar, start/stop from command palette
- [ ] `nerd tui` — Ratatui terminal UI for browsing sessions

## Phase 4: Mobile + AI

*Extend to mobile devices and AI coding agents.*

- [ ] Tauri mobile app (iOS + Android) — view sessions, start/stop
- [ ] MCP server — expose start/stop/status/sessions as AI tools
- [ ] GitHub OAuth — login via GitHub instead of email/password
- [ ] Team workspaces (maybe — wait for demand)

## Never

- Windows support
- Source-available licensing
- Sponsorship / donation model
