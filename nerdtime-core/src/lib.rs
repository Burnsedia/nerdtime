// SPDX-License-Identifier: AGPL-3.0-only
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Uuid,
    pub project_name: String,
    pub branch_name: Option<String>,
    pub commit_hash: Option<String>,
    pub description: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub is_synced: bool,
    pub task_id: Option<String>,
    pub estimated_seconds: Option<i64>,
    pub labels: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPayload {
    pub id: Uuid,
    pub project_name: String,
    pub branch_name: Option<String>,
    pub commit_hash: Option<String>,
    pub description: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub task_id: Option<String>,
    pub estimated_seconds: Option<i64>,
    pub labels: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRow {
    pub id: String,
    pub project_name: String,
    pub title: String,
    pub description: Option<String>,
    pub estimated_seconds: Option<i64>,
    pub urgency: u8,
    pub importance: u8,
    pub quadrant: u8,
    pub status: String,
    pub labels: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
    pub actual_seconds: i64,
    pub github_repo: Option<String>,
    pub github_issue_number: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevlogEntry {
    pub id: String,
    pub date: String,
    pub title: String,
    pub role: String,
    pub tags: Vec<String>,
    pub context: String,
    pub changes: Vec<String>,
    pub decisions: Vec<String>,
    pub commits: Vec<String>,
    pub session_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedCommit {
    pub sha: String,
    pub subject: String,
    pub branch: String,
    pub files_changed: i64,
    pub lines_added: i64,
    pub lines_removed: i64,
    pub committed_at: String,
    pub cached_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeatmapCell {
    pub day: u32,
    pub hour: u32,
    pub total_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insights {
    pub total_seconds: i64,
    pub session_count: i64,
    pub per_block: [i64; 4],
    pub per_day_of_week: [i64; 7],
    pub per_project: Vec<(String, i64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectStat {
    pub project: String,
    pub total_seconds: i64,
    pub session_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvisorInput {
    pub available_seconds: i64,
    pub energy: String,
    pub blocked: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Advice {
    pub task_id: Option<String>,
    pub task_title: String,
    pub project: String,
    pub reason: String,
}

pub type SessionEstimate = (String, String, i64, Option<i64>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: Uuid,
    pub project: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub description: Option<String>,
    pub duration_seconds: Option<i64>,
    pub is_synced: bool,
    pub task_id: Option<String>,
    pub labels: Option<String>,
}
