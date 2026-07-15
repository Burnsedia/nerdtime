// SPDX-License-Identifier: AGPL-3.0-only
use sea_orm::Statement;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        db.execute(Statement::from_string(
            db.get_database_backend(),
            "ALTER TABLE sessions ADD COLUMN task_id TEXT;".to_string(),
        ))
        .await?;
        db.execute(Statement::from_string(
            db.get_database_backend(),
            "ALTER TABLE sessions ADD COLUMN estimated_seconds BIGINT;".to_string(),
        ))
        .await?;
        db.execute(Statement::from_string(
            db.get_database_backend(),
            "ALTER TABLE sessions ADD COLUMN labels TEXT;".to_string(),
        ))
        .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        let db = m.get_connection();
        db.execute(Statement::from_string(
            db.get_database_backend(),
            "ALTER TABLE sessions DROP COLUMN task_id;".to_string(),
        ))
        .await?;
        db.execute(Statement::from_string(
            db.get_database_backend(),
            "ALTER TABLE sessions DROP COLUMN estimated_seconds;".to_string(),
        ))
        .await?;
        db.execute(Statement::from_string(
            db.get_database_backend(),
            "ALTER TABLE sessions DROP COLUMN labels;".to_string(),
        ))
        .await?;
        Ok(())
    }
}
