# Stage 1: Build
FROM rust:1.91-trixie AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --bin rest

# Stage 2: Runtime
FROM debian:trixie-slim AS runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -r -s /bin/false mcpclient

WORKDIR /app

COPY --from=builder /app/target/release/rest /app/rest
COPY config /app/config

RUN chown -R mcpclient:mcpclient /app

USER mcpclient

ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

ENTRYPOINT ["/app/rest"]
