FROM rust:1.83-slim-bookworm AS builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev libsqlite3-dev && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY nerdtime-core nerdtime-core/
COPY nerdtime-api nerdtime-api/

RUN cargo build --release -p nerdtime-api && \
    cp target/release/nerdtime_api-cli /app/nerdtime-api

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/nerdtime-api /usr/local/bin/nerdtime-api
COPY --from=builder /app/nerdtime-api/config /app/config
COPY --from=builder /app/nerdtime-api/assets /app/assets

WORKDIR /app
EXPOSE 3000
CMD ["/usr/local/bin/nerdtime-api", "start"]
