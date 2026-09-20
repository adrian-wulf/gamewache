FROM rust:slim AS builder

WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev cmake build-essential && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY web ./web

RUN cargo build --release

FROM debian:trixie-slim

WORKDIR /app
RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/gamewache /app/gamewache

ENV PORT=8090
ENV HOST=0.0.0.0
ENV GAMEWACHE_DATA_DIR=/data

VOLUME ["/data"]
EXPOSE 8090

HEALTHCHECK --interval=30s --timeout=3s \
  CMD curl -f http://localhost:8090/api/v1/cas/stats || exit 1

CMD ["/app/gamewache", "serve", "--port", "8090", "--host", "0.0.0.0", "--data-dir", "/data"]
