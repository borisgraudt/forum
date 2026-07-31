# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS builder
WORKDIR /app

# Cache deps
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs \
  && cargo build --release \
  && rm -rf src

COPY src ./src
COPY migrations ./migrations
COPY rustfmt.toml ./rustfmt.toml
# Force rebuild of the real binary after dummy main
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/* \
  && useradd --system --create-home --uid 10001 forum

WORKDIR /app
COPY --from=builder /app/target/release/forum-backend /usr/local/bin/forum-backend
COPY --from=builder /app/migrations /app/migrations

RUN mkdir -p /data && chown -R forum:forum /data /app
USER forum

ENV HOST=0.0.0.0 \
    PORT=3000 \
    DATABASE_URL=sqlite:/data/forum.db?mode=rwc \
    RUST_LOG=info,forum_backend=info \
    COOKIE_SECURE=false

EXPOSE 3000
VOLUME ["/data"]

CMD ["forum-backend"]
