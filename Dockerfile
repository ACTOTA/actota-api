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

# Create a directory for configuration files
RUN mkdir -p /app/config

# Create a minimal but valid service account JSON structure
# This satisfies the format requirements without containing real credentials
RUN echo '{ \
  "type": "service_account", \
  "project_id": "dummy-project", \
  "private_key_id": "dummy", \
  "private_key": "-----BEGIN PRIVATE KEY-----\ndummy\n-----END PRIVATE KEY-----\n", \
  "client_email": "dummy@example.com", \
  "client_id": "dummy", \
  "auth_uri": "https://accounts.google.com/o/oauth2/auth", \
  "token_uri": "https://oauth2.googleapis.com/token", \
  "auth_provider_x509_cert_url": "https://www.googleapis.com/oauth2/v1/certs", \
  "client_x509_cert_url": "dummy" \
}' > /app/config/dummy-credentials.json

# Copy the build artifact from the builder stage
COPY --from=builder /usr/src/app/target/release/actota-api /usr/local/bin/actota-api

# Expose the port
EXPOSE 8080

# Set runtime environment variables
ENV RUST_LOG=actix_web=debug,actix_http=debug

# Point to our dummy credentials file to satisfy the cloud_storage crate
ENV GOOGLE_APPLICATION_CREDENTIALS=/app/config/dummy-credentials.json

# Set the same dummy credentials as SERVICE_ACCOUNT_JSON for double coverage
# This ensures the cloud_storage crate can find credentials in either location
ENV SERVICE_ACCOUNT_JSON='{ \
  "type": "service_account", \
  "project_id": "dummy-project", \
  "private_key_id": "dummy", \
  "private_key": "-----BEGIN PRIVATE KEY-----\ndummy\n-----END PRIVATE KEY-----\n", \
  "client_email": "dummy@example.com", \
  "client_id": "dummy", \
  "auth_uri": "https://accounts.google.com/o/oauth2/auth", \
  "token_uri": "https://oauth2.googleapis.com/token", \
  "auth_provider_x509_cert_url": "https://www.googleapis.com/oauth2/v1/certs", \
  "client_x509_cert_url": "dummy" \
}'

# Run the application
CMD ["/usr/local/bin/actota-api"]
