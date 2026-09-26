# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Vigo is a quick-commerce (10-minute grocery delivery) platform for **India only**: phone numbers are `+91` mobiles (enforced in the backend and in a DB `CHECK`), money is INR stored as `BIGINT` paise, and payments are UPI-first (Razorpay). It is a monorepo built in numbered phases (see git history / PR #1). Unfinished work is marked `// TODO(phase-N):`. Keep that convention and don't stub anything silently.

Four clients, four roles: `mobile-customer` (CUSTOMER), `mobile-picker` (PICKER), `mobile-rider` (RIDER), `web-admin` (ADMIN).

`apps/user_backend` and `packages/db` are superseded leftovers. They are excluded from both the Cargo and pnpm workspaces. Don't edit them or build on them.

## Commands

Local services come from `docker compose` (Postgres 17 + PostGIS, Redis) plus the Firebase Auth emulator. `.env.example` is preconfigured for the emulator (`demo-vigo`), so no Firebase account is needed. Copy it to `.env` at the repo root; the backend finds it from any subdirectory, and the Expo apps load it through their `metro.config.js`.

```sh
pnpm install
pnpm db:up                                  # Postgres + Redis
pnpm emulators                              # Firebase Auth emulator :9099, UI :4000 (OTPs visible at /auth)
pnpm --filter @vigo/backend dev             # cargo run; applies migrations on boot
pnpm --filter @vigo/web-admin dev           # :5173
cargo run -p backend -- promote-admin +91XXXXXXXXXX   # bootstrap an ADMIN (user must have signed in once)
cargo run -p backend -- seed-demo                     # idempotent demo stores/catalog/stock (apps/backend/seeds/demo.sql)

pnpm lint | typecheck | test | build        # turbo, whole repo (typecheck/build run typegen first)
pnpm typegen                                # regenerate packages/types from Rust DTOs
```

Single tests:

```sh
cargo test -p backend --lib firebase                   # unit tests in one module
cargo test -p backend --test auth role_change          # one integration test (needs db:up)
cargo test -p backend --test catalog serviceability    # catalog/stores/inventory/uploads tests
cargo test -p backend --test orders concurrent         # checkout/overselling/payments/state machine
cargo test -p backend --test picker websocket          # picking flow + a real WebSocket session (TestApp::serve)
cargo test -p backend --test rider first_accept        # dispatch, OTP delivery, rider WS
pnpm e2e:rider                                         # packed -> live offer -> accept -> pickup -> OTP delivery
pnpm e2e:picker                                        # picking + live WS against a running backend
pnpm e2e:orders                                        # API purchase flow against a running backend (needs seed-demo)
pnpm --filter @vigo/api-client test                    # node:test unit tests (money/phone/media helpers)
pnpm --filter @vigo/api-client lint                    # ESLint for one package
pnpm e2e:auth                                          # API e2e (needs emulator + backend running)
pnpm --filter @vigo/web-admin exec playwright test -g "assign roles"   # browser e2e; runs `seed-demo` first, reuses a running Vite
# e2e tests shell out to the backend CLI: BACKEND_CLI (default `cargo run -q -p backend --`), PROMOTE_CMD for e2e-auth.mjs
scripts/smoke-test.sh [url]                            # black-box check of /healthz + error shape
```

Mobile apps use `@react-native-firebase`, so they need a **dev build** (`pnpm prebuild && pnpm ios|android` in the app dir, with `firebase/GoogleService-Info.plist` / `google-services.json`, which are gitignored). Expo Go won't work. To check JS without a device, run `CI=1 npx expo export --platform ios` in the app dir.

## Things that bite

- **SQLx compile-time queries.** `sqlx::query!` checks against the live DB whenever `DATABASE_URL` is set, and against the committed `.sqlx/` cache when `SQLX_OFFLINE=true` (Docker, typegen, CI). After adding a migration, run `sqlx migrate run --source apps/backend/migrations` before building. After changing any SQL, run `cargo sqlx prepare --workspace` and commit `.sqlx/`. CI fails on a stale cache (`prepare --check`).
- **Generated TS types.** DTOs in `apps/backend/src/dto/` derive `ts_rs::TS` with plain `#[ts(export)]`. The output directory comes from `TS_RS_EXPORT_DIR` in `.cargo/config.toml`, not `export_to`. After changing a DTO, run `pnpm typegen` and commit `packages/types/src/bindings/**` and `src/index.ts`. CI fails if they drift. Response DTOs use `#[serde(rename_all = "camelCase")]`. Any `i64` exposed to TS needs `#[ts(type = "number")]`, because ts-rs would otherwise emit `bigint`.
- **Lints.** Clippy denies `unwrap()` and `-D warnings` makes `expect()` an error too. Use `?`, `AppError` and `anyhow` context instead; test crates opt out with a crate-level `#![allow(...)]`. ESLint is type-aware and bans `any` (root `eslint.config.mjs`).
- **Money** is `i64` paise end to end. Only `api-client/src/money.ts` (`formatPaise`, `rupeesToPaise`) converts, without floats. Selling price ≤ MRP is enforced in the DB, in the service (including store overrides and MRP cuts), and in the admin form.
- **Background processes.** Don't leave dev servers, emulators or backends running after a check, and before stopping one, confirm it's yours: the user's own `pnpm … dev` processes show absolute paths.
- **Dependency versions** live in the pnpm `catalog:` in `pnpm-workspace.yaml`. React/RN versions follow Expo SDK 57 (TypeScript is 6.0 for that reason). `.npmrc` uses `node-linker=hoisted` for Metro.

## Backend architecture (`apps/backend`, Axum)

`lib.rs` exposes all modules, `main.rs` is a thin CLI (`serve` / `promote-admin`), and `tests/` builds the real router through `backend::app::build_router`.

Layering is strict:
- `routes/` only wires paths to handlers.
- `handlers/` parse and validate input, call services, and map results to DTOs. No SQL here.
- `services/` hold business rules and cache invalidation.
- `repositories/` hold all SQL (functions take `impl PgExecutor` so they work inside transactions).
- `cache/` is Redis, using the `deadpool_redis::redis` re-export and no direct `redis` dependency.
- `models/` are DB rows; `dto/` are the wire types.

**PostGIS.** Geometry never crosses into Rust as a PostGIS type. Repositories convert in SQL: GeoJSON in via `ST_GeomFromGeoJSON($1::jsonb::text)::geography`, out via `ST_AsGeoJSON(...)::jsonb` into `Json<GeoJsonPolygon>`, and points as `ST_Y/ST_X`. Serviceability is `ST_Covers` on the store polygon, and the nearest store wins when areas overlap. `GeoJsonPolygon::normalized` does the shape checks (single ring, closed, inside India's bbox); `ST_IsValid` and area limits run in `store_service`.

**Catalog visibility** (`repositories/catalog.rs`): a customer sees a product only if the store, product and category are active and the store has an `is_available` inventory row for it. Price is `coalesce(store override, base price)`. Search is `ILIKE` plus pg_trgm `word_similarity ≥ 0.4`, which handles typos. Customer catalog routes are public (no auth); admin routes use the `Admin` guard.

**Media.** `services/media_service.rs` has a `MediaStore` enum (only `Local` for now; R2/S3 is a TODO). Uploads are sniffed by magic bytes, stored under `MEDIA_DIR` (relative paths resolve against the `.env` directory) and served at `/media/...` by `ServeDir` in `app.rs`. URLs are stored relative, and clients resolve them with `resolveMediaUrl(url, apiBaseUrl)`.

**Stock, reservations, orders** (`cache/inventory.rs` + `cache/scripts/*.lua`, `services/order_service.rs`):
- **Redis keys.** Per store, Redis keeps `inv:{store}:stock` (a mirror of Postgres quantity) and `inv:{store}:held` (units reserved), plus one hash per reservation and a `resv_exp` zset. All keys share the `{store}` hash tag, so they stay Cluster-safe. The reservation id is the order id.
- **Checkout.** `reserve.lua` checks `stock - held >= qty` for every line and then applies all of them, or none. Missing mirror entries are loaded from Postgres with `HSETNX`, then the reservation is retried.
- **Confirm** (COD immediately, online on the webhook): a Postgres transaction runs conditional `UPDATE … WHERE quantity >= n` for each line, then `commit.lua`. Cancelling from PLACED runs `release.lua`; cancelling after CONFIRMED restocks Postgres and `add_stock`. Admin inventory edits call `set_stock`. The sweeper task in `main.rs` cancels expired unpaid orders and releases orphaned reservations.
- **Status changes** go only through `order_service` (`transition`, `confirm`, `cancel`). Each one is a compare-and-set in SQL (`status = ANY(predecessors)`), writes `order_status_events`, and publishes an `OrderStatusChanged` event to Redis channels `orders:{id}` and `stores:{storeId}:orders`. The legal transitions are `OrderStatus::can_transition_to` in `models/order.rs`.
- **Payments.** The `PaymentProvider` trait lives in `services/payments/`, and `Payments { cod, razorpay: Option }` sits on `AppState`. The Razorpay webhook verifies HMAC-SHA256 in constant time and dedupes on `payment_events(provider, event_id)`.
- **Checkout requests.** Checkout requires `Idempotency-Key`; orders are unique on `(user_id, idempotency_key)`. Orders snapshot prices, names and the address, so later catalog edits don't change them.

**WebSockets** (`ws/`):
- **Hub.** `PubSubHub` (on `AppState.hub`) holds one Redis Pub/Sub connection per instance. It fans messages out through tokio broadcast channels, ref-counts SUBSCRIBE/UNSUBSCRIBE, and reconnects and resubscribes on its own.
- **Sessions.** The first client message must be `{"type":"auth","token":…}`, sent within 10s; tokens never go in URLs. The server closes with 4001 (bad token), 4003 (wrong role or store) or 4008 (token expired). It sends `resync` when a client lags. Payloads are the `WsServerMessage` enum from `dto/ws.rs`.
- **Adding an endpoint.** Add an `Audience` variant that maps an authenticated session to a channel, plus a route in `routes/ws.rs`.
- **Client side.** `api-client/src/ws.ts` (`openLiveSocket`) reconnects with backoff, refreshes the token after 4008, stops after FORBIDDEN, and emits `resync` after every reconnect. `useLiveEvents(path, onMessage)` wraps it for React.

**Picking** (`services/picker_service.rs`):
- **Access.** Pickers see only their `store_id`; other stores' orders return 404. Every mutation requires PICKING and `picker_id == me`, and returns codes such as `NOT_YOUR_ORDER` and `ALREADY_CLAIMED`.
- **Scanning.** A scan is one atomic `UPDATE … picked_quantity + 1 … < quantity`. It answers 422 `SCAN_MISMATCH` or 409 `LINE_COMPLETE` via `AppError::Coded`.
- **Packing.** Packing re-bills `item_total` to the picked units (the delivery fee stays as charged) and zeroes the shelf for short lines. Cancelling restocks `picked_quantity` when the line was counted, otherwise the full `quantity`.

**Dispatch & delivery** (`services/dispatch_service.rs`, `services/rider_service.rs`, `cache/geo.rs`):
- **Positions.** Riders' fixes go to Redis `riders:{store}:geo` (GEOADD; searched with GEOSEARCH, never GEORADIUS) plus `riders:{store}:seen`. Riders with no fix within `RIDER_STALE_SECS`, offline riders (`rider_profiles.is_online`) and busy riders (an active `deliveries` row) are never offered work.
- **Offers.** Packing calls `dispatch_best_effort`, and `main.rs` runs a 5 s `dispatch_loop` for anything left over. Each wave offers the nearest `DISPATCH_WAVE_SIZE` untried riders, stored in `dsp:{order}:offered` with the offer TTL. `claim_offer.lua` decides the first accept (`dsp:{order}:winner`), then `accept` does the PACKED→RIDER_ASSIGNED compare-and-set and inserts the delivery. The partial unique indexes allow one active delivery per order and per rider. On failure the winner key is cleared.
- **Rider feed.** The rider channel `riders:{id}` carries pre-serialised `WsServerMessage`s (`offer`, `offerRevoked`); the WS session passes anything starting with `{"type"` straight through. `publish_status` also notifies the order's latest rider.
- **Delivery.** Pickup checks the bag count. The first fix more than 200 m from the store flips PICKED_UP→OUT_FOR_DELIVERY. Deliver requires: being within 500 m of the drop (when the fix is under 2 min old); the OTP, compared in constant time and locked after 5 wrong tries with 423 `OTP_LOCKED`; and the exact COD amount. The order ends DELIVERED, or PARTIALLY_FULFILLED if any line was picked short.
- **Cancellation.** `order_service::cancel` ends any active delivery and clears the winner.

Map unique/FK violations to client errors with `error::map_constraint(e, &[(constraint_name, On::Conflict|On::Invalid, message)])` instead of letting them 500.

**Errors.** Everything returns `AppResult<T>`. `AppError` renders `{ "error": { "code", "message" } }`, and 5xx details are logged, never returned. For input, use `extractors::{ValidJson, ValidQuery, PathParam, Pagination}` instead of axum's `Json`/`Query`/`Path`: those keep the JSON error shape and run `validator` rules.

**Auth flow.** Postgres is the source of truth for roles; the token only proves identity.
1. `FirebaseIdentity` verifies the Bearer Firebase ID token (`auth/firebase.rs`). It uses RS256 against Google's JWKS, cached per `Cache-Control: max-age`, and refetches on an unknown `kid` at most every 30s. A JWKS outage returns 503, not 401. Only `/v1/auth/sync` uses this extractor directly, because it creates users (new users are always `CUSTOMER`, and a phone that reappears under a new UID is re-linked).
2. `AuthUser` = verified token + session lookup: Redis `session:v1:{firebase_uid}`, falling back to Postgres. Unknown user → 403 `USER_NOT_REGISTERED`; disabled user → 403 `ACCOUNT_DISABLED`.
3. `RequireRole<R>` guards are aliased as `Customer`, `Picker`, `Rider` and `Admin`. Put the guard in the handler signature.
4. Any change to a user's role, `store_id` or `is_active` **must** call `auth_service::invalidate_best_effort`. Otherwise the old role survives until the cache TTL runs out.
5. `FIREBASE_AUTH_EMULATOR_HOST` switches to accepting unsigned emulator tokens. `Config::from_env` refuses this when `APP_ENV=production`. Tests use `FirebaseVerifier::with_static_keys`, and `tests/common` signs real RS256 tokens with keys from `tests/fixtures`.

Integration tests use `#[sqlx::test]`, which gives each test a fresh migrated database, plus the shared dev Redis. Keep keys unique per test (`random_phone()` and UUID-based UIDs do this).

## Client architecture

- **`packages/api-client`** is the only path to the backend. `createApiClient({ getIdToken })` attaches the token and retries once with a forced refresh on 401. It turns error bodies into `ApiError { status, code }`. `AuthProvider` (with `requiredRole`) listens to Firebase, calls `POST /v1/auth/sync` and signs out accounts with the wrong role. Firebase SDKs live behind subpath exports (`@vigo/api-client/native` for RN Firebase, `@vigo/api-client/web` for the JS SDK), so each app only bundles its own. The adapter interface uses function-typed properties so methods can be passed unbound.
- **`packages/ui`**: shared React Native components styled with NativeWind (v4, Tailwind 3). Design tokens live in `src/tokens.json`, which feeds both `theme.ts` and `tailwind-preset.js`; add colours there. Components take callbacks (e.g. `PhoneLoginForm` gets `onSendCode` / `normalizePhone`) and never import Firebase or the API.
- **Cart** (`useSetCartItem`): updates are optimistic and run in one mutation `scope` per store, so they apply serially; the server response replaces the optimistic guess. Checkout generates one `newIdempotencyKey()` per attempt and keeps it across network retries.
- **Expo apps** use Expo Router with `Stack.Protected` guards driven by `useAuth().state`. Typed routes are **off** on purpose, because the generated `.expo/types` is gitignored and would make local and CI type checks differ. Tabs come from `expo-router/js-tabs` (the `expo-router` export is deprecated). The customer app resolves location through `lib/location.tsx` (a TanStack query for GPS, then `useServiceability`); screens under `DeliveryGate` can call `useStore()`. `EXPO_PUBLIC_DEV_LOCATION=lat,lng` skips GPS. Firebase config plugins and `expo-build-properties` (`useFrameworks: static`) are in `app.json`; `app.config.ts` adds the Google services file paths.
- **web-admin**: Vite + React Router 7 (v8 needs a newer React than Expo pins). Firebase phone auth uses an invisible reCAPTCHA. The store map is Leaflet + Geoman, lazy-loaded. Geoman needs a global `L`, so always import Leaflet through `src/lib/leaflet.ts` before `@geoman-io/leaflet-geoman-free`: without it, the whole app renders blank. Editors are split into a loader plus a form keyed by id; the React Compiler lint rules reject copying fetched data into state inside effects.

## CI/CD (`.github/workflows/ci.yml`)

Runs on PRs and on pushes to `main`. Four jobs:
- **backend**: fmt, clippy, migrations, `sqlx prepare --check`, tests, and a smoke test of the binary.
- **frontend**: lint, typecheck and build, a typegen drift check, and Expo JS bundles for all apps.
- **e2e**: Firebase emulator + backend + `scripts/e2e-auth.mjs` + Playwright.
- **docker**: builds `apps/backend/Dockerfile` (from the repo root) and smoke-tests the image. On `main` it pushes the image to `ghcr.io/<owner>/vigo-backend`.

Postgres uses `imresamu/postgis`, because the official PostGIS image has no arm64 build.
