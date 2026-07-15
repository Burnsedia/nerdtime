// SPDX-License-Identifier: AGPL-3.0-only
use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "subscriptions",
            &[
                ("id", ColType::PkAuto),
                ("user_id", ColType::Uuid),
                ("stripe_customer_id", ColType::StringNull),
                ("stripe_subscription_id", ColType::StringNull),
                ("status", ColType::String),
                ("tier", ColType::String),
                ("current_period_end", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;

        m.create_index(
            Index::create()
                .name("idx_subscriptions_stripe_customer_id")
                .table(Alias::new("subscriptions"))
                .col(Alias::new("stripe_customer_id"))
                .to_owned(),
        )
        .await?;

        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "subscriptions").await?;
        Ok(())
    }
}
