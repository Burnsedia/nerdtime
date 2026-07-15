# nerdtime DEVLOG — Implementation Plan

## Overview

A structured development logging system that turns every coding session into a queryable, quantified-self record. Replaces the manual `DEVLOG.md` workflow with a deterministic CLI prompt that captures context, decisions, commits, and author type (human/ai/hybrid).

## Data Model

New table `devlog_entries` in the CLI's SQLite (`data.db`):

```sql
CREATE TABLE devlog_entries (
  id TEXT PRIMARY KEY,
  date TEXT NOT NULL,
  title TEXT NOT NULL,
  role TEXT NOT NULL DEFAULT 'human',
  tags TEXT NOT NULL DEFAULT '[]',
  context TEXT NOT NULL DEFAULT '',
  changes TEXT NOT NULL DEFAULT '',
  decisions TEXT NOT NULL DEFAULT '',
  commits TEXT NOT NULL DEFAULT '[]',
  session_id TEXT,
  created_at TEXT NOT NULL
);
```

## CLI Subcommands

### `nerd devlog new` — interactive session log

```
$ nerd devlog new
? Title: Stripe SDK Migration
? Role: [human/ai/hybrid]
? Tags (comma-separated): billing, migration, deps
? Context: Evaluated switching from raw reqwest to async-stripe SDK
? Changes (one per line, blank to finish):
  Swap deps: -reqwest -hmac -sha2 -hex, +async-stripe
  Rewrite create_checkout with SDK types
  Rewrite webhook handler with Webhook::construct_event()
  Rewrite portal endpoint
  Delete stripe_request() and hmac_sha256()

? Key decisions (one per line, blank to finish):
  Use OnceLock for StripeClient to avoid per-request init
  Pin async-stripe to 0.41.x for stability

[Preview]
────────────────────────────────────────────────────────────────
## 2026-07-15: Stripe SDK Migration

**role:** hybrid
**commits:** `8a4afa5` (+203 / -0 lines, 1 file)
**tags:** `billing`, `migration`, `deps`

### Context

Evaluated switching from raw reqwest to async-stripe SDK...

### Changes

- Swap deps...
────────────────────────────────────────────────────────────────
? Append to DEVLOG.md? [Y/n]
```

Auto-detects unlogged commits from git (commits since last devlog entry date). Pre-fills `commits`, `role`, `date`. User fills the narrative.

### `nerd devlog edit <id>` — edit an entry

Opens `$EDITOR` with a TOML representation of the entry. On save, updates SQLite and regenerates DEVLOG.md.

### `nerd devlog query <search>` — search entries

```
$ nerd devlog query "stripe"
Found 2 entries:

2026-07-15: MVP Launch Planning & Stripe SDK Evaluation (hybrid)
  Stripe SDK migration decision, tradeoffs evaluated

2026-07-14: Open Core + Paid Sync Billing (human)
  Stripe integration approach, webhook security

$ nerd devlog query --tags billing
(returns all entries tagged "billing")
```

Uses SQLite FTS5 for full-text search with date-aware context rendering.

### `nerd devlog list` — list recent entries

```
$ nerd devlog list --limit 10
2026-07-15  Stripe SDK Migration                hybrid  3 commits
2026-07-15  MVP Launch Planning                 hybrid  3 commits
2026-07-14  Open Core + Paid Sync Billing       human   17 commits
2026-07-14  Initial Build Session               human   17 commits
```

### `nerd devlog generate` — regenerate DEVLOG.md

Re-renders `DEVLOG.md` from the SQLite table. Used after editing or importing entries. Deterministic — same data always produces same output.

## Post-Commit Hook (`.githooks/post-commit`)

Captures deterministic data automatically on every `git commit`:

```bash
#!/usr/bin/env bash
nerd devlog cache-commit
```

Cached data includes SHA, files changed (+N / -M), message subject, branch, and timestamp. When user runs `nerd devlog new`, cached commits are offered as a batch.

### Author type detection

The commit hook always marks commits as `human` by default. AI agents use the MCP tool `devlog_log_session(author_type: "ai")` to override. Hybrid is set by the user during `nerd devlog new`.

## DEVLOG.md Output Format

Generated from SQLite. Deterministic.

```markdown
## 2026-07-15: Stripe SDK Migration

**role:** hybrid
**commits:** [`8a4afa5`](https://github.com/Burnsedia/nerdtime/commit/8a4afa5) (+203 lines, 1 file)
**tags:** `billing`, `migration`, `deps`

### Context

Evaluated switching from raw reqwest to async-stripe SDK...

### Changes

- Swapped deps: removed reqwest/hmac/sha2/hex, added async-stripe
- Rewrote create_checkout, webhook, portal with typed SDK calls
- Deleted stripe_request() and hmac_sha256() helper functions

### Decisions

- **OnceLock for StripeClient** — avoid per-request initialization cost
- **Pin async-stripe 0.41.x** — breaking changes are infrequent but painful
```

## MCP Server Tools

Exposed via the planned MCP server:

| Tool | Purpose | Params |
|---|---|---|
| `devlog_log_session` | Log a session entry | `title`, `role`, `tags`, `context`, `changes[]`, `decisions[]`, `commits[]` |
| `devlog_query` | Search entries | `query`, `tags`, `limit` |
| `devlog_get_decisions` | Return all decisions | `tag` (optional) |
| `devlog_get_timeline` | Structured timeline | `start_date`, `end_date`, `tags` |

AI agents call `devlog_log_session` after completing a task batch, setting `role: "ai"`.

## Insights Integration

`nerd insights` gains a `--devlog` flag:

```
$ nerd insights --devlog --week
Week of Jul 14:
  Sessions logged: 3
  Decisions made: 12
  AI-only commits: 8 (450 lines)
  Human-only commits: 22 (890 lines)
  Hybrid commits: 5 (340 lines)
  Lines per commit (AI): 56 avg
  Lines per commit (human): 40 avg
  Lines per commit (hybrid): 68 avg
```

## Files Changed

| File | Change |
|---|---|
| `nerd/Cargo.toml` | No new deps (rusqlite already present) |
| `nerd/src/devlog.rs` | New: devlog subcommand module |
| `nerd/src/main.rs` | Register `devlog` subcommand |
| `nerd/src/db.rs` | Add `devlog_entries` table creation + CRUD + FTS5 |
| `.githooks/post-commit` | New: auto-cache commit data |
| `DEVLOG.md` | Generated from SQLite (no longer manually edited) |

## Implementation Order

1. `db.rs` — add `devlog_entries` table + CRUD + FTS5 index
2. `devlog.rs` — `cache-commit` subcommand
3. `devlog.rs` — `new` subcommand with interactive prompt
4. `devlog.rs` — `list` subcommand
5. `devlog.rs` — `query` subcommand
6. `devlog.rs` — `generate` subcommand (render DEVLOG.md)
7. `devlog.rs` — `edit` subcommand
8. `.githooks/post-commit` — wire up cache-commit
9. `main.rs` — register devlog routes
10. Update `AGENTS.md` — add devlog workflow + post-commit setup

## Verification

- `cargo build -p nerd` — compiles
- `nerd devlog new` — interactive prompt works
- `git commit --allow-empty -m "test"` → cache populated
- `nerd devlog query "test"` — finds the entry
- `nerd devlog generate` — DEVLOG.md regenerated identically
- `nerd devlog list` — shows entries

## Non-goals

- Cloud sync of devlog entries (stays local; synced alongside sessions via `nerd sync`)
- AI-generated summaries (the AI writes the log via the MCP tool, but the CLI doesn't call an LLM)
- Migration of existing DEVLOG.md entries (manual `nerd devlog new` for past sessions if desired)
