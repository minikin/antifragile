FROM rust:1.85-slim AS builder

WORKDIR /build

RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy manifests and create dummy sources to cache dependency builds
COPY Cargo.toml /build/antifragile/Cargo.toml
RUN mkdir -p /build/antifragile/src && echo "pub trait Antifragile {}" > /build/antifragile/src/lib.rs

COPY examples/adaptive-pricing-api/Cargo.toml /build/example/Cargo.toml
RUN mkdir -p /build/example/src && echo "fn main() {}" > /build/example/src/main.rs

WORKDIR /build/example
RUN sed -i 's|path = "../../"|path = "/build/antifragile"|' Cargo.toml
RUN cargo build --release || true

# Copy real source and rebuild (only this layer reruns on code changes)
COPY src /build/antifragile/src
COPY examples/adaptive-pricing-api/src /build/example/src
RUN touch /build/antifragile/src/lib.rs /build/example/src/main.rs && cargo build --release

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/example/target/release/adaptive-pricing-api /app/adaptive-pricing-api

EXPOSE 3000

ENV RUST_LOG=info

HEALTHCHECK --interval=10s --timeout=5s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

CMD ["/app/adaptive-pricing-api"]
