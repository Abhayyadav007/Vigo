# syntax=docker/dockerfile:1.7
# Build from the repo root:  docker build -f apps/backend/Dockerfile -t vigo-backend .

FROM rust:1.96-slim-trixie AS build
WORKDIR /src
# Compile-time checked queries use the committed .sqlx cache; no DB needed.
ENV SQLX_OFFLINE=true
COPY Cargo.toml Cargo.lock ./
COPY .cargo .cargo
COPY .sqlx .sqlx
COPY apps/backend apps/backend
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p backend \
    && cp target/release/backend /usr/local/bin/backend

FROM debian:trixie-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --no-create-home vigo \
    && mkdir -p /data/media && chown vigo /data/media
COPY --from=build /usr/local/bin/backend /usr/local/bin/backend
USER vigo
ENV APP_ENV=production HOST=0.0.0.0 PORT=8080 MEDIA_DIR=/data/media
# Uploaded images (local media store). Mount a volume to keep them.
VOLUME /data/media
EXPOSE 8080
HEALTHCHECK --interval=15s --timeout=3s --start-period=10s --retries=3 \
    CMD curl -fsS "http://127.0.0.1:${PORT}/healthz" || exit 1
ENTRYPOINT ["/usr/local/bin/backend"]
