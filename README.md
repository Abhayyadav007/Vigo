# Vigo

Quick-commerce (10-minute delivery) platform for India: Axum backend, three Expo apps
(customer, picker, rider) and a Vite web admin, in one Cargo + pnpm/Turborepo monorepo.

## Prerequisites

Rust (stable, ≥ 1.90), Node ≥ 22, pnpm 10, Docker, Java 21 (Firebase emulator),
`sqlx-cli` (`cargo install sqlx-cli --no-default-features --features rustls,postgres`).

## Run locally

`.env.example` is set up for the **Firebase Auth emulator** (project `demo-vigo`),
so everything runs offline with no Firebase account. OTPs are not sent by SMS:
read them in the emulator UI at http://localhost:4000/auth.

```sh
cp .env.example .env
pnpm install
pnpm db:up                                  # Postgres 17 + PostGIS 3.5, Redis 7.4
pnpm emulators                              # Firebase Auth emulator :9099, UI :4000
pnpm typegen                                # Rust DTOs -> packages/types
pnpm --filter @vigo/backend dev             # http://localhost:8080/healthz (runs migrations on boot)
pnpm --filter @vigo/web-admin dev           # http://localhost:5173
```

### Demo data

```sh
cargo run -p backend -- seed-demo   # 2 Bengaluru dark stores, 6 categories, 24 products, stock
```

Idempotent. `EXPO_PUBLIC_DEV_LOCATION` in `.env.example` puts the customer app inside
the Indiranagar store's area so the catalog shows up without real GPS.

### First admin

New accounts are always `CUSTOMER`. Sign in once (any app), then promote yourself:

```sh
cargo run -p backend -- promote-admin +919876543210
```

After that, assign `PICKER` / `RIDER` / `ADMIN` from web-admin → Staff (pickers and
riders must be assigned to a store).

### Catalog (web-admin)

1. **Stores:** place the store on the map and draw its delivery area (or generate a
   hexagon of N km around it). Areas must be inside India and 0.05–150 km².
2. **Categories**, then **Products:** prices are entered in ₹ and stored as paise;
   selling price can't exceed MRP. Images (JPEG/PNG/WebP ≤ 5 MB) go to `MEDIA_DIR`.
3. **Inventory:** per store, set quantity, bin location, optional store price, and
   availability. Customers only see products their serving store carries. Each app only
admits its own role (customer app → CUSTOMER, picker → PICKER, rider → RIDER, admin → ADMIN).

### Mobile apps

The apps use `@react-native-firebase`, so they need a **development build** (Expo Go
won't work). Put `GoogleService-Info.plist` / `google-services.json` in
`apps/mobile-*/firebase/` (see the README there), then:

```sh
cd apps/mobile-customer
pnpm prebuild && pnpm ios        # or: pnpm android
pnpm dev                         # Metro for the dev client
```

### Real Firebase

Set `FIREBASE_PROJECT_ID` to your project, remove `FIREBASE_AUTH_EMULATOR_HOST`,
`VITE_FIREBASE_AUTH_EMULATOR_HOST` and `EXPO_PUBLIC_FIREBASE_AUTH_EMULATOR_HOST`, and
fill in the `VITE_FIREBASE_*` web config. Enable Phone sign-in in the Firebase console.

### Orders & payments

Checkout reserves stock atomically in Redis (Lua, all lines or nothing, 10-minute
hold), then commits it in Postgres when the order is confirmed: immediately for
cash on delivery, or on Razorpay's `payment.captured` webhook for online payment.
Unpaid reservations are released by a background sweeper. `POST /v1/customer/checkout`
requires an `Idempotency-Key` header, so retries never double-order.

Online payment is off unless `RAZORPAY_KEY_ID` and `RAZORPAY_WEBHOOK_SECRET` are set.
The Razorpay order-creation call is still a stub; the webhook
(`POST /v1/payments/razorpay/webhook`) verifies signatures and is idempotent per event.

### Picking (mobile-picker)

Pickers are assigned to a store in web-admin → Staff. The queue updates live over
`/v1/ws/picker` (authenticated with the Firebase token in the first message). A picker
claims an order (first one wins), scans each item with the camera or a hardware scanner
(keyboard wedge), marks anything missing, then packs it with a bag count and staging
slot. The customer is billed only for what was found; shelves found empty are zeroed.

## Checks

```sh
pnpm lint        # ESLint (type-aware) + cargo fmt --check + clippy -D warnings
pnpm typecheck   # runs typegen first, then tsc in every package
pnpm test        # cargo test incl. #[sqlx::test] integration tests (needs db:up)
pnpm build

# End-to-end (needs db:up, emulators and the backend running as above)
pnpm e2e:auth                                  # API: phone OTP -> sync -> roles
pnpm e2e:orders                                # API: address -> cart -> COD checkout -> cancel
pnpm e2e:picker                                # API + WebSocket: live queue -> claim -> scan -> pack
pnpm --filter @vigo/web-admin e2e              # Playwright: admin login, roles, catalog setup
```

After changing any SQL in `apps/backend/src/repositories`, refresh the offline cache
with `cargo sqlx prepare --workspace` and commit `.sqlx/`.

## Layout

| Path | What |
|---|---|
| `apps/backend` | Axum API. `routes/` wires paths, `handlers/` hold request logic, `repositories/` hold SQL, `cache/` holds Redis |
| `apps/mobile-{customer,picker,rider}` | Expo Router apps |
| `apps/web-admin` | Vite + React admin |
| `packages/types` | ts-rs output from `apps/backend/src/dto` (generated, never hand-edited) |
| `packages/api-client` | Axios client (token + 401 refresh), `AuthProvider`, Firebase adapters (`/native`, `/web`), React Query hooks |
| `packages/ui` | Theme tokens, Tailwind/NativeWind preset, shared RN components |
