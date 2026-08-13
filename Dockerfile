# syntax=docker/dockerfile:1

FROM rust:1.94-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY templates ./templates
COPY static ./static
COPY migrations ./migrations

# REQ Tests sections must stay green in every image build.
RUN cargo test
RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 10001 app \
    && useradd --system --uid 10001 --gid app --home-dir /app --shell /usr/sbin/nologin app

WORKDIR /app

COPY --from=builder /app/target/release/dm-photo-website /app/dm-photo-website
COPY --from=builder /app/templates /app/templates
COPY --from=builder /app/static /app/static
COPY docker-entrypoint.sh /docker-entrypoint.sh

RUN mkdir -p /app/data/ingest \
    && chown -R app:app /app \
    && chmod +x /docker-entrypoint.sh

# Entrypoint starts as root to fix data/ ownership, then drops to app.
USER root

# Absolute persist paths (bind/volume → /app/data). Entrypoint rewrites legacy
# relative sqlite://data/... URLs that SQLite URI-parses incorrectly.
ENV SERVER_ADDR=0.0.0.0:8080
ENV DATABASE_URL=sqlite:/app/data/app.db
ENV ANALOG_INGEST_DIR=/app/data/ingest
ENV STATIC_DIR=static

EXPOSE 8080

ENTRYPOINT ["/docker-entrypoint.sh"]
CMD ["./dm-photo-website"]
