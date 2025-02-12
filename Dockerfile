FROM --platform=linux/amd64 rust:latest AS builder
WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

FROM --platform=linux/amd64 debian:bullseye-slim
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/app/target/release/actota-api /usr/local/bin/actota-api

# Environment variables
ENV HOST=0.0.0.0
ENV PORT=8080
ENV RUST_LOG=info

# Make sure the binary is executable
RUN chmod +x /usr/local/bin/actota-api

EXPOSE 8080

# Use ENTRYPOINT with CMD for better signal handling
ENTRYPOINT ["/usr/local/bin/actota-api"]

