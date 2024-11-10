FROM mcr.microsoft.com/devcontainers/rust

WORKDIR /usr/src/app

COPY . .

RUN cargo build --release --target x86_64-unknown-linux-gnu

COPY /usr/src/app/target/release/actota-api /usr/local/bin/actota-api

EXPOSE 8080

CMD [actota-api]
