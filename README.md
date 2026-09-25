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

### First admin

New accounts are always `CUSTOMER`. Sign in once (any app), then promote yourself:

```sh
cargo run -p backend -- promote-admin +919876543210
```

After that, assign `PICKER` / `RIDER` / `ADMIN` from web-admin → Staff. Each app only
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

## Checks

```sh
pnpm lint        # ESLint (type-aware) + cargo fmt --check + clippy -D warnings
pnpm typecheck   # runs typegen first, then tsc in every package
pnpm test        # cargo test incl. #[sqlx::test] integration tests (needs db:up)
pnpm build

# End-to-end (needs db:up, emulators and the backend running as above)
pnpm e2e:auth                                  # API: phone OTP -> sync -> roles
pnpm --filter @vigo/web-admin e2e              # Playwright: admin login + role assignment
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
