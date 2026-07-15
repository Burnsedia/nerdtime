// SPDX-License-Identifier: AGPL-3.0-only
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        db.execute_unwrap("ALTER TABLE sessions ADD COLUMN task_id TEXT;")
            .await?;
        db.execute_unwrap("ALTER TABLE sessions ADD COLUMN estimated_seconds BIGINT;")
            .await?;
        db.execute_unwrap("ALTER TABLE sessions ADD COLUMN labels TEXT;")
            .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        db.execute_unwrap("ALTER TABLE sessions DROP COLUMN task_id;")
            .await?;
        db.execute_unwrap("ALTER TABLE sessions DROP COLUMN estimated_seconds;")
            .await?;
        db.execute_unwrap("ALTER TABLE sessions DROP COLUMN labels;")
            .await?;
        Ok(())
    }
}
