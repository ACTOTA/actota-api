FROM --platform=$BUILDPLATFORM mcr.microsoft.com/devcontainers/rust AS builder
WORKDIR /usr/src/app
COPY . .

# Install dependencies required for HTTP/2
RUN apt-get update && apt-get install -y \
    libssl-dev \
    ca-certificates \
    gcc \
    g++ \
    && rm -rf /var/lib/apt/lists/*

# Determine the target platform and install necessary Rust target
ARG TARGETPLATFORM
RUN case "$TARGETPLATFORM" in \
        "linux/amd64") \
            CARGO_TARGET="x86_64-unknown-linux-gnu" ;; \
        "linux/arm64") \
            CARGO_TARGET="aarch64-unknown-linux-gnu" ;; \
        *) \
            CARGO_TARGET="x86_64-unknown-linux-gnu" ;; \
    esac && \
    rustup target add $CARGO_TARGET && \
    echo "Building for target: $CARGO_TARGET" && \
    cargo build --release --target $CARGO_TARGET && \
    mkdir -p /build && \
    cp /usr/src/app/target/$CARGO_TARGET/release/actota-api /build/

# Create the final image - using Debian 12 (Bookworm) which has OpenSSL 3.x
FROM --platform=$TARGETPLATFORM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/actota-api /usr/local/bin/actota-api

EXPOSE 8080

# Set runtime environment variables
ENV RUST_LOG=actix_web=debug,actix_http=debug

CMD ["/usr/local/bin/actota-api"]
