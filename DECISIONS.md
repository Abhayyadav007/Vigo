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

## Business model — multi-vendor marketplace

**vigo is a marketplace of local shops, not a dark-store operation.** We do not own
inventory. Shop owners list their own items; customers browse open shops nearby and
order; our own riders deliver.

| Aspect | Model |
|--------|-------|
| Inventory | **Owned by shop owners.** They set price and stock. |
| Delivery | **Our own rider fleet.** We recruit, assign, and pay riders. |
| Cart scope | **One shop per order.** No mixed-shop carts. |
| Money | **We collect from the customer**, settle to shops on a cycle minus commission. |

Three sides to serve: **customers, shop owners, riders** — plus internal ops.

### Delivery promise — 10-20 minutes

The product is **Zomato's marketplace mechanics with Blinkit's speed promise**:
order from third-party shops, delivered in 10-20 minutes.

Time budget:

| Step | Budget |
|------|--------|
| Shop notices + accepts | 1-2 min (biggest risk) |
| Shop picks + packs | 3-5 min |
| Rider reaches shop | 2-4 min (**must overlap with picking**) |
| Travel to customer | 5-8 min (only within ~2-3 km) |
| Handover | ~1 min |
| **Total** | **12-20 min, only if steps overlap** |

**Why this is harder than Blinkit:** Blinkit owns the dark store — exact inventory,
aisles laid out for picking, trained pickers, riders parked outside. A kirana owner
serving walk-in customers has none of that, and may not look at their phone for
three minutes, burning 20% of the budget. **Shop responsiveness is our core
operational problem**, the way rider supply is Zomato's.

### What the promise forces into the design

1. **Tight radius: 2-3 km per shop**, not city-wide. Zomato serves 7 km because it
   promises 35 minutes. At 15 minutes geography is unforgiving. The `ST_DWithin`
   radius is a **product decision**, not a config value.
2. **Assign the rider in parallel with shop acceptance, never after.** Dispatch on
   successful payment so the rider travels while the shop picks. Serialising
   (accept -> find rider -> travel) blows the budget.
3. **Hard shop-acceptance timeout: 60-90 seconds**, then auto-reroute to another
   shop or auto-cancel with refund. Unbounded, one distracted owner produces a
   40-minute order.
4. **Shop SLA scoring is first-class.** Track acceptance rate and *actual* prep time
   per shop; rank shops in customer search by measured speed, not just distance.
   This is the lever that makes the model work — fast shops earn more orders, slow
   shops self-select out.

Schema implications: `shops` carries `avg_prep_seconds`, `acceptance_rate`,
`is_accepting_orders`, and `delivery_radius_m`. The order state machine needs
`pending_shop_acceptance` with a timeout, and assignment runs concurrently with it.

### Consequences

- **Catalog quality is the hardest problem.** Fifty shops each typing their own item
  names produces "Amul Milk 500ml" nine different ways. Browse and search quality
  depend on solving this — see the master-catalog approach in the schema.
- **Shop state is first-class**: open/closed, business hours, accepting-orders,
  suspended. "Show shops which are open" is a core query, not a detail.
- **Order flow gains a step**: the shop must accept and prepare before a rider is
  assigned. Shop rejection and timeout are real states to design for.
- **We hold customer money and owe it to shops** — that requires a payouts ledger
  and reconciliation, not just a payment integration.

---

| # | Decision | Status | Date |
|---|----------|--------|------|
| 1 | Authentication — phone + OTP, self-owned | ✅ Locked | 2026-09-08 |
| 2 | Payment gateway — Razorpay | ✅ Locked | 2026-09-08 |
| 3 | Mobile tooling — Expo with dev builds | ✅ Locked | 2026-09-08 |
| 4 | Mobile apps — three separate apps | ✅ Locked (revised 2026-09-09) | 2026-09-08 |
| 5 | Hosting — DigitalOcean, Bangalore (BLR1) | ✅ Locked | 2026-09-08 |

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

### Marketplace addendum (added 2026-09-09)

The business model is a marketplace: we collect from the customer and settle to
shop owners minus commission. That needs **split settlement**, not plain checkout:

- **Razorpay Route** — sub-merchant accounts, split a payment across shop + platform,
  scheduled settlements, and a transfers ledger.
- Cashfree's **Easy Split** is the equivalent. Cashfree's payouts strength — noted
  above as its edge — matters considerably more under this model than it did under
  the dark-store model.
- **Re-evaluate Razorpay vs Cashfree specifically on split-settlement and payout
  features before building checkout.** The gateway choice stands for now; the
  integration surface is different from what was originally scoped.
- Shop owners need onboarding as sub-merchants (their own KYC) — another calendar
  clock, per shop.

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

## 3. Mobile tooling — Expo with dev builds

**Status:** ✅ Locked — 2026-09-08

### Decision

**Expo with dev builds** (`expo prebuild` + a custom dev client), not bare React
Native and not Expo Go.

Note this is **not** the old "managed vs ejected" tradeoff. Dev builds support
**any** native module. The choice is Expo's tooling around RN vs raw RN.

### Why

1. **`expo-updates` gives OTA updates out of the box.** Shipping a pricing or
   checkout fix in hours instead of waiting on a 1-7 day store review is close to
   essential for q-commerce. In bare RN this is now awkward since CodePush retired.
   **This reason alone decides it.**
2. **RN version upgrades stop being a lost week.** Building solo, we cannot afford
   days a year on native upgrade diffs.
3. **Config plugins keep native config in version control as code** instead of
   hand-edited `Info.plist` / `AndroidManifest.xml` / Gradle that drifts.

### Consequences

- `android/` and `ios/` are **gitignored and regenerated** by prebuild. Never
  hand-edit them — write a config plugin instead, or the change is lost on the next
  prebuild.
- EAS Build free tier is limited. Local builds (`npx expo run:android` /
  `run:ios`) are free and we are on a Mac, so iOS builds work locally.
- One more abstraction layer when debugging native issues.

### Escape hatch

Run `expo prebuild`, commit `android/` and `ios/`, and we are effectively bare
while keeping Expo's modules. **We are never trapped.** This asymmetry — easy to
leave Expo, painful to adopt it later — is the core argument for starting here.

### SDK compatibility notes

- **Truecaller: solved.** `@dhana-cs/react-native-truecaller` ships a built-in Expo
  config plugin that configures `AndroidManifest.xml` from `app.json`. It
  implements exactly the flow in Decision 1 — Truecaller if installed, OTP
  fallback if not. Requires a dev build (not Expo Go) and does not work on
  emulators.
- **Razorpay: one open issue, but it is not an Expo problem.**
  `react-native-razorpay` does not yet support the **New Architecture**
  (razorpay/react-native-razorpay#510). New Arch has been the default in **bare RN
  since 0.76** as well, so this breaks identically either way — it is a
  "Razorpay's SDK is behind" problem, not an Expo-vs-bare differentiator.
  Mitigations (same in both worlds): rely on the New Arch interop layer, pin
  `newArchEnabled: false`, or fall back to Razorpay's WebView checkout.
- Ignore Razorpay's docs claiming "Expo doesn't support native code" — stale text
  from the Expo Go era.

### Action items

- [ ] **Spike before writing app code (~30 min):** scaffold a throwaway Expo app,
      `npx expo install react-native-razorpay @dhana-cs/react-native-truecaller`,
      `npx expo prebuild`, `npx expo run:android`. If Razorpay checkout opens,
      we are clear.
- [ ] Add `android/` and `ios/` to `.gitignore` from the first commit.

## 4. Mobile apps — three separate apps

**Status:** ✅ Locked — 2026-09-08

### Decision

**Revised 2026-09-09:** the marketplace model adds a third audience — shop owners —
so this is now **three separate Expo apps**, not two. The reasoning below is
unchanged and applies with more force.

```
frontend/
├── apps/
│   ├── customer/        # browse open shops, cart, checkout, track
│   ├── shop/            # NEW - shop owner: open/closed, catalog, price,
│   │                    #       stock, accept/reject orders, earnings
│   └── rider/           # go online, accept, pick up from shop, deliver
└── packages/
    ├── api-client/      # typed API client, auth interceptor, refresh logic
    ├── types/           # shared request/response types
    └── ui/              # shared primitives only - most UI is NOT shared
```

Plus **internal ops/admin on web** (Retool or similar initially, per the stack
notes) — shop approval, catalog moderation, order intervention, payout runs.

**The shop app is mobile, not web.** Shop owners are Android-first and need
reliable background push with an audible alert when an order arrives — a browser
tab cannot do that dependably.

**Scope warning:** three apps plus an admin surface is substantially more work than
the original two-app plan. See the phasing note below.

### Why

1. **Store review isolation.** With one app, a rejected rider feature blocks the
   customer release too. Separate listings mean separate review queues and
   independent release cadence.
2. **Background location permission.** The rider app needs continuous background
   location. Both Apple and Google scrutinise that permission heavily and demand
   justification. A **customer** app requesting background location invites
   rejection and user distrust. Two apps keeps that permission scoped to the app
   that genuinely needs it — this is the strongest single reason.
3. **The flows share almost nothing.** Catalog, cart, and checkout have no
   counterpart in the rider app; assignment, navigation, and delivery confirmation
   have none in the customer app. The real shared surface is the API client and
   auth — which is exactly what `packages/` covers.
4. **Bundle size and code leakage.** One app ships rider logic (and its internal
   assignment/ops semantics) to every customer install.

### Consequences

- Monorepo tooling: **pnpm workspaces** plus Expo's monorepo configuration
  (`metro.config.js` needs `watchFolders` and node_modules resolution set up).
- **Two app store listings, two EAS projects, two release pipelines.** More CI
  surface, but each is simpler than a combined one.
- Requires discipline about what belongs in `packages/` — resist sharing screens
  or navigation. Share the API client, types, and low-level primitives only.
- Rider app can lag behind: manual dispatch is viable at one dark store, so the
  rider app is deliberately scheduled after the customer flow works end to end.

### Action items

- [ ] Set up pnpm workspaces + Expo monorepo config before building the second app,
      not after.
- [ ] Build `packages/api-client` first, consumed by the customer app — the rider
      app inherits it for free.
- [ ] Keep three separate EAS project IDs and bundle identifiers from the start
      (`in.vigo.customer`, `in.vigo.shop`, `in.vigo.rider`) — renaming later means
      new store listings.

### Phasing — do not build all three at once

1. **Customer app first**, against seeded shop data entered via SQL.
2. **Shop app second** — this is what makes the marketplace real, and it is the
   app that unblocks onboarding actual shops.
3. **Rider app last.** Manual dispatch (a phone call, a WhatsApp group) is viable
   for the first handful of shops and buys weeks.

## 5. Hosting — DigitalOcean, Bangalore (BLR1)

**Status:** ✅ Locked — 2026-09-08

### Decision

**DigitalOcean, Bangalore (BLR1) region** for all backend infrastructure:
App Platform for the Rust service and worker, Managed PostgreSQL for the
database, Spaces (+ built-in CDN) for product images.

### Why

1. **~40% cheaper than Fly.io** for an equivalent setup. Cost discipline matters
   pre-revenue with thin q-commerce unit economics.
2. **One vendor for all four pieces** — compute, Postgres, object storage, CDN.
   Less to wire together when building solo.
3. **Flat, predictable monthly pricing** — no usage-based surprises, unlike Neon's
   CU-hours or Fly's metered egress.
4. **Bangalore is an India region**, which is the requirement. Mumbai vs Bangalore
   is ~15 ms domestically — invisible. What mattered was not being overseas.

### Rejected alternatives

- **Neon — ruled out: no India region.** Its Asia presence is Singapore. (An
  earlier draft of this file suggested Fly.io + Neon; that predates the India-only
  scope decision.)
- **Fly.io (Mumbai `bom`)** — nicer DX, but Managed Postgres starts at $38/mo
  (~3x DO's entry tier), India egress is $0.12/GB (their highest rate globally),
  and the free tier is gone.
- **AWS ap-south-1** — most capable, most complexity. Choose only if we already
  know AWS well.
- **Railway / Render / Hetzner — no India region.** Hetzner is exceptional value
  and completely wrong here: ~250 ms of ocean per request kills the feel of a
  10-minute-delivery app.

### Costs

Building blocks (before GST):

| Service | Price/mo | Spec |
|---------|----------|------|
| Droplet (Basic) | $6 / $12 / $18 | 1 GB / 2 GB / 2 GB-2vCPU |
| App Platform | $5 per service | dynamic services; static free |
| Managed Postgres | $15.15 / $30.45 / $60.90 | 1 / 2 / 4 GiB RAM |
| Spaces + CDN | $5 | 250 GiB storage + 1 TiB transfer |

Projected totals:

| Stage | Monthly | INR approx |
|-------|---------|------------|
| Development / beta | ~$25 | ~Rs 2,200 |
| Beta launch (customer app + worker) | ~$37 | ~Rs 3,300 |
| ~1,000 orders/day (HA, Redis) | ~$195 | ~Rs 17,000 |

**Perspective:** at 1,000 orders/day infrastructure is ~Rs 0.57/order, while the
Razorpay fee at standard rates is ~Rs 9/order. **The payment gateway costs ~15x
what the servers do** — optimise the UPI rate (Decision 2), not the hosting bill.

Add ~18% GST as an Indian customer; likely claimable as input tax credit if
GST-registered.

### Consequences

- Deploy target shapes the Dockerfile, the GitHub Actions deploy workflow, and
  secrets management. Settle these before writing `backend-deploy.yml`.
- Managed Postgres gives automated backups and point-in-time recovery — the reason
  we are not self-hosting Postgres for order and payment data.
- **PostGIS is supported** on DO Managed Postgres — verified as a selection
  criterion, since serviceability depends on it.
- Serve product images from Spaces' CDN, never through the app — true regardless
  of provider.

### Action items

- [ ] Create the DigitalOcean account and claim the **$200 / 60-day free credit**.
      This covers the entire build phase through the thin-slice demo.
- [ ] Provision in **BLR1** — do not accept a default US region.
- [ ] Enable the **PostGIS** extension on the managed database at provisioning.
- [ ] Optional cost saver during development only: run backend + worker + Postgres
      on a single $12 droplet via docker-compose. **Move Postgres to managed before
      taking a single real order.**
