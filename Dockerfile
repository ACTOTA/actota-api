# Build stage - use buildplatform to adapt to the builder's architecture
FROM --platform=$BUILDPLATFORM rust:slim-bookworm AS builder
# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*
# Set up cross-compilation environment variables
ARG TARGETPLATFORM
RUN echo "Building for $TARGETPLATFORM"
# Set up the target based on build platform
RUN case "$TARGETPLATFORM" in \
    "linux/amd64") echo "x86_64-unknown-linux-gnu" > /tmp/target ;; \
    "linux/arm64") echo "aarch64-unknown-linux-gnu" > /tmp/target ;; \
    *) echo "Unsupported platform: $TARGETPLATFORM" && exit 1 ;; \
    esac
# Install target if cross-compiling
RUN if [ "$(uname -m)" != "$(cat /tmp/target | cut -d'-' -f1)" ]; then \
    rustup target add $(cat /tmp/target); \
    fi
# Create a new empty shell project
WORKDIR /usr/src/app
# Copy your manifests and source code
COPY Cargo.toml ./
COPY src/ ./src/
# Check the binary name in Cargo.toml
RUN grep -A 5 "\[\[bin\]\]" Cargo.toml || echo "No [[bin]] section found"
RUN grep "name" Cargo.toml | head -5

# Build your application for release with the correct target
RUN if [ "$(uname -m)" != "$(cat /tmp/target | cut -d'-' -f1)" ]; then \
    cargo build --release --target $(cat /tmp/target); \
    else \
    cargo build --release; \
    fi

# Verify the build output and find the binary
RUN find /usr/src/app/target -type f -executable | grep -v "\.d" || echo "No executables found"
RUN ls -la /usr/src/app/target/release/ || echo "Release directory not found"
RUN if [ -d "/usr/src/app/target/$(cat /tmp/target)/release/" ]; then \
    ls -la /usr/src/app/target/$(cat /tmp/target)/release/; \
else \
    echo "Cross-compilation target directory not found"; \
fi

# Final stage - use targetplatform for the final image
FROM --platform=$TARGETPLATFORM debian:bookworm-slim
# Install only runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    findutils \
    && rm -rf /var/lib/apt/lists/*
# Create a directory for configuration files
RUN mkdir -p /app/config


# Copy ALL release artifacts to make sure we get the binary
COPY --from=builder /usr/src/app/target /usr/src/target
# Find the actual binary and copy it to the right location
RUN find /usr/src/target -type f -executable | grep -v "\.d" || echo "No executables found in the final image"
# Copy the binary with the correct path (to be determined from the output)
# COPY --from=builder /usr/src/app/target/*/release/actota-api /usr/local/bin/actota-api

# This is a fallback to make sure we at least have a binary
RUN find /usr/src/target -type f -executable -not -path "*/\.*" | head -1 > /tmp/binary_path || echo "No binary found"
RUN if [ -s /tmp/binary_path ]; then \
    mkdir -p /usr/local/bin && \
    cp $(cat /tmp/binary_path) /usr/local/bin/actota-api && \
    chmod +x /usr/local/bin/actota-api && \
    echo "Copied $(cat /tmp/binary_path) to /usr/local/bin/actota-api"; \
else \
    echo "Failed to find any executable"; \
    exit 1; \
fi

# Expose the port
EXPOSE 8080
# Set runtime environment variables
ENV RUST_LOG=actix_web=debug,actix_http=debug

# Verify our binary exists
RUN ls -la /usr/local/bin/actota-api || echo "Binary not found in final location"

# Run the application
CMD ["/usr/local/bin/actota-api"]
