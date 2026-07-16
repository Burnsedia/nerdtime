// SPDX-License-Identifier: AGPL-3.0-only
pub mod advisor;
pub mod connection;
pub mod devlog;
pub mod git;
pub mod sessions;
pub mod stats;
pub mod tasks;
pub mod util;

pub use connection::{get_connection, init_schema};
pub use git::{git_branch, git_commit_hash};
pub use util::{fmt_duration, parse_duration};

// Re-export core types for convenience
pub use rusqlite::Connection;

pub use nerdtime_core::{
    Advice, AdvisorInput, CachedCommit, DevlogEntry, HeatmapCell, Insights, ProjectStat, Session,
    SessionEstimate, SessionSummary, SyncPayload, TaskRow,
};

// Re-export important functions for convenient access
pub use advisor::decide;
pub use devlog::{
    cache_commit, get_cached_commit_map, get_cached_commits, get_devlog_entry,
    get_last_devlog_date, get_used_commit_shas, insert_devlog_entry, list_devlog_entries,
    render_devlog_md, search_devlog_entries, update_devlog_entry,
};
pub use sessions::{
    get_unsynced_sessions, list_sessions, mark_synced, show_status, start_session,
    stats_by_project, stop_session,
};
pub use stats::{heatmap_data, insights_data, label_summary};
pub use tasks::{
    add_task, cancel_task, complete_task, edit_task, find_task_by_github_issue,
    get_task_github_info, get_task_labels, list_tasks, resolve_task_id, task_estimate,
    unsynced_active_tasks,
};
