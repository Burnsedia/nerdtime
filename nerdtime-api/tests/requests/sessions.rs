// SPDX-License-Identifier: AGPL-3.0-only
use insta::assert_debug_snapshot;
use loco_rs::testing::prelude::*;
use nerdtime_api::app::App;
use serial_test::serial;

use super::prepare_data;

#[tokio::test]
#[serial]
async fn test_sync_requires_auth() {
    request::<App, _, _>(|request, _ctx| async move {
        let payload = serde_json::json!([{
            "id": "test-id",
            "project_name": "test",
            "started_at": "2026-07-15T10:00:00Z",
            "ended_at": "2026-07-15T12:00:00Z"
        }]);

        let response = request.post("/api/sync").json(&payload).await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_sync_creates_sessions() {
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;

        let payload = serde_json::json!([{
            "id": "sync-test-1",
            "project_name": "nerdtime",
            "branch_name": null,
            "commit_hash": null,
            "description": "E2E sync test",
            "started_at": "2026-07-15T10:00:00Z",
            "ended_at": "2026-07-15T12:00:00Z",
            "task_id": null,
            "estimated_seconds": null,
            "labels": null
        }]);

        let response = request
            .post("/api/sync")
            .add_header(prepare_data::auth_header(&user.token))
            .json(&payload)
            .await;

        assert_eq!(response.status_code(), 200, "sync failed: {:?}", response.text());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_list_sessions() {
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;

        // First sync a session
        let payload = serde_json::json!([{
            "id": "list-test-1",
            "project_name": "nerdtime",
            "started_at": "2026-07-15T10:00:00Z",
            "ended_at": "2026-07-15T12:00:00Z"
        }]);

        request
            .post("/api/sync")
            .add_header(prepare_data::auth_header(&user.token))
            .json(&payload)
            .await;

        let response = request
            .get("/api/sessions")
            .add_header(prepare_data::auth_header(&user.token))
            .await;

        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = serde_json::from_str(&response.text()).unwrap();
        let sessions = body.as_array().unwrap_or(&vec![]);
        assert!(!sessions.is_empty());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_list_sessions_project_filter() {
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;

        let payload = serde_json::json!([{
            "id": "filter-test-1",
            "project_name": "website",
            "started_at": "2026-07-15T10:00:00Z",
            "ended_at": "2026-07-15T12:00:00Z"
        }]);

        request
            .post("/api/sync")
            .add_header(prepare_data::auth_header(&user.token))
            .json(&payload)
            .await;

        let response = request
            .get("/api/sessions?project=website")
            .add_header(prepare_data::auth_header(&user.token))
            .await;

        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = serde_json::from_str(&response.text()).unwrap();
        let sessions = body.as_array().unwrap_or(&vec![]);
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0]["project_name"], "website");
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_list_sessions_limit() {
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;

        for i in 0..3 {
            let payload = serde_json::json!([{
                "id": format!("limit-test-{}", i),
                "project_name": "nerdtime",
                "started_at": format!("2026-07-{:02}T10:00:00Z", 15 - i),
                "ended_at": format!("2026-07-{:02}T12:00:00Z", 15 - i),
            }]);

            request
                .post("/api/sync")
                .add_header(prepare_data::auth_header(&user.token))
                .json(&payload)
                .await;
        }

        let response = request
            .get("/api/sessions?limit=2")
            .add_header(prepare_data::auth_header(&user.token))
            .await;

        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = serde_json::from_str(&response.text()).unwrap();
        let sessions = body.as_array().unwrap_or(&vec![]);
        assert_eq!(sessions.len(), 2);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_stats() {
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;

        let payload = serde_json::json!([{
            "id": "stats-test-1",
            "project_name": "nerdtime",
            "started_at": "2026-07-15T10:00:00Z",
            "ended_at": "2026-07-15T12:00:00Z"
        }]);

        request
            .post("/api/sync")
            .add_header(prepare_data::auth_header(&user.token))
            .json(&payload)
            .await;

        let response = request
            .get("/api/stats")
            .add_header(prepare_data::auth_header(&user.token))
            .await;

        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = serde_json::from_str(&response.text()).unwrap();
        let stats = body.as_array().unwrap_or(&vec![]);
        assert!(!stats.is_empty());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_stats_requires_auth() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/stats").await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_list_sessions_requires_auth() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/sessions").await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}
