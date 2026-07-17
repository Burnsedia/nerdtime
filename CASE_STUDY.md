# Building nerdtime: A Case Study in Vibe Coding / Spec Driven Development

## Origin

nerdtime started as a PDF specification describing a full-stack Rust time tracker. The spec was extracted via `pdftotext` and fed to an AI coding agent (OpenCode). In ~2 hours of iterative sessions, the core MVP was built:

- **CLI** (`nerd start/stop/status/log/sync/login`) — clap + rusqlite, offline-first
- **Backend API** (auth, sync, sessions, stats) — Loco / Axum + SeaORM + PostgreSQL
- **Shared types** (`nerdtime-core`) — serde + chrono + uuid
- **Billing** — Stripe Checkout, webhook, portal, subscription gating
- **Self-host deployment** — docker-compose with Traefik + PostgreSQL

Total: ~3,500 lines of Rust + TOML, compiled and running in a single evening.

## Process: Spec Driven Development

Every coding session follows the same loop:

1. **Session begins** — AI reads AGENTS.md (conventions, quirks, commands) and DEVLOG.md (context from past sessions)
2. **Conversation** — User describes what to build; AI proposes a plan
3. **Implementation** — AI writes code, user reviews, back-and-forth refinement
4. **Session ends** — Changes are committed, DEVLOG.md is updated with what was done and why

The key files that make this work:
- **AGENTS.md** — Compact instruction file answering "What would an agent likely miss?" Commands, architecture quirks, framework gotchas, error handling patterns.
- **DEVLOG.md** — Structured development history. Every entry has context, decisions, tradeoffs, files changed, and commit references. This is the transferable memory between sessions.
- **Spec files** (`spec/`) — Implementation plans for every feature. Written before code, updated as design evolves.

## What Went Right

- **Offline-first architecture** — The CLI works without the backend. Data lives in local SQLite. Cloud sync is optional. This means the product is useful immediately, and the self-host deployment is just another sync target.
- **Shared types crate** — `nerdtime-core` defines `Session` and `SyncPayload` once. CLI and backend both import it. No serialization mismatches.
- **Self-correcting via Q&A** — The user asked "is this ready to ship?" which triggered a security review that found critical issues the AI had missed. The human-in-the-loop caught what the automation didn't.

## Security Review Findings

After the MVP was "feature complete," a security review of every line of code found several issues. This is what happens when AI writes fast code without operational security instincts:

### Critical

**1. Hardcoded JWT secret in `development.yaml`**

```yaml
auth:
  jwt:
    secret: WqOAD0KPFoE8YgKw7Ok1  # committed to git
```

The development config had a literal JWT signing key checked into the repository. Production.yaml correctly used `{{ get_env(name="JWT_SECRET") }}`, but the dev file was a copy-paste from Loco's default template. Anyone who cloned the repo could forge authentication tokens against any deployment using that config.

*Lesson: AI templates from frameworks often include insecure defaults. Always audit generated configs before committing.*

**2. World-readable credential files**

```rust
std::fs::write(&path, content)?;  // default umask = 644
```

The CLI stores the user's JWT token in `~/.config/nerdtime/config.toml` with default file permissions (`rw-r--r--`). Any process on the machine can read it. The same applies to the SQLite database.

*Lesson: AI assumes files it creates are private. On shared machines (CI, servers), they're not. Explicit `chmod 600` is mandatory for credential-bearing files.*

### High

**3. Webhook replay attack**

The Stripe webhook handler parsed the HMAC signature correctly but never checked the timestamp for freshness. Stripe sends a `t=` parameter alongside each signature. The code parsed it but never validated how old it was. An attacker who captures a valid webhook payload can replay it indefinitely — upgrading or downgrading subscriptions on demand.

```rust
// Manual HMAC verification — no freshness check
let expected = hmac_sha256(&secret, &format!("{}.{}", timestamp, body));
if sig != expected { return bad_request("invalid webhook signature"); }
// timestamp was never checked for age
```

The fix was migrating to the `async-stripe` SDK, where `Webhook::construct_event()` enforces a 5-minute tolerance window by default.

*Lesson: Rolling your own crypto/webhook verification is fragile. Use the SDK. The AI chose raw reqwest to "keep dependencies simple" — but simplicity in deps cost security in implementation.*

**4. Silent failure on Stripe errors**

```rust
let url = json.get("url").and_then(|u| u.as_str()).unwrap_or("");
```

If Stripe returned an error response, the client got `{"url": ""}` — no error message, no logging, no indication anything was wrong. The AI used `unwrap_or("")` as a defensive default, but in practice it just hid failures.

*Lesson: `unwrap_or(default)` for display values is fine. For API responses that represent payment flows, every error must be surfaced. Defaults should skew toward breaking loudly, not silently.*

### Medium

**5. No rate limiting on any endpoint** — Auth login and sync endpoints are unthrottled. Infrastructure-level fix, but not built into the application.

**6. Domain coupling in subscription gating** — `is_active()` returned true for free-tier users even when billing was enabled. The gating logic compensated with a secondary `tier != "free"` check, but the design invited misuse.

### Low

**7. New HTTP client per request** — Every Stripe API call created a fresh `reqwest::Client`, wasting connection pools.

**8. No sync payload size limit** — The sync endpoint accepts unbounded arrays of session data.

## Scope Creep in AI-Assisted Development

**Where it started:**

> A CLI time tracker with cloud sync and Stripe billing. Ship it in one session.

**Where it is now (same project, same week):**

- Heatmap + Insights
- DEVLOG (structured logging)
- Tasks + Eisenhower Matrix + Analysis Paralysis Advisor
- MCP Server (12 tools, zero token cost)
- Labels + Summary + Estimates
- GitHub Issues sync
- Desktop app with SVG heatmaps
- Mobile app (Tauri, iOS + Android)
- Team workspaces (maybe)

That's 3x the original MVP scope, and none of it was planned at session zero.

**Why AI accelerates scope creep:**

- Saying "yes" costs nothing — the AI writes the spec in 30 seconds
- Every feature spawns a sub‑feature (tasks → Eisenhower → advisor → MCP tools)
- "While we're here" is the default mode of conversation
- No one has to estimate the work — the AI just says "~2 hours" every time

**The pattern:**

```
User: "Should I add X?"
AI: "Here's why X is essential, here's a spec, ~2 hours."
User: "What about Y?"
AI: "Y builds on X, here's how, ~2 hours."
Result: 3x scope, zero additional timeline.
```

**The fix (not yet implemented):**

- Freeze the spec at session start. No new features until current batch ships.
- Track "spec'd vs built" ratio. If spec files outnumber working features, stop specing.
- Ship something *before* adding the next thing. The CLI works today. Ship it.

## Lessons Learned

### For AI-Assisted Development

1. **AI writes fast code, but not secure code.** The security review found issues the AI never considered: file permissions, replay attacks, hardcoded secrets, silent error swallowing. These are operational security patterns, not language patterns. The AI knows Rust syntax perfectly but doesn't think like an ops engineer.

2. **SDKs over raw HTTP.** The AI's instinct was to minimize deps by using raw `reqwest` for Stripe. That choice directly caused the webhook replay vulnerability (no built-in timestamp check), the silent error swallowing (no typed response parsing), and extra boilerplate (manual JSON construction). The SDK costs build time but pays for itself in correctness.

3. **Configs need the same review as code.** The hardcoded JWT secret was in a `.yaml` file, not a `.rs` file. AI moves fast and templates configs from framework defaults. These need human review just like business logic.

4. **Session logs are the killer feature.** DEVLOG.md lets a new AI session pick up exactly where the last one left off — no context loss, no "what were we doing?" Every decision, every tradeoff, every quirk discovered is captured. This is the single highest-leverage file in the repo.

### For the Product (nerdtime)

5. **Quantified self for developers needs to be boring.** The heatmap, the devlog, the tasks — these are unexciting features that create compounding value over months. The security flaws were exciting (in a bad way). The product succeeds by being trustworthy, not by being clever.

6. **Deterministic beats magical.** The `what-should-i-work-on` advisor is a decision tree, not an LLM call. The heatmap uses `strftime`, not a charting library. The devlog query uses ripgrep, not embeddings. Every feature works offline, costs nothing, and behaves predictably. For a developer tool, predictability is trust.

## Metrics

- **Total code:** ~3,500 lines Rust + TOML
- **Time to MVP:** ~4 hours of AI sessions (2 sessions)
- **Security issues found:** 9 (2 critical, 2 high, 2 medium, 3 low)
- **Dependencies:** 3 workspace members, ~40 crate deps total
- **Files:** ~30 source files across CLI, backend, shared types
- **Price:** CLI free (AGPL), cloud sync $10/mo, self-host free
- **Platforms:** Linux, macOS (CLI); iOS, Android, macOS+Linux (Tauri, planned)
