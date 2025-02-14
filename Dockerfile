FROM mcr.microsoft.com/devcontainers/rust

WORKDIR /usr/src/app

COPY . .

RUN cargo build --release --target x86_64-unknown-linux-gnu

RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

RUN ls -l /usr/src/app/target/x86_64-unknown-linux-gnu/release

RUN cp /usr/src/app/target/x86_64-unknown-linux-gnu/release/actota-api /usr/local/bin/actota-api

EXPOSE 8080

CMD ["/usr/local/bin/actota-api"]
