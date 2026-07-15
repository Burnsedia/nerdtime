use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "sessions",
            &[
                ("id", ColType::Uuid),
                ("user_id", ColType::Uuid),
                ("project_name", ColType::String),
                ("branch_name", ColType::StringNull),
                ("commit_hash", ColType::StringNull),
                ("description", ColType::TextNull),
                ("started_at", ColType::TimestampWithTimeZone),
                ("ended_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "sessions").await?;
        Ok(())
    }
}
