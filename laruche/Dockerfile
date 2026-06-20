# =============================================================
#  LaRuche Node - Docker Image
# Multi-stage build for minimal image size
# =============================================================

# Stage 1: Build
FROM rust:1.77-slim-bookworm AS builder

WORKDIR /build
COPY . .

RUN apt-get update && apt-get install -y pkg-config libssl-dev && \
    cargo build --release -p laruche-node -p laruche-cli -p laruche-dashboard

# Stage 2: Runtime
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Copy binaries
COPY --from=builder /build/target/release/laruche-node /usr/local/bin/
COPY --from=builder /build/target/release/laruche /usr/local/bin/
COPY --from=builder /build/target/release/laruche-dashboard /usr/local/bin/

# Default environment
ENV LARUCHE_NAME=laruche-docker \
    LARUCHE_TIER=core \
    LARUCHE_MODEL=mistral \
    LARUCHE_PORT=8419 \
    LARUCHE_DASH_PORT=8420 \
    OLLAMA_URL=http://host.docker.internal:11434

# Expose ports
EXPOSE 8419 8420

# mDNS requires host networking for discovery
# Run with: docker run --network host laruche-node
CMD ["laruche-node"]
