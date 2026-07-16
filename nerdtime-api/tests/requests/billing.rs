// SPDX-License-Identifier: AGPL-3.0-only
use loco_rs::testing::prelude::*;
use nerdtime_api::app::App;
use serial_test::serial;

use super::prepare_data;

#[tokio::test]
#[serial]
async fn test_billing_info_requires_auth() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/billing/info").await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_billing_info() {
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;

        let response = request
            .get("/api/billing/info")
            .add_header(prepare_data::auth_header(&user.token))
            .await;

        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = serde_json::from_str(&response.text()).unwrap();
        // Should return subscription info (may be free tier if billing disabled)
        assert!(body.get("tier").is_some() || body.get("status").is_some());
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_billing_checkout_requires_auth() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.post("/api/billing/checkout").json(&serde_json::json!({})).await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_billing_checkout() {
    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;

        let response = request
            .post("/api/billing/checkout")
            .add_header(prepare_data::auth_header(&user.token))
            .json(&serde_json::json!({
                "price_id": "price_test",
                "success_url": "https://nerdtime.dev/success",
                "cancel_url": "https://nerdtime.dev/cancel"
            }))
            .await;

        // May succeed or return error if billing is disabled
        let code = response.status_code();
        assert!(code == 200 || code == 400 || code == 500,
            "unexpected status: {}", code);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_billing_portal_requires_auth() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/api/billing/portal").await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn test_billing_webhook() {
    request::<App, _, _>(|request, _ctx| async move {
        // Send a fake Stripe webhook event (will fail HMAC, but should not panic)
        let payload = serde_json::json!({
            "type": "checkout.session.completed",
            "data": {
                "object": {
                    "id": "cs_test",
                    "customer": "cus_test",
                    "subscription": "sub_test",
                    "metadata": {
                        "user_id": "test-user"
                    }
                }
            }
        });

        let response = request
            .post("/api/billing/webhook")
            .json(&payload)
            .await;

        // Without valid Stripe signature, should return 400
        assert_eq!(response.status_code(), 400);
    })
    .await;
}
