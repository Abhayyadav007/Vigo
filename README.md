# Vigo

Quick-commerce (10-minute delivery) platform for India: Axum backend, three Expo apps
(customer, picker, rider) and a Vite web admin, in one Cargo + pnpm/Turborepo monorepo.

## Prerequisites

Rust (stable, ≥ 1.90), Node ≥ 22, pnpm 10, Docker, `sqlx-cli` (`cargo install sqlx-cli --no-default-features --features rustls,postgres`).

## Run locally

```sh
cp .env.example .env
pnpm install
pnpm db:up                                  # Postgres 17 + PostGIS 3.5, Redis 7.4
pnpm typegen                                # Rust DTOs -> packages/types
pnpm --filter @vigo/backend dev             # http://localhost:8080/healthz (runs migrations on boot)
pnpm --filter @vigo/web-admin dev           # http://localhost:5173
pnpm --filter @vigo/mobile-customer start   # Expo; switch to a dev build from phase 2
```

## Checks

```sh
pnpm typecheck   # runs typegen first, then tsc in every package
pnpm lint        # cargo fmt --check + clippy -D warnings
pnpm test
pnpm build
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
| `packages/api-client` | Axios client, React Query hooks |
| `packages/ui` | Theme tokens and shared RN components |
