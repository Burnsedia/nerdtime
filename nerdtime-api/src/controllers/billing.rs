// SPDX-License-Identifier: AGPL-3.0-only
use axum::http::HeaderMap;
use loco_rs::prelude::*;
use std::sync::OnceLock;
use stripe::{
    BillingPortalSession, CheckoutSession, CreateBillingPortalSession, CreateCheckoutSession,
    EventObject, EventType, Webhook,
};
use uuid::Uuid;

use crate::models::subscriptions::{self, BillingSettings};

fn stripe_client(secret: &str) -> &'static stripe::Client {
    static CLIENT: OnceLock<stripe::Client> = OnceLock::new();
    CLIENT.get_or_init(|| stripe::Client::new(secret))
}

async fn require_billing(ctx: &AppContext) -> Result<BillingSettings> {
    let settings = BillingSettings::from_settings(&ctx.config.settings);
    if !settings.enabled {
        return bad_request("billing is not enabled on this server");
    }
    Ok(settings)
}

fn extract_customer_id(customer: &Option<stripe::Expandable<stripe::Customer>>) -> Option<String> {
    match customer {
        Some(stripe::Expandable::Id(id)) => Some(id.to_string()),
        Some(stripe::Expandable::Object(obj)) => Some(obj.id.to_string()),
        None => None,
    }
}

fn extract_customer_id_from_expandable(customer: &stripe::Expandable<stripe::Customer>) -> String {
    match customer {
        stripe::Expandable::Id(id) => id.to_string(),
        stripe::Expandable::Object(obj) => obj.id.to_string(),
    }
}

/// Create a Stripe Checkout Session and return the URL.
pub async fn create_checkout(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let settings = require_billing(&ctx).await?;

    let user_id = match Uuid::parse_str(&auth.claims.pid) {
        Ok(id) => id,
        Err(_) => return unauthorized("invalid user"),
    };

    let sub = subscriptions::Model::find_or_create(&ctx.db, user_id).await?;
    let customer_id = sub.stripe_customer_id.clone().unwrap_or_default();

    let client = stripe_client(&settings.stripe_secret_key);

    let mut params = CreateCheckoutSession::new();
    params.mode = Some(stripe::CheckoutSessionMode::Subscription);
    params.success_url = Some(&settings.success_url);
    params.cancel_url = Some(&settings.cancel_url);
    params.line_items = Some(vec![stripe::CreateCheckoutSessionLineItems {
        price: Some(settings.price_id.clone()),
        quantity: Some(1),
        ..Default::default()
    }]);

    let uid = user_id.to_string();

    if customer_id.is_empty() {
        params.client_reference_id = Some(&uid);
        params.customer_creation = Some(stripe::CheckoutSessionCustomerCreation::Always);
    } else {
        params.customer = Some(
            customer_id
                .parse::<stripe::CustomerId>()
                .map_err(|_| Error::string("invalid customer id"))?,
        );
    }

    let session = CheckoutSession::create(client, params)
        .await
        .map_err(|e| Error::string(&format!("stripe request failed: {}", e)))?;

    format::json(serde_json::json!({"url": session.url}))
}

/// Handle Stripe webhook events (checkout complete, subscription updates).
pub async fn webhook(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    body: String,
) -> Result<Response> {
    let settings = BillingSettings::from_settings(&ctx.config.settings);
    if !settings.enabled {
        return bad_request("billing is not enabled on this server");
    }

    let sig_header = match headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
    {
        Some(h) => h,
        None => return bad_request("missing stripe signature"),
    };

    let event = Webhook::construct_event(&body, sig_header, &settings.stripe_webhook_secret)
        .map_err(|e| Error::string(&format!("webhook verification failed: {}", e)))?;

    match event.type_ {
        EventType::CheckoutSessionCompleted => {
            let session = match event.data.object {
                EventObject::CheckoutSession(s) => s,
                _ => return bad_request("unexpected event object type"),
            };

            let customer_id = extract_customer_id(&session.customer).unwrap_or_default();
            let sub_id = session.id.to_string();
            let status = "active";
            let client_ref = session.client_reference_id;

            if let Some(user_id_str) = &client_ref {
                if let Ok(user_id) = Uuid::parse_str(user_id_str) {
                    subscriptions::Model::update_stripe(
                        &ctx.db,
                        user_id,
                        &customer_id,
                        &sub_id,
                        status,
                        None,
                    )
                    .await?;
                }
            } else if !customer_id.is_empty() {
                if let Ok(existing) =
                    subscriptions::Model::find_by_stripe_customer_id(&ctx.db, &customer_id).await
                {
                    subscriptions::Model::update_stripe(
                        &ctx.db,
                        existing.user_id,
                        &customer_id,
                        &sub_id,
                        status,
                        None,
                    )
                    .await?;
                }
            }
        }
        EventType::CustomerSubscriptionUpdated | EventType::CustomerSubscriptionCreated => {
            let sub = match event.data.object {
                EventObject::Subscription(s) => s,
                _ => return bad_request("unexpected event object type"),
            };

            let customer_id = extract_customer_id_from_expandable(&sub.customer);
            let sub_id = sub.id.to_string();
            let status = sub.status.to_string();
            let period_end = Some(
                chrono::DateTime::from_timestamp(sub.current_period_end, 0).unwrap_or_default(),
            );

            if !customer_id.is_empty() {
                if let Ok(existing) =
                    subscriptions::Model::find_by_stripe_customer_id(&ctx.db, &customer_id).await
                {
                    subscriptions::Model::update_stripe(
                        &ctx.db,
                        existing.user_id,
                        &customer_id,
                        &sub_id,
                        &status,
                        period_end,
                    )
                    .await?;
                }
            }
        }
        EventType::CustomerSubscriptionDeleted => {
            let sub = match event.data.object {
                EventObject::Subscription(s) => s,
                _ => return bad_request("unexpected event object type"),
            };

            let customer_id = extract_customer_id_from_expandable(&sub.customer);
            if !customer_id.is_empty() {
                if let Ok(existing) =
                    subscriptions::Model::find_by_stripe_customer_id(&ctx.db, &customer_id).await
                {
                    subscriptions::Model::set_tier(&ctx.db, existing.user_id, "free", "canceled")
                        .await?;
                }
            }
        }
        _ => {}
    }

    format::json(serde_json::json!({"received": true}))
}

/// Redirect to Stripe Customer Portal.
pub async fn portal(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let settings = require_billing(&ctx).await?;

    let user_id = match Uuid::parse_str(&auth.claims.pid) {
        Ok(id) => id,
        Err(_) => return unauthorized("invalid user"),
    };

    let sub = subscriptions::Model::find_or_create(&ctx.db, user_id).await?;
    let customer_id = match sub.stripe_customer_id {
        Some(ref c) => c.clone(),
        None => return bad_request("no stripe customer found"),
    };

    let client = stripe_client(&settings.stripe_secret_key);

    let cid: stripe::CustomerId = customer_id
        .parse()
        .map_err(|_| Error::string("invalid customer id"))?;

    let mut params = CreateBillingPortalSession::new(cid);
    params.return_url = Some(&settings.success_url);

    let session = BillingPortalSession::create(client, params)
        .await
        .map_err(|e| Error::string(&format!("stripe request failed: {}", e)))?;

    format::json(serde_json::json!({"url": session.url}))
}

/// Get current billing info for the authenticated user.
pub async fn billing_info(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let user_id = match Uuid::parse_str(&auth.claims.pid) {
        Ok(id) => id,
        Err(_) => return unauthorized("invalid user"),
    };

    let sub = subscriptions::Model::find_or_create(&ctx.db, user_id).await?;
    format::json(sub.billing_info())
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/billing")
        .add("/checkout", post(create_checkout))
        .add("/webhook", post(webhook))
        .add("/portal", get(portal))
        .add("/info", get(billing_info))
}
