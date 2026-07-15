use loco_rs::prelude::*;
use nerdtime_core::SyncPayload;
use uuid::Uuid;

use crate::models::sessions;

pub async fn sync_sessions(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Json(payload): Json<Vec<SyncPayload>>,
) -> Result<Response> {
    let user_id = match Uuid::parse_str(&auth.claims.pid) {
        Ok(id) => id,
        Err(_) => return unauthorized("invalid user"),
    };

    for session in &payload {
        sessions::Model::upsert_sync(&ctx.db, user_id, session)
            .await?;
    }

    format::json(serde_json::json!({"status": "ok", "count": payload.len()}))
}

pub async fn list_sessions(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Query(params): Query<ListParams>,
) -> Result<Response> {
    let user_id = match Uuid::parse_str(&auth.claims.pid) {
        Ok(id) => id,
        Err(_) => return unauthorized("invalid user"),
    };

    let sessions = sessions::Model::find_by_user(
        &ctx.db,
        user_id,
        params.project.as_deref(),
        params.limit.unwrap_or(50),
    )
    .await?;

    format::json(sessions)
}

pub async fn get_stats(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user_id = match Uuid::parse_str(&auth.claims.pid) {
        Ok(id) => id,
        Err(_) => return unauthorized("invalid user"),
    };

    let stats = sessions::Model::stats_by_user(&ctx.db, user_id).await?;
    format::json(stats)
}

pub async fn health() -> Result<Response> {
    format::json(serde_json::json!({"status": "ok"}))
}

#[derive(Debug, serde::Deserialize)]
pub struct ListParams {
    pub project: Option<String>,
    pub limit: Option<u64>,
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/sync", post(sync_sessions))
        .add("/sessions", get(list_sessions))
        .add("/stats", get(get_stats))
        .add("/health", get(health))
}
