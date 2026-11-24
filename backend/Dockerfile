# =============================================================================
# JORNAL DIGITAL FEDERADO — Backend Rust/Axum (Multi-Stage Build)
# =============================================================================
FROM rust:1.99.0-slim-bookworm AS builder

WORKDIR /app

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml rust-toolchain.toml ./
COPY src ./src

RUN cargo build --release --bin jornal_backend

# -----------------------------------------------------------------------------
# Runtime enxuto e sem privilégio de root
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 10001 jornal \
    && useradd --system --uid 10001 --gid jornal --no-create-home jornal

WORKDIR /app

COPY --from=builder /app/target/release/jornal_backend /usr/local/bin/jornal_backend

ENV HOST=0.0.0.0 \
    PORT=8080 \
    RUST_LOG=info,jornal_backend=info

USER jornal

EXPOSE 8080

HEALTHCHECK --interval=10s --timeout=5s --start-period=10s --retries=5 \
    CMD curl -fsS http://127.0.0.1:8080/health || exit 1

CMD ["jornal_backend"]
