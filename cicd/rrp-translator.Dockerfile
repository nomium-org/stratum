FROM rust:1.82 AS build

RUN apt-get update && apt-get install -y musl-tools

WORKDIR /stratum
COPY ./ .

WORKDIR /stratum/roles/translator
RUN which openssl
#RUN export OPENSSL_DIR=/usr/bin/openssl
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo check --target x86_64-unknown-linux-musl
RUN cargo build --target x86_64-unknown-linux-musl --release

FROM scratch

WORKDIR /app
ENV RUST_LOG=debug
COPY --from=build /stratum/roles/target/x86_64-unknown-linux-musl/release/translator_sv2 .
COPY --from=build /stratum/roles/translator/config-examples/tproxy-config-local-pool-PoolTest.toml .

CMD ["/app/translator_sv2", "-c", "/app/tproxy-config-local-pool-PoolTest.toml"]
