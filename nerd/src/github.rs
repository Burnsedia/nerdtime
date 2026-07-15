// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::Value;

pub fn detect_repo() -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .context("Failed to run git. Are you in a git repository?")?;
    if !output.status.success() {
        anyhow::bail!("No git remote 'origin' found.");
    }
    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
    parse_github_repo(&url)
}

fn parse_github_repo(url: &str) -> Result<String> {
    // SSH: git@github.com:user/repo.git
    if let Some(rest) = url.strip_prefix("git@github.com:") {
        let repo = rest.strip_suffix(".git").unwrap_or(rest);
        return Ok(repo.to_string());
    }
    // HTTPS: https://github.com/user/repo.git
    if let Some(rest) = url.strip_prefix("https://github.com/") {
        let repo = rest.strip_suffix(".git").unwrap_or(rest);
        return Ok(repo.to_string());
    }
    anyhow::bail!("Could not parse GitHub repo URL: {}", url);
}

pub fn parse_issue_ref(s: &str, detected_repo: Option<&str>) -> Result<(String, i64)> {
    // "user/repo#42"
    if let Some(rest) = s.find('#') {
        let repo_part = &s[..rest];
        let num_part = &s[rest + 1..];
        let number: i64 = num_part.parse().context("Invalid issue number after '#'")?;
        if repo_part.is_empty() {
            let repo = detected_repo.context(
                "No repo detected. Use 'user/repo#N' syntax or set default_github_repo.",
            )?;
            Ok((repo.to_string(), number))
        } else {
            Ok((repo_part.to_string(), number))
        }
    } else {
        // bare number — use detected repo
        let number: i64 = s
            .parse()
            .context("Invalid issue reference. Use 'N' or 'user/repo#N'.")?;
        let repo = detected_repo
            .context("No repo detected. Use 'user/repo#N' syntax or set default_github_repo.")?;
        Ok((repo.to_string(), number))
    }
}

fn gh_api_get(path: &str) -> Result<Vec<u8>> {
    let output = std::process::Command::new("gh")
        .args(["api", path, "--jq", "."])
        .output()
        .context("Failed to run `gh`. Install GitHub CLI or set github_token in config.")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // check if gh is not installed
        if stderr.contains("command not found") || stderr.contains("not found") {
            anyhow::bail!("`gh` CLI not found. Install it or set github_token in config.");
        }
        anyhow::bail!("GitHub API error: {}", stderr.trim());
    }
    Ok(output.stdout)
}

fn token_api_get(path: &str, token: &str) -> Result<Vec<u8>> {
    let url = format!("https://api.github.com{}", path);
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "nerdtime")
        .send()
        .context("Network error contacting GitHub API")?;
    let status = resp.status();
    let body = resp.bytes().context("Failed to read GitHub response")?;
    if status.is_success() {
        Ok(body.to_vec())
    } else if status.as_u16() == 404 {
        anyhow::bail!("GitHub API returned 404 Not Found for: {}", path);
    } else if status.as_u16() == 403 {
        anyhow::bail!("GitHub API rate limited. Wait or authenticate.");
    } else {
        anyhow::bail!(
            "GitHub API error ({}): {}",
            status,
            String::from_utf8_lossy(&body)
        );
    }
}

fn token_api_patch(path: &str, token: &str, body: &Value) -> Result<Vec<u8>> {
    let url = format!("https://api.github.com{}", path);
    let client = reqwest::blocking::Client::new();
    let resp = client
        .patch(&url)
        .json(body)
        .header("Authorization", format!("Bearer {}", token))
        .header("User-Agent", "nerdtime")
        .send()
        .context("Network error contacting GitHub API")?;
    let status = resp.status();
    let body_bytes = resp.bytes().context("Failed to read GitHub response")?;
    if status.is_success() {
        Ok(body_bytes.to_vec())
    } else if status.as_u16() == 404 {
        anyhow::bail!("GitHub API returned 404 Not Found for: {}", path);
    } else if status.as_u16() == 403 {
        anyhow::bail!("GitHub API rate limited or insufficient permissions.");
    } else {
        anyhow::bail!(
            "GitHub API error ({}): {}",
            status,
            String::from_utf8_lossy(&body_bytes)
        );
    }
}

pub fn get_issue(repo: &str, number: i64, token: Option<&str>) -> Result<Value> {
    let response = match token {
        Some(t) => token_api_get(&format!("/repos/{}/issues/{}", repo, number), t),
        None => gh_api_get(&format!("/repos/{}/issues/{}", repo, number)),
    }?;
    serde_json::from_slice(&response).context("Failed to parse GitHub issue response")
}

pub fn close_issue(repo: &str, number: i64) -> Result<()> {
    let cfg = crate::config::load().ok();
    let token = cfg.as_ref().and_then(|c| c.github_token.as_deref());

    match token {
        Some(t) => {
            let body = serde_json::json!({"state": "closed"});
            token_api_patch(&format!("/repos/{}/issues/{}", repo, number), t, &body)?;
        }
        None => {
            let output = std::process::Command::new("gh")
                .args(["issue", "close", &number.to_string(), "--repo", repo])
                .output()
                .context(
                    "Failed to run `gh issue close`. Install GitHub CLI or set github_token.",
                )?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("Failed to close issue: {}", stderr.trim());
            }
        }
    }
    Ok(())
}

pub fn list_issues(
    repo: &str,
    label: Option<&str>,
    milestone: Option<&str>,
    state: Option<&str>,
    token: Option<&str>,
) -> Result<Vec<Value>> {
    let state = state.unwrap_or("open");
    let mut path = format!("/repos/{}/issues?state={}&per_page=100", repo, state);
    if let Some(l) = label {
        path.push_str(&format!("&labels={}", l));
    }
    if let Some(ms) = milestone {
        // resolve milestone name → number
        let num = resolve_milestone(repo, ms, token)?;
        path.push_str(&format!("&milestone={}", num));
    }

    let response = match token {
        Some(t) => token_api_get(&path, t),
        None => gh_api_get(&path),
    }?;
    let issues: Vec<Value> =
        serde_json::from_slice(&response).context("Failed to parse GitHub issues list response")?;
    // filer out PRs (GitHub returns PRs as issues in this endpoint)
    Ok(issues
        .into_iter()
        .filter(|i| i.get("pull_request").is_none())
        .collect())
}

fn resolve_milestone(repo: &str, name: &str, token: Option<&str>) -> Result<i64> {
    let path = format!("/repos/{}/milestones?state=all&per_page=100", repo);
    let response = match token {
        Some(t) => token_api_get(&path, t),
        None => gh_api_get(&path),
    }?;
    let milestones: Vec<Value> =
        serde_json::from_slice(&response).context("Failed to parse milestones response")?;
    for ms in &milestones {
        if ms["title"].as_str() == Some(name) {
            return ms["number"].as_i64().context("Milestone has no number");
        }
    }
    anyhow::bail!("Milestone '{}' not found in {}.", name, repo);
}

pub fn import_issue_as_task(
    conn: &rusqlite::Connection,
    repo: &str,
    issue: &Value,
    _token: Option<&str>,
) -> Result<Option<String>> {
    let number = issue["number"].as_i64().context("Issue missing number")?;
    // dedup
    if let Some(existing) = crate::db::find_task_by_github_issue(conn, repo, number)? {
        return Ok(Some(existing));
    }
    let title = issue["title"]
        .as_str()
        .context("Issue missing title")?
        .to_string();
    let body = issue["body"].as_str().unwrap_or("");
    let repo_full = repo.to_string();

    // extract GitHub labels as comma-separated
    let gh_labels: Vec<String> = issue["labels"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|l| l["name"].as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let labels_str = if gh_labels.is_empty() {
        None
    } else {
        Some(gh_labels.join(","))
    };

    let est = issue["body"].as_str().and_then(|b| {
        // try to parse "[estimate: 2h]" from body
        let lower = b.to_lowercase();
        let prefix = "[estimate: ";
        if let Some(start) = lower.find(prefix) {
            let rest = &lower[start + prefix.len()..];
            if let Some(end) = rest.find(']') {
                let val = rest[..end].trim();
                return crate::db::parse_duration(val).ok().flatten();
            }
        }
        None
    });

    let tid = crate::db::add_task(
        conn,
        &repo_full,
        &title,
        Some(body),
        est,
        3,
        3,
        labels_str.as_deref(),
        Some(&repo_full),
        Some(number),
    )?;
    println!(
        "  {} Imported #{} {} {}",
        "+".green(),
        number,
        title,
        if let Some(e) = est {
            format!("({})", crate::insights::fmt_duration(e))
        } else {
            String::new()
        }
    );
    Ok(Some(tid))
}
