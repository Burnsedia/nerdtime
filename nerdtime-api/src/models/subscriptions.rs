// SPDX-License-Identifier: AGPL-3.0-only
use chrono::{DateTime, Utc};
use loco_rs::prelude::*;
use sea_orm::ActiveValue;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use super::_entities::subscriptions::{self, ActiveModel, Entity, Model};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingSettings {
    pub enabled: bool,
    pub stripe_secret_key: String,
    pub stripe_webhook_secret: String,
    pub price_id: String,
    pub success_url: String,
    pub cancel_url: String,
}

impl BillingSettings {
    pub fn from_settings(val: &Option<serde_json::Value>) -> Self {
        let v = match val {
            Some(v) => v.clone(),
            None => return Self::default(),
        };
        Self {
            enabled: v
                .get("billing")
                .and_then(|b| b.get("enabled"))
                .and_then(|e| e.as_bool())
                .unwrap_or(false),
            stripe_secret_key: v
                .get("billing")
                .and_then(|b| b.get("stripe_secret_key"))
                .and_then(|s| s.as_str().map(String::from))
                .unwrap_or_default(),
            stripe_webhook_secret: v
                .get("billing")
                .and_then(|b| b.get("stripe_webhook_secret"))
                .and_then(|s| s.as_str().map(String::from))
                .unwrap_or_default(),
            price_id: v
                .get("billing")
                .and_then(|b| b.get("price_id"))
                .and_then(|s| s.as_str().map(String::from))
                .unwrap_or_default(),
            success_url: v
                .get("billing")
                .and_then(|b| b.get("success_url"))
                .and_then(|s| s.as_str().map(String::from))
                .unwrap_or_else(|| "http://localhost:5150/settings".to_string()),
            cancel_url: v
                .get("billing")
                .and_then(|b| b.get("cancel_url"))
                .and_then(|s| s.as_str().map(String::from))
                .unwrap_or_else(|| "http://localhost:5150/settings".to_string()),
        }
    }
}

impl Default for BillingSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            stripe_secret_key: String::new(),
            stripe_webhook_secret: String::new(),
            price_id: String::new(),
            success_url: "http://localhost:5150/settings".to_string(),
            cancel_url: "http://localhost:5150/settings".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingInfo {
    pub tier: String,
    pub status: String,
    pub is_active: bool,
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, _db: &C, _insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        Ok(self)
    }
}

impl Model {
    pub async fn find_by_stripe_customer_id(
        db: &DatabaseConnection,
        customer_id: &str,
    ) -> ModelResult<Self> {
        Entity::find()
            .filter(subscriptions::Column::StripeCustomerId.eq(customer_id))
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)
    }

    pub async fn find_by_user(db: &DatabaseConnection, user_id: Uuid) -> ModelResult<Self> {
        Entity::find()
            .filter(subscriptions::Column::UserId.eq(user_id))
            .one(db)
            .await?
            .ok_or_else(|| ModelError::EntityNotFound)
    }

    pub async fn find_or_create(db: &DatabaseConnection, user_id: Uuid) -> ModelResult<Self> {
        match Self::find_by_user(db, user_id).await {
            Ok(sub) => Ok(sub),
            Err(_) => ActiveModel {
                user_id: ActiveValue::Set(user_id),
                status: ActiveValue::Set("active".to_string()),
                tier: ActiveValue::Set("free".to_string()),
                ..Default::default()
            }
            .insert(db)
            .await
            .map_err(ModelError::from),
        }
    }

    pub fn is_active(&self) -> bool {
        self.status == "active" || self.status == "trialing" || self.tier == "free"
    }

    pub fn billing_info(&self) -> BillingInfo {
        BillingInfo {
            tier: self.tier.clone(),
            status: self.status.clone(),
            is_active: self.is_active(),
        }
    }

    pub async fn update_stripe(
        db: &DatabaseConnection,
        user_id: Uuid,
        customer_id: &str,
        subscription_id: &str,
        status: &str,
        period_end: Option<DateTime<Utc>>,
    ) -> ModelResult<Self> {
        let sub = Self::find_or_create(db, user_id).await?;
        let mut model: ActiveModel = sub.into();
        model.stripe_customer_id = ActiveValue::Set(Some(customer_id.to_string()));
        model.stripe_subscription_id = ActiveValue::Set(Some(subscription_id.to_string()));
        model.status = ActiveValue::Set(status.to_string());
        model.tier = ActiveValue::Set("pro".to_string());
        model.current_period_end = ActiveValue::Set(period_end.map(|d| d.into()));
        model.update(db).await.map_err(ModelError::from)
    }

    pub async fn set_tier(
        db: &DatabaseConnection,
        user_id: Uuid,
        tier: &str,
        status: &str,
    ) -> ModelResult<Self> {
        let sub = Self::find_or_create(db, user_id).await?;
        let mut model: ActiveModel = sub.into();
        model.tier = ActiveValue::Set(tier.to_string());
        model.status = ActiveValue::Set(status.to_string());
        model.update(db).await.map_err(ModelError::from)
    }
}
