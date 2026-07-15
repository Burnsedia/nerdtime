// SPDX-License-Identifier: AGPL-3.0-only
#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_users;
mod m20260101_000001_sessions;
mod m20260103_000001_subscriptions;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_users::Migration),
            Box::new(m20260101_000001_sessions::Migration),
            Box::new(m20260103_000001_subscriptions::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
