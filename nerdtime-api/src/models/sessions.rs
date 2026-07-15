// SPDX-License-Identifier: AGPL-3.0-only
use loco_rs::prelude::*;
use nerdtime_core::SyncPayload;
use sea_orm::{ActiveValue, QueryOrder, QuerySelect};
use uuid::Uuid;

pub use super::_entities::sessions::{self, ActiveModel, Entity, Model};

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if insert {
            Ok(self)
        } else {
            Ok(self)
        }
    }
}

impl Model {
    pub async fn upsert_sync(
        db: &DatabaseConnection,
        user_id: Uuid,
        payload: &SyncPayload,
    ) -> ModelResult<Self> {
        let existing = Entity::find()
            .filter(sessions::Column::Id.eq(payload.id))
            .filter(sessions::Column::UserId.eq(user_id))
            .one(db)
            .await?;

        if let Some(session) = existing {
            let mut model: ActiveModel = session.into();
            model.ended_at = ActiveValue::Set(payload.ended_at.map(Into::into));
            model.project_name = ActiveValue::Set(payload.project_name.clone());
            model.branch_name = ActiveValue::Set(payload.branch_name.clone());
            model.commit_hash = ActiveValue::Set(payload.commit_hash.clone());
            model.description = ActiveValue::Set(payload.description.clone());
            model.update(db).await.map_err(ModelError::from)
        } else {
            ActiveModel {
                id: ActiveValue::Set(payload.id),
                user_id: ActiveValue::Set(user_id),
                project_name: ActiveValue::Set(payload.project_name.clone()),
                branch_name: ActiveValue::Set(payload.branch_name.clone()),
                commit_hash: ActiveValue::Set(payload.commit_hash.clone()),
                description: ActiveValue::Set(payload.description.clone()),
                started_at: ActiveValue::Set(payload.started_at.into()),
                ended_at: ActiveValue::Set(payload.ended_at.map(Into::into)),
            }
            .insert(db)
            .await
            .map_err(ModelError::from)
        }
    }

    pub async fn find_by_user(
        db: &DatabaseConnection,
        user_id: Uuid,
        project: Option<&str>,
        limit: u64,
    ) -> ModelResult<Vec<Self>> {
        let mut query = Entity::find().filter(sessions::Column::UserId.eq(user_id));

        if let Some(project) = project {
            query = query.filter(sessions::Column::ProjectName.eq(project));
        }

        query
            .order_by_desc(sessions::Column::StartedAt)
            .limit(limit)
            .all(db)
            .await
            .map_err(ModelError::from)
    }

    pub async fn stats_by_user(
        db: &DatabaseConnection,
        user_id: Uuid,
    ) -> ModelResult<Vec<ProjectStats>> {
        let rows = Entity::find()
            .filter(sessions::Column::UserId.eq(user_id))
            .filter(sessions::Column::EndedAt.is_not_null())
            .all(db)
            .await?;

        let mut stats: std::collections::HashMap<String, ProjectStats> =
            std::collections::HashMap::new();

        for session in &rows {
            if let Some(end) = &session.ended_at {
                let start = &session.started_at;
                let duration = end.naive_utc() - start.naive_utc();
                let entry = stats
                    .entry(session.project_name.clone())
                    .or_insert_with(|| ProjectStats {
                        project_name: session.project_name.clone(),
                        total_seconds: 0,
                        session_count: 0,
                    });
                entry.total_seconds += duration.num_seconds();
                entry.session_count += 1;
            }
        }

        let mut result: Vec<ProjectStats> = stats.into_values().collect();
        result.sort_by(|a, b| b.total_seconds.cmp(&a.total_seconds));
        Ok(result)
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ProjectStats {
    pub project_name: String,
    pub total_seconds: i64,
    pub session_count: i64,
}
