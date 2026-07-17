# Code Quality Review — nerdtime

Generated from review on 2026-07-16.

---

## Critical (Functional Bugs)

| # | File:Line | Issue |
|---|-----------|-------|
| 1 | `nerd/src/tui/app.rs:246-253` | **Delete confirm is a no-op.** `ConfirmAction::DeleteSession(idx)` and `ConfirmAction::DeleteTask(idx)` call `refresh_all()` but never call any delete function. The `idx` parameter is ignored. Also, no code ever sets these variants — there's no `dd` keybinding to trigger the confirm dialog. |
| 2 | `nerd/src/tui/app.rs:527-530` | **`$EDITOR` passed as literal string** to `sh -c`. The shell variable is never expanded because the string is single-quoted by the shell. Devlog edit is silently broken. |
| 3 | `nerdtime-api/config/development.yaml:107`, `test.yaml:94` | **Hardcoded JWT secrets** committed to repo. Anyone with repo access can forge JWTs for dev/test instances. |

## High (Error Swallowing — Silent Failures)

| # | File:Line | Issue |
|---|-----------|-------|
| 4 | `nerd/src/tui/app.rs` (14x: lines 259, 285, 397, 437, 444, 447, 471, 473, 503, 510, 527, 567, 605, 707) | `let _ =` silently discards `Result` from DB operations (`start_session`, `stop_session`, `add_task`, `complete_task`, `cancel_task`, `insert_devlog_entry`, editor spawn). Users get zero feedback when these operations fail. |
| 5 | `nerdtime-api/src/controllers/billing.rs:157` | `chrono::DateTime::from_timestamp(...).unwrap_or_default()` silently substitutes `1970-01-01T00:00:00Z` if Stripe returns an unrepresentable timestamp. |
| 6 | `nerdtime-api/src/controllers/auth.rs:19` | `Regex::new(...).expect("...")` will panic if the regex fails to compile. While unlikely for this static regex, `expect()` in a request handler is fragile. |
| 7 | `nerdtime-db/src/connection.rs:72-79` | 5 ALTER TABLE migrations use `let _ = conn.execute(...)`. Intentional for idempotency, but a failure for reasons other than "column already exists" (e.g., disk full) would be invisible. |

## Medium (Code Smells / Dead Code)

| # | File:Line | Issue |
|---|-----------|-------|
| 8 | `nerd/src/tui/app.rs:8-9` | 5 unused imports: `Advice`, `ProjectStat`, `Session`, `TaskRow`, `Instant` (all re-exported via `nerdtime_db`) |
| 9 | `nerd/src/tui/widgets.rs:3,6` | 4 unused imports: `Direction`, `List`, `ListItem`, `ListState` |
| 10 | `nerd/src/tui/panels/stats.rs:12`, `advisor.rs:3`, `tasks.rs:3` | `SparklineBar` (stats), `Direction` (stats, advisor, tasks) imported but unused |
| 11 | `nerd/src/tui/app.rs:239` | `self.active_modal.clone()` clones the entire `Modal` enum (can contain large `String` fields like `message` in `Confirm`) on **every keypress** in modal mode. Could use a reference. |
| 12 | `nerd/src/tui/app.rs:552` | `self.insert_buffer.clone()` clones the entire buffer on every `Enter` in insert mode. |
| 13 | `nerd/src/tui/modals.rs` (5x) | `.clone()` calls on committed field values on every frame render in rendering hot paths. |
| 14 | `nerdtime-api/src/workers/downloader.rs:19-21` | `DownloadWorker::perform` is an empty `// TODO` no-op, registered as a background worker. Will silently do nothing if triggered. |
| 15 | `nerd/src/main.rs:745` | `_conn` passed to `resolve_eisenhower()` but never used. Function signature could be simplified. |
| 16 | `nerd/src/github.rs:186` | Typo: `filer` should be `filter` |
| 17 | `nerd/src/devlog.rs:306` | Awkward pluralization: `"Found {} entr(ies):\n"` — should be `"Found {} entr(y/ies):\n"` or simply `"Found {} entries:\n"` |
| 18 | `nerdtime-db/src/tasks.rs:28` | `matches.into_iter().next().unwrap()` — safe because guarded by `matches.len() == 1`, but fragile. Add `.expect("exactly one match")`. |
| 19 | `nerdtime-api/src/models/subscriptions.rs:36,41,46` | Stripe credentials (`secret_key`, `webhook_secret`, `price_id`) silently default to empty string via `unwrap_or_default()`. If `billing.enabled = true` but a key is missing, downstream Stripe calls fail with confusing errors. |
| 20 | `nerdtime-api/src/controllers/sync.rs:18` | `if sub.tier != "free" && sub.is_active()` — `is_active()` returns `true` for `tier == "free"`, making the condition read confusingly. Logic is correct but not obvious. |

## Low (Rustfmt / Style)

| # | File:Line | Issue |
|---|-----------|-------|
| 21 | Workspace root | No `.rustfmt.toml` in `nerd/` or `nerdtime-db/`. Only `nerdtime-api/` has one. Running `cargo fmt` from workspace root uses default settings (120-char limit instead of 100). |
| 22 | All TUI files | 30+ lines exceed 100 characters (see section 3 of the full review). |
| 23 | `nerdtime-api/src/mailers/auth.rs:10-12` | Static dir vars use `snake_case` instead of `SCREAMING_SNAKE_CASE`, suppressed by `#[allow(non_upper_case_globals)]`. |
| 24 | `nerdtime-api/src/controllers/auth.rs:173-181` | Doc comments exceed 100-char limit. |
| 25 | `nerd/src/tui/app.rs:124` | `filter_text_prev` — inconsistent naming vs `filter_text`. Should be `previous_filter_text` or `filter_text_previous`. |

## Project Health Summary

| Metric | Status |
|--------|--------|
| SPDX headers (all 26+ `.rs` files) | ✅ 100% compliant |
| `unsafe` blocks in production code | ✅ 0 occurrences |
| `dbg!` / `println!` in production code | ✅ 0 occurrences |
| SQL injection risk | ✅ 0 (SeaORM query builder throughout API) |
| Build warnings (`nerd` crate) | ⚠️ 51 (all unused imports / dead code) |
| Build warnings (`nerdtime-api` crate) | ✅ 0 (clean) |
| Clippy violations | ❓ Not yet checked — `cargo clippy` not run as part of this review |
| Test coverage | ✅ Unit tests exist for sessions (10), tasks (10), devlog (10), heatmap (7), CLI (10), MCP (11); API integration tests for billing + sessions |
