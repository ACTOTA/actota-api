# Build stage - explicitly set to x86_64/amd64
FROM --platform=linux/amd64 rust:slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create a new empty shell project
WORKDIR /usr/src/app

# Copy your manifests and source code
COPY Cargo.toml ./
COPY src/ ./src/

# Build your application for release
RUN cargo build --release

# Final stage - also explicitly set to x86_64/amd64
FROM --platform=linux/amd64 debian:bookworm-slim

# Install only runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create a directory for any required configuration files
RUN mkdir -p /app/config

# Copy the build artifact from the builder stage
COPY --from=builder /usr/src/app/target/release/actota-api /usr/local/bin/actota-api

# Expose the port
EXPOSE 8080

# Set runtime environment variables
ENV RUST_LOG=actix_web=debug,actix_http=debug

# No credential environment variables needed
# The google-cloud-storage crate automatically uses Application Default Credentials

# Run the application
CMD ["/usr/local/bin/actota-api"]
