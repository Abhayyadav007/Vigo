# Architecture Decisions — vigo

Locked decisions for the vigo quick-commerce project.
Do not reopen these without a written reason. Add new decisions at the bottom.

## Scope — India only

**This application targets the Indian market exclusively.** No multi-country or
multi-currency support. This constrains every decision below:

- **Currency is INR only.** Store money as integer **paise**, never floats.
  Do not add a currency column "just in case".
- **Vendors are India-first**: MSG91 for SMS (not Twilio), Razorpay / Cashfree /
  PhonePe for payments (not Stripe).
- **UPI is the dominant rail** — expect 70-80% of payment volume. UPI Intent flow
  quality is the primary gateway selection criterion.
- **Indian regulatory work is on the critical path**: TRAI DLT registration for SMS,
  RBI card tokenization, GST on invoices.
- **Phone numbers are +91 only.** Validate on that assumption rather than building
  general international parsing.
- **Single timezone (IST).** No timezone handling needed for delivery slots or
  store hours.

---

| # | Decision | Status | Date |
|---|----------|--------|------|
| 1 | Authentication — phone + OTP, self-owned | ✅ Locked | 2026-09-08 |
| 2 | Payment gateway — Razorpay | ✅ Locked | 2026-09-08 |
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

## 2. Payment gateway — Razorpay

**Status:** ✅ Locked — 2026-09-08

### Decision

**Razorpay** as the payment gateway. Enable UPI Intent, cards, netbanking,
wallets, and **COD**.

### Why

- **Highest UPI success rate (~93%).** At q-commerce volumes a 2% gap in payment
  success is a straight 2% revenue gap, and on a Rs 400 order that is worth ~Rs 8 —
  roughly cancelling the entire fee difference between gateways.
- **Best React Native SDK and documentation.** Integration velocity matters when
  building solo and already paying a Rust velocity tax (see Decision on backend).
- **Mature refunds tooling**, which will be used daily for out-of-stock
  substitutions and cancellations.

Runner-up was **Cashfree** (lower card rates, best-in-class vendor payouts —
revisit when automating rider payouts). **PhonePe PG** rejected as sole gateway:
weak on cards, thin ops tooling. Consider as a second gateway for failover later.
**Stripe** rejected — weak UPI support in India.

### Cost reality

All rates exclude 18% GST (claimable as input tax credit if GST-registered).

| Gateway | Cards / Netbanking / Wallets | UPI | Setup | AMC |
|---------|------------------------------|-----|-------|-----|
| Razorpay | 2% | 2% | Rs 0 | Rs 0 |
| Cashfree | ~1.95% | 0-1.99% (plan/volume) | Rs 0 | Rs 0 |
| PhonePe  | ~1.99% | ~1.99% | Rs 0 | — |

**Zero-MDR on UPI is the NPCI network rate, not the aggregator's fee.** Razorpay
charges its 2% platform fee on UPI as well. UPI is not free through a gateway.

At Rs 400 AOV with 75% UPI / 20% cards / 5% COD:

| Scenario | Cost per order | At 1,000 orders/day |
|----------|----------------|---------------------|
| 2% flat on everything | Rs 8.97 | Rs 32.7 L/year |
| UPI negotiated to 0%, cards 2% | Rs 1.89 | Rs 6.9 L/year |

**The UPI platform fee is the entire game.** The gap between a 1.95% and a 2% card
rate is worth ~Rs 15,000/year; the UPI rate is worth ~Rs 26 lakh/year. Treat it as
a negotiation to be won before scaling, not a fixed cost.

### Implementation rules

- **UPI Intent flow only** — never Collect (manual VPA entry kills conversion) and
  never QR (pointless on mobile). Intent opens PhonePe / GPay / Paytm directly with
  the amount pre-filled.
- **The webhook is the source of truth, not the SDK callback.** Verify the
  signature, make it idempotent on payment ID, and only then move the order to
  `paid`. Users kill the app mid-payment; the webhook still arrives.
- Do **not** hand-roll `upi://pay?pa=...` deep links to avoid fees — no signed
  callback, no reconciliation, trivially spoofed. Disqualifying for physical goods.
- Keep **COD** enabled. It still matters in Indian q-commerce and costs no gateway
  fee — lean on it harder while the negotiated UPI rate is poor.
- Build against Razorpay **test mode** until KYC clears.

### Action items

- [ ] **Start Razorpay merchant KYC now** — 1-3 weeks calendar time, parallel to
      DLT registration (Decision 1). Both clocks should already be running.
- [ ] Activate to capture the **0% platform fee offer** (merchants activating on or
      after 1 July 2026: free until Rs 5 lakh cumulative GMV or 90 days, whichever
      comes first) — this covers the beta window.
- [ ] **Before that offer expires**, get written quotes from both Razorpay and
      Cashfree and negotiate the **UPI rate specifically** — not the blended or card
      rate. Cite a projected 75%+ UPI share; use Cashfree's advertised
      "UPI from 0%" as leverage.

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
