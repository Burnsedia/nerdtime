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
