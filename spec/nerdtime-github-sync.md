# nerdtime GitHub Sync — Implementation Plan

> **Status**: ✅ Implemented (import, link, close via `nerd/src/github.rs`)
> 
## Overview

Sync local tasks with GitHub Issues. Pull issue metadata (title, labels, status) when creating local tasks. Optionally close issues when completing local tasks. Auto-detect repository from git remote or accept explicit `user/repo#42` syntax.

## Authentication

Two methods, tried in order:

1. **GitHub CLI (`gh`)** — shell out to `gh api` subprocess. Preferred when available. Handles auth, OAuth, and token management for the user.
2. **Personal Access Token** — store `github_token = "ghp_..."` in `config.toml`. Fallback when `gh` is not installed.

```rust
fn github_api(path: &str) -> Result<reqwest::blocking::Response> {
    // Try gh first
    if which_gh() {
        let output = std::process::Command::new("gh")
            .args(["api", path])
            .output()?;
        // parse stdout as response...
    }
    // Fall back to PAT
    let cfg = config::load()?;
    let token = cfg.github_token.as_ref()
        .context("No GitHub token configured. Set it with `nerd config --github-token` or install `gh`.")?;
    let client = reqwest::blocking::Client::new();
    Ok(client
        .get(&format!("https://api.github.com/{}", path))
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "nerdtime")
        .send()?)
}
```

### Config additions

```toml
# ~/.config/nerdtime/config.toml
api_url = "http://localhost:3000/api"
token = "jwt..."
user_email = "user@example.com"
github_token = "ghp_abc123def456"          # NEW — optional
default_github_repo = "burnsedia/nerdtime" # NEW — optional, for auto-detect fallback
```

```rust
pub struct Config {
    pub api_url: String,
    pub token: Option<String>,
    pub user_email: Option<String>,
    pub github_token: Option<String>,       // NEW
    pub default_github_repo: Option<String>,// NEW
}
```

## Data model

### Tasks table additions

```sql
ALTER TABLE tasks ADD COLUMN github_repo TEXT;
ALTER TABLE tasks ADD COLUMN github_issue_number INTEGER;
```

- `github_repo` — e.g., `"burnsedia/nerdtime"`. NULL = not linked to GitHub.
- `github_issue_number` — e.g., `42`. NULL = not linked.

### Sessions

No changes. Sessions link to tasks via `task_id`. The GitHub association is on the task, not the session.

## CLI commands

### Start tracking linked to an issue

```sh
# Auto-detect repo from git remote
nerd start project --issue 42

# Explicit cross-repo
nerd start project --issue burnsedia/nerdtime#42
```

Flow:
1. Detect repo from `--issue` syntax or git remote
2. Fetch issue title/labels from `GET /repos/{repo}/issues/{number}`
3. Create task (if not exists) with issue title, repo, issue number
4. Start session linked to that task

### Complete task with close-issue

```sh
# Mark complete + prompt to close GitHub issue
nerd task complete <id>

# Mark complete + auto-close GitHub issue
nerd task complete <id> --close-issue
```

When completing a task linked to a GitHub issue, the CLI prompts:
```
✗ Complete task "fix login bug" (nerdtime#42)?
  Close GitHub issue? [Y/n]:
✓ Task completed
```

With `--close-issue`, skips prompt and closes automatically.

### Import issues as tasks

```sh
# Import all open issues from current repo
nerd task import-github

# Import from specific repo + milestone
nerd task import-github --repo burnsedia/nerdtime --milestone "v2.0"

# Import a single issue
nerd task import-github --issue 42

# Import issues with a specific label
nerd task import-github --label bug

# Dry-run (preview without creating)
nerd task import-github --dry-run
```

Output:
```
$ nerd task import-github --repo burnsedia/nerdtime --label bug

Importing issues from burnsedia/nerdtime (label: bug):

  #42  fix login redirect            open         imported
  #57  handle rate limit errors      open         imported
  #63  null pointer in parser        open         skipped (already tracked)

  3 issues | 2 imported, 1 skipped
```

### Listing tasks with GitHub info

```rust
nerd task list
```

Updated output:
```
Status  Title                      Est      Actual    GitHub
●       implement login            4h 00m   2h 15m    #42
○       fix sync bug               —        1h 15m    #57 (closed)
✗       refactor cli args          2h 00m   2h 45m    —
```

## GitHub API calls

| Action | Method | Endpoint | Auth |
|---|---|---|---|
| Get issue | `GET` | `/repos/{repo}/issues/{number}` | Public repos: no auth. Private: token. |
| Close issue | `PATCH` | `/repos/{repo}/issues/{number}` | Token required |
| List issues | `GET` | `/repos/{repo}/issues?state=open&labels={label}&milestone={m}&per_page=100` | Token required |
| Get milestone | `GET` | `/repos/{repo}/milestones` | Token required (to resolve name → number) |

### Issue API response (relevant fields)

```json
{
  "number": 42,
  "title": "fix login redirect",
  "state": "open",
  "labels": [{"name": "bug"}, {"name": "frontend"}],
  "milestone": {"title": "v2.0", "number": 3},
  "html_url": "https://github.com/burnsedia/nerdtime/issues/42",
  "created_at": "2026-01-15T10:00:00Z",
  "updated_at": "2026-01-20T14:30:00Z"
}
```

## Repo auto-detection

Detects the GitHub repo from the current project's git remote:

```rust
fn detect_github_repo() -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output().ok()?;
    let url = String::from_utf8(output.stdout).ok()?;
    parse_github_url(&url)
}

fn parse_github_url(url: &str) -> Option<String> {
    // git@github.com:user/repo.git → user/repo
    // https://github.com/user/repo.git → user/repo
    // https://github.com/user/repo → user/repo
    let url = url.trim();
    let repo = url
        .strip_prefix("git@github.com:")
        .or_else(|| url.strip_prefix("https://github.com/"))
        .or_else(|| url.strip_prefix("ssh://git@github.com/"))?;
    let repo = repo.strip_suffix(".git").unwrap_or(repo);
    let repo = repo.strip_suffix('/').unwrap_or(repo);
    if repo.contains('/') { Some(repo.to_string()) } else { None }
}
```

## Issue → Task mapping

Importing an issue creates a task with:

| Issue field | Task field |
|---|---|
| `title` | `title` |
| `number` | `github_issue_number` |
| `repo` | `github_repo` |
| `state == "open"` | `status = "active"` |
| `state == "closed"` | `status = "completed"` |
| `labels[].name` | `labels` (merged with local labels) |
| `html_url` | `description` (appended or stored) |

### Deduplication

When importing, check if a task already exists with the same `(github_repo, github_issue_number)`. If found, skip or update (if issue state changed).

```sql
SELECT id FROM tasks WHERE github_repo = ?1 AND github_issue_number = ?2
```

## Task complete → Issue close

```rust
pub fn complete_task(conn: &Connection, task_id: &str, close_issue: bool) -> Result<()> {
    let task = get_task(conn, task_id)?;

    // Close GitHub issue if requested
    if close_issue {
        if let (Some(repo), Some(num)) = (&task.github_repo, task.github_issue_number) {
            github_api_patch(&format!("/repos/{}/issues/{}", repo, num), json!({
                "state": "closed"
            }))?;
            println!("✓ Closed GitHub issue {}#{}", repo, num);
        }
    }

    // Update local task
    conn.execute(
        "UPDATE tasks SET status = 'completed', completed_at = ?1 WHERE id = ?2",
        params![Utc::now().to_rfc3339(), task_id],
    )?;
    println!("✓ Task completed");
    Ok(())
}
```

## Error handling

| Situation | Behavior |
|---|---|
| `gh` not installed + no PAT | Error: "Install `gh` or set `github_token` in config" |
| Issue not found (404) | Error: "Issue #42 not found in repo" |
| Private repo + no token | Error: "Authentication required for private repo. Set `github_token`." |
| No git remote | Error: "Could not detect GitHub repo. Use `--issue user/repo#42`." |
| Rate limited | Error: "GitHub API rate limited. Wait or authenticate." |
| No network | Error: "Network error. GitHub sync skipped (local data preserved)." |

## New files and changes

| File | Change |
|---|---|
| `nerd/Cargo.toml` | No new deps (reqwest + serde_json already present) |
| `nerd/src/config.rs` | + `github_token`, `default_github_repo` fields |
| `nerd/src/github.rs` | **NEW** — API calls, repo detection, URL parsing |
| `nerd/src/db.rs` | + `github_repo`, `github_issue_number` columns on tasks, + import/update logic |
| `nerd/src/main.rs` | + `--issue` on `Start`, + `ImportGithub` sub-subcommand, + `--close-issue` on `Complete` |

## Implementation order

| Step | Files | Time |
|---|---|---|
| `github.rs`: repo detection, URL parsing, API helpers | new | 1 hr |
| `github.rs`: get issue, close issue, list issues calls | new | 1 hr |
| `db.rs`: task table migration (github columns) + dedup logic | `db.rs` | 30 min |
| `db.rs`: `import_github_issue()` creates/updates tasks | `db.rs` | 30 min |
| `main.rs`: `--issue` on `Start` command | `main.rs` | 30 min |
| `main.rs`: `ImportGithub` subcommand | `main.rs` | 30 min |
| `main.rs`: `--close-issue` on `Task Complete` | `main.rs` | 15 min |
| `config.rs`: new fields | `config.rs` | 5 min |
| Manual testing | | 1 hr |
| **Total** | | **~5 hrs** |
