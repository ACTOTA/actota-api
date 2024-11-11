FROM mcr.microsoft.com/devcontainers/rust

WORKDIR /usr/src/app

COPY . .

RUN cargo build --release --target x86_64-unknown-linux-gnu

RUN ls -l /usr/src/app/target/x86_64-unknown-linux-gnu/release

RUN cp /usr/src/app/target/x86_64-unknown-linux-gnu/release/actota-api /usr/local/bin/actota-api

EXPOSE 8080

CMD ["/usr/local/bin/actota-api"]
