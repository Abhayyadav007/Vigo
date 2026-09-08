# Architecture Decisions — vigo

Locked decisions for the vigo quick-commerce project.
Do not reopen these without a written reason. Add new decisions at the bottom.

| # | Decision | Status | Date |
|---|----------|--------|------|
| 1 | Authentication — phone + OTP, self-owned | ✅ Locked | 2026-09-08 |
| 2 | Payment gateway | ⬜ Open | — |
| 3 | Expo (dev builds) vs bare React Native | ⬜ Open | — |
| 4 | One app with role switch vs two apps | ⬜ Open | — |
| 5 | Hosting / deployment target | ⬜ Open | — |

---

## 1. Authentication — phone + OTP, self-owned

**Status:** ✅ Locked — 2026-09-08

### Decision

Own our auth. Do not outsource identity to Firebase, Clerk, Auth0, or Supabase.
Issue our own JWTs against our own `customers` table.

The stack:

1. **MSG91** as the SMS provider — India-focused, cheaper than Twilio domestically,
   good delivery rates, and they assist with DLT registration.
   Revisit Twilio only if we need multi-country coverage.
2. **Own OTP logic in the Rust backend** — a table with hashed OTP + TTL,
   a verify endpoint, and JWT issuance.
3. **Truecaller SDK** in the React Native app as a one-tap fast path,
   with OTP as the fallback. Add this *after* OTP works end to end — not before.

### Why

- Outsourcing identity puts a foreign user ID in our schema and creates a second
  source of truth for who a customer is.
- Per-MAU billing scales badly for a consumer app.
- We already run our own Postgres and Rust backend; a managed auth provider is
  redundant infrastructure.

### Consequences

- We are responsible for OTP TTL, rate limiting, retry logic, and fraud handling.
- **DLT registration is on our critical path** (see action items).
- Truecaller is an optimization, not a dependency — the app must work fully without it.

### Implementation rules

- 6-digit OTP, 5-minute TTL, max 3 verify attempts, 30-second resend cooldown.
- Store OTPs **hashed**, never in plaintext.
- Rate limit on three axes: per phone number, per IP, per device ID.
- Guard against SMS pumping fraud — cap requests per number per day, alert on
  unusual country/prefix patterns.
- Android: use the SMS Retriever API. The 11-char app hash must be included in
  the DLT-approved template — get this right the first time.
- iOS: keyboard autofill works automatically if the message contains "code".
- **Reserve a test number with a fixed OTP** (e.g. +919999999999 -> 123456).
  App Store and Play reviewers cannot receive our SMS and will reject the app
  without this.
- Long-lived refresh token in `react-native-keychain`, short-lived access token.
  A grocery app that logs users out weekly is dead.

### Action items

- [ ] **Start DLT registration now** — 1-2 weeks of pure calendar time.
      Entity registration (needs GST/incorporation docs), Sender ID (6 chars,
      e.g. `VIGOIN`), and exact-match message templates.
- [ ] Build against a stubbed SMS sender that logs the OTP to console until
      DLT clears.
- [ ] Swap in the real MSG91 client once the templates are approved.

---

## 2. Payment gateway

**Status:** ⬜ Open

Candidates: Razorpay, Cashfree, Stripe.
Key criterion: quality of UPI intent flow in the React Native SDK — UPI is
expected to be 70-80% of volume. Merchant KYC takes 1-3 weeks, so decide early.

## 3. Expo (dev builds) vs bare React Native

**Status:** ⬜ Open

Leaning Expo with dev builds unless a required native module rules it out.
Ejecting later is possible.

## 4. One app with role switch vs two apps

**Status:** ⬜ Open

Leaning two apps (customer + rider) sharing a `packages/api-client`, since the
flows diverge completely.

## 5. Hosting / deployment target

**Status:** ⬜ Open

Leaning Fly.io + Neon for lowest friction. Shapes the Dockerfile and CI, so
decide before writing the deploy workflow.
