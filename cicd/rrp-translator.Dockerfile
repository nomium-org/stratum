FROM rust:1.82 AS build

RUN apt-get update 
RUN apt-get install -y musl-tools

WORKDIR /stratum
COPY ./ .

WORKDIR /stratum/roles/translator
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --target x86_64-unknown-linux-musl --release

FROM alpine:latest

RUN apk add --no-cache bash iproute2

WORKDIR /app
COPY --from=build /stratum/cicd/connection_monitor.sh .
COPY --from=build /stratum/roles/target/x86_64-unknown-linux-musl/release/translator_sv2 .
COPY --from=build /stratum/roles/translator/config-examples/tproxy-config-local-pool-PoolTest.toml .

CMD ["/app/translator_sv2", "-c", "/app/tproxy-config-local-pool-PoolTest.toml"]
