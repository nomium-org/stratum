FROM rust:1.82 AS build

RUN apt-get update && apt-get install -y musl-tools

WORKDIR /stratum
COPY ./ .

WORKDIR /stratum/roles/pool
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --target x86_64-unknown-linux-musl --release

FROM scratch

WORKDIR /app
ENV SHARES_LOGGER__CLICKHOUSE__BATCH_SIZE="5"
ENV SHARES_LOGGER__CLICKHOUSE__BATCH_FLUSH_INTERVAL_SECS="60"
ENV RUST_LOG=info
COPY --from=build /stratum/roles/target/x86_64-unknown-linux-musl/release/pool_sv2 /app/pool_sv2
COPY --from=build /stratum/roles/pool/config-examples/pool-config-hosted-tp-example.toml /app/pool-config-hosted-tp-example.toml
#COPY --from=build /stratum/roles/pool/config-examples/pool-config-local-tp-example.toml /app/pool-config-local-tp-example.toml

CMD ["/app/pool_sv2", "-c", "/app/pool-config-hosted-tp-example.toml"]
#CMD ["/app/pool_sv2", "-c", "/app/pool-config-local-tp-example.toml"]
