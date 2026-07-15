# Stripe SDK Migration — Implementation Plan

## Overview

Replace ~270 lines of raw `reqwest` + manual `hmac/sha2/hex` webhook verification with the typed `async-stripe` SDK (~150 lines). Eliminates 4 crate dependencies. Hardens error handling — no more `.unwrap_or("")` on JSON fields.

## Motivation

| Concern | Current approach | SDK approach |
|---|---|---|
| Type safety | Raw `serde_json::Value` with string fallbacks | Full Rust types for every Stripe object |
| Webhook verification | 50 lines of manual HMAC parsing | `Webhook::construct_event()` — one line |
| Error handling | `.unwrap_or("")` silently swallows missing fields | `Option<T>` or `Result<T, StripeError>` |
| API changes | Manual URL construction | SDK tracks Stripe API changes |
| Build time | 4 small crates | 1 large crate (~net slower but simpler) |

## Dependencies

In `nerdtime-api/Cargo.toml`:

```diff
- reqwest = { version = "0.12", features = ["json"] }
- hmac = "0.12"
- sha2 = "0.10"
- hex = "0.4"
+ async-stripe = { version = "0.41", default-features = false, features = [
+   "runtime-tokio-hyper-rustls",
+   "checkout",
+   "billing",
+   "webhook",
+ ] }
```

Feature rationale:
- `runtime-tokio-hyper-rustls` — matches project's tokio runtime, no openssl
- `checkout` — `CheckoutSession` create/read
- `billing` — `BillingPortalSession` create
- `webhook` — `Webhook::construct_event()` + `Event` type

## Architecture

```
┌──────────────────────┐
│   BillingSettings     │  ← unchanged (config YAML + env vars)
│   (stripe_secret_key) │
└──────┬───────────────┘
       │
       ▼
┌──────────────────────┐
│   stripe_client()     │  ← new: per-boot lazy OnceLock
│   &'static StripeClient │
└──────┬───────────────┘
       │
       ▼
┌──────────────────────┐
│  Controller handlers  │  ← rewritten with SDK types
│  create_checkout      │
│  webhook              │
│  portal               │
│  billing_info         │  ← unchanged
└──────────────────────┘
```

### StripeClient initialization

```rust
use std::sync::OnceLock;
use stripe::StripeClient;

fn stripe_client(secret: &str) -> &'static StripeClient {
    static CLIENT: OnceLock<StripeClient> = OnceLock::new();
    CLIENT.get_or_init(|| StripeClient::new(secret))
}
```

Called from `require_billing()` — lazy init on first real billing request. The `&'static` borrow is safe because the client is never dropped or recreated.

Webhook secret stays in `BillingSettings` (the struct is cheap to clone) and is passed directly to `Webhook::construct_event()` each call.

## Endpoint rewrites

### `POST /api/billing/checkout`

Current: 35 lines of `serde_json::Map` + raw POST → SDK.

```rust
use stripe::{
    CheckoutSession, CreateCheckoutSession, CheckoutSessionMode,
    CreateCheckoutSessionLineItems, SubscriptionData, Currency,
};

let mut params = CreateCheckoutSession::new();
params.mode = Some(CheckoutSessionMode::Subscription);
params.success_url = Some(&settings.success_url);
params.cancel_url = Some(&settings.cancel_url);
params.line_items = Some(vec![/* price, quantity */]);
params.client_reference_id = Some(&user_id.to_string());
params.customer_creation = Some(/* always */);
// If returning customer: params.customer = Some(customer_id);

let session = CheckoutSession::create(client, params).await?;
format::json(json!({"url": session.url}))
```

No `unwrap_or("")` — `session.url` is `Option<String>`; if `None`, the error propagates.

### `POST /api/billing/webhook`

Current: 100 lines of manual HMAC + JSON scraping → SDK.

```rust
use stripe::{Event, EventType, Webhook, CheckoutSession, Subscription};

let sig_header = headers
    .get("stripe-signature")
    .and_then(|v| v.to_str().ok())
    .ok_or_else(|| bad_request("missing stripe signature"))?;

let event = Webhook::construct_event(&body, sig_header, &settings.stripe_webhook_secret)
    .map_err(|e| Error::string(&format!("webhook verification failed: {}", e)))?;

match event.type_ {
    EventType::CheckoutSessionCompleted => {
        let session: CheckoutSession = event.data.object.deserialize()?;
        // session.customer, session.id, session.client_reference_id — all typed
    }
    EventType::CustomerSubscriptionUpdated
    | EventType::CustomerSubscriptionCreated => {
        let sub: Subscription = event.data.object.deserialize()?;
        // sub.customer, sub.id, sub.status, sub.current_period_end
    }
    EventType::CustomerSubscriptionDeleted => {
        // downgrade to free/canceled
    }
    _ => {} // ignore
}
```

No manual timestamp parsing, no hex encoding, no `unwrap_or("")`.

### `POST /api/billing/portal`

Current: raw POST → SDK.

```rust
use stripe::{BillingPortalSession, CreateBillingPortalSession};

let mut params = CreateBillingPortalSession::new(&customer_id);
params.return_url = Some(&settings.success_url);

let session = BillingPortalSession::create(client, params).await?;
format::json(json!({"url": session.url}))
```

### `GET /api/billing/info`

**No changes.** This endpoint reads from the local database only — no Stripe API call.

## Files changed

| File | Change | Lines delta |
|---|---|---|
| `nerdtime-api/Cargo.toml` | `-reqwest -hmac -sha2 -hex`, `+async-stripe` | ~0 |
| `nerdtime-api/src/controllers/billing.rs` | Full rewrite of 3 endpoints + webhook | ~270 → ~150 |

## Files not changed

- `nerdtime-api/src/models/subscriptions.rs` — `BillingSettings`, `UpdateStripe`, `SetTier`, `FindByStripeCustomerId` all stay
- `nerdtime-api/config/development.yaml` — same env var schema
- `nerdtime-api/src/app.rs` — route registration unchanged
- `nerdtime-api/src/controllers/mod.rs` — re-export unchanged

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| `EventType` enum missing our webhook events | Low | `_ => {}` catches unknown; our events are standard Stripe events |
| `Subscription.status` vs `CheckoutSession.mode` mismatch | Low | Both map 1:1 with Stripe API; SDK tracks latest API version |
| Build time increase | Medium | ~30s slower on cold build; only affects `nerdtime-api` crate |
| SDK panic on unexpected field types | Low | SDK uses `serde(deny_unknown_fields)` — if Stripe adds fields, SDK patch needed. Pin minor version. |

## Verification

1. `cargo build -p nerdtime-api` — compiles
2. `cargo clippy -p nerdtime-api -- -D warnings` — zero warnings
3. `cargo test -p nerdtime-api` — existing tests pass
4. Manual: start server with `BILLING_ENABLED=true`, hit checkout endpoint with valid Stripe key
5. Manual: `stripe trigger checkout.session.completed` → verify webhook handler runs

## Execution order

1. Swap deps in `Cargo.toml`
2. Add `stripe_client()` helper + imports
3. Rewrite `create_checkout`
4. Rewrite `webhook`
5. Rewrite `portal`
6. Delete `stripe_request` and `hmac_sha256`
7. Fix unused imports
8. `cargo build && cargo clippy`

## Estimate

~1.5 hours total.
