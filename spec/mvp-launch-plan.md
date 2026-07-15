# MVP Launch Plan

## Overview

Ship nerdtime v1.0. The MVP is functionally complete (CLI + backend + billing gating).
This plan covers the remaining work to go from "works on my machine" to "people can pay for it."

## Prerequisites

- [ ] Stripe SDK migration (`spec/stripe-sdk-migration-plan.md`)
- [ ] `BILLING_ENABLED=false` is the default — safe for self-host

## Launch checklist

### Week 1: Production-hardening

- [ ] **Stripe SDK migration** — swap raw reqwest for `async-stripe`
- [ ] **Smoke test full flow**:
  - Deploy backend with `BILLING_ENABLED=true`
  - Register via API → get JWT
  - CLI `nerd login <token>` → `nerd start` → `nerd stop` → `nerd sync`
  - Check sessions appear in API
  - Run checkout flow (Stripe test mode)
  - Trigger webhooks with Stripe CLI
- [ ] **Set up production deployment**:
  - PostgreSQL (managed: Neon / Railway / Supabase)
  - Redis (managed: Upstash / Railway)
  - Deploy backend (Railway / Fly.io / your own VPS)
  - DNS: `api.nerdtime.app` → backend
  - ENV: `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`, `STRIPE_PRICE_ID`, `BILLING_ENABLED=true`

### Week 2: Landing page + docs

- [ ] **Landing page** at `nerdtime.app`:
  - Hero: "Terminal-native time tracking for developers"
  - Features: offline-first, git auto-detection, cloud sync
  - Pricing: CLI free, cloud sync $10/mo
  - Download: `curl -fsSL https://nerdtime.app/install.sh | sh`
  - Footer: GitHub link, AGPL license
- [ ] **Quickstart docs** (could just be the README polished):
  - Install
  - `nerd start/stop/status`
  - `nerd sync` (with login)
  - Self-host guide (docker-compose)
- [ ] **Install script**: `install.sh` that detects OS + arch, downloads release binary from GitHub
- [ ] **GitHub release**: tag `v1.0.0` with prebuilt binaries (Linux x86_64, macOS x86_64 + aarch64)

### Week 3: Soft launch

- [ ] Post to HN / Reddit (r/rust, r/programming)
- [ ] Monitor signups and Stripe payments
- [ ] Fix bugs as they come in
- [ ] Add GitHub Issues template for bug reports

## Post-launch success metrics

| Metric | Target |
|---|---|
| Signups (week 1) | > 50 |
| Paid conversions | > 5% |
| CLI downloads | > 500 |
| Active users (DAU) | > 10 |

## Revenue at launch

- $10/mo for cloud sync
- Self-host is free forever
- CLI is free forever
- No annual plans, no trials (Stripe Checkout handles it)
