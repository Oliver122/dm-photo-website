# syntax=docker/dockerfile:1

FROM rust:1.94-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY templates ./templates
COPY static ./static
COPY migrations ./migrations

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

RUN chown -R app:app /app

USER app

ENV SERVER_ADDR=0.0.0.0:8080
ENV STATIC_DIR=/app/static

EXPOSE 8080

CMD ["./dm-photo-website"]
