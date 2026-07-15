// SPDX-License-Identifier: AGPL-3.0-only
use loco_rs::prelude::*;
use uuid::Uuid;

use crate::models::subscriptions::{self, BillingSettings};

fn stripe_request(
    method: reqwest::Method,
    path: &str,
    body: Option<serde_json::Value>,
    secret: &str,
) -> reqwest::RequestBuilder {
    let client = reqwest::Client::new();
    let url = format!("https://api.stripe.com/v1{}", path);
    let req = client
        .request(method, &url)
        .header("Authorization", format!("Bearer {}", secret))
        .header("Stripe-Version", "2025-02-24.acacia");

    match body {
        Some(json) => req.form(&json),
        None => req,
    }
}

async fn require_billing(ctx: &AppContext) -> Result<BillingSettings> {
    let settings = BillingSettings::from_settings(&ctx.config.settings);
    if !settings.enabled {
        return bad_request("billing is not enabled on this server");
    }
    Ok(settings)
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

    let mut body = serde_json::Map::new();
    body.insert(
        "success_url".to_string(),
        serde_json::Value::String(settings.success_url.clone()),
    );
    body.insert(
        "cancel_url".to_string(),
        serde_json::Value::String(settings.cancel_url.clone()),
    );
    body.insert(
        "mode".to_string(),
        serde_json::Value::String("subscription".to_string()),
    );
    body.insert(
        "line_items[0][price]".to_string(),
        serde_json::Value::String(settings.price_id.clone()),
    );
    body.insert(
        "line_items[0][quantity]".to_string(),
        serde_json::Value::Number(1.into()),
    );

    if !customer_id.is_empty() {
        body.insert(
            "customer".to_string(),
            serde_json::Value::String(customer_id),
        );
    } else {
        body.insert(
            "client_reference_id".to_string(),
            serde_json::Value::String(user_id.to_string()),
        );
        body.insert(
            "customer_creation".to_string(),
            serde_json::Value::String("always".to_string()),
        );
    }

    let resp = stripe_request(
        reqwest::Method::POST,
        "/checkout/sessions",
        Some(serde_json::Value::Object(body)),
        &settings.stripe_secret_key,
    )
    .send()
    .await
    .map_err(|e| Error::string(&format!("stripe request failed: {}", e)))?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| Error::string(&format!("stripe parse failed: {}", e)))?;

    let url = json.get("url").and_then(|u| u.as_str()).unwrap_or("");

    format::json(serde_json::json!({"url": url}))
}

/// Handle Stripe webhook events (checkout complete, subscription updates).
pub async fn webhook(
    State(ctx): State<AppContext>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Result<Response> {
    let settings = BillingSettings::from_settings(&ctx.config.settings);
    if !settings.enabled {
        return bad_request("billing is not enabled on this server");
    }

    let signature = match headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
    {
        Some(s) => s.to_string(),
        None => return bad_request("missing stripe signature"),
    };

    // Verify webhook signature
    let parts: Vec<&str> = signature.split(',').collect();
    let mut timestamp = String::new();
    let mut sig = String::new();
    for part in &parts {
        if let Some(t) = part.strip_prefix("t=") {
            timestamp = t.to_string();
        } else if let Some(s) = part.strip_prefix("v1=") {
            sig = s.to_string();
        }
    }

    let expected = hmac_sha256(
        &settings.stripe_webhook_secret,
        &format!("{}.{}", timestamp, body),
    );
    if sig != expected {
        return bad_request("invalid webhook signature");
    }

    let event: serde_json::Value =
        serde_json::from_str(&body).map_err(|_| Error::string("invalid webhook payload"))?;

    let event_type = event.get("type").and_then(|t| t.as_str()).unwrap_or("");

    match event_type {
        "checkout.session.completed"
        | "customer.subscription.updated"
        | "customer.subscription.created" => {
            let data = &event["data"]["object"];
            let customer_id = data.get("customer").and_then(|c| c.as_str()).unwrap_or("");
            let sub_id = data.get("id").and_then(|i| i.as_str()).unwrap_or("");
            let status = data
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("active");
            let period_end = data
                .get("current_period_end")
                .and_then(|t| t.as_i64())
                .map(|ts| {
                    let secs = ts as i64;
                    chrono::DateTime::from_timestamp(secs, 0).unwrap_or_default()
                });

            let client_ref = event_type
                .starts_with("checkout")
                .then(|| {
                    event["data"]["object"]["client_reference_id"]
                        .as_str()
                        .map(String::from)
                })
                .flatten()
                .or_else(|| {
                    // For subscription events, find user by customer_id
                    None
                });

            if let Some(ref user_id_str) = client_ref {
                if let Ok(user_id) = Uuid::parse_str(user_id_str) {
                    subscriptions::Model::update_stripe(
                        &ctx.db,
                        user_id,
                        customer_id,
                        sub_id,
                        status,
                        period_end,
                    )
                    .await?;
                }
            } else {
                // Look up by stripe_customer_id for non-checkout subscription events
                if !customer_id.is_empty() {
                    if let Ok(sub) =
                        subscriptions::Model::find_by_stripe_customer_id(&ctx.db, customer_id).await
                    {
                        subscriptions::Model::update_stripe(
                            &ctx.db,
                            sub.user_id,
                            customer_id,
                            sub_id,
                            status,
                            period_end,
                        )
                        .await?;
                    }
                }
            }
        }
        "customer.subscription.deleted" => {
            let data = &event["data"]["object"];
            let customer_id = data.get("customer").and_then(|c| c.as_str()).unwrap_or("");
            if !customer_id.is_empty() {
                if let Ok(sub) =
                    subscriptions::Model::find_by_stripe_customer_id(&ctx.db, customer_id).await
                {
                    subscriptions::Model::set_tier(&ctx.db, sub.user_id, "free", "canceled")
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

    let body = serde_json::json!({
        "customer": customer_id,
        "return_url": settings.success_url,
    });

    let resp = stripe_request(
        reqwest::Method::POST,
        "/billing_portal/sessions",
        Some(body),
        &settings.stripe_secret_key,
    )
    .send()
    .await
    .map_err(|e| Error::string(&format!("stripe request failed: {}", e)))?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| Error::string(&format!("stripe parse failed: {}", e)))?;

    let url = json.get("url").and_then(|u| u.as_str()).unwrap_or("");
    format::json(serde_json::json!({"url": url}))
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

fn hmac_sha256(secret: &str, payload: &str) -> String {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC key");
    mac.update(payload.as_bytes());
    let result = mac.finalize();
    let code = result.into_bytes();
    hex::encode(code)
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/billing")
        .add("/checkout", post(create_checkout))
        .add("/webhook", post(webhook))
        .add("/portal", get(portal))
        .add("/info", get(billing_info))
}
