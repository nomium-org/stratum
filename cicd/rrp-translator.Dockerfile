FROM rust:1.82 AS build

RUN apt-get update 
RUN apt-get install -y musl-tools libssl-dev pkg-config
ENV OPENSSL_DIR=/usr \
    OPENSSL_INCLUDE_DIR=/usr/include \
    OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu

# Устанавливаем musl-версию OpenSSL
RUN apt-get install -y musl-dev && \
    curl -O https://www.openssl.org/source/openssl-1.1.1u.tar.gz && \
    tar -xzf openssl-1.1.1u.tar.gz && \
    cd openssl-1.1.1u && \
    CC=musl-gcc ./Configure no-shared no-dso no-hw no-engine linux-mips64 && \
    make && \
    make install && \
    cd .. && \
    rm -rf openssl-1.1.1u openssl-1.1.1u.tar.gz

WORKDIR /stratum
COPY ./ .

WORKDIR /stratum/roles/translator
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --target x86_64-unknown-linux-musl --release

FROM scratch

WORKDIR /app
ENV RUST_LOG=debug
COPY --from=build /stratum/roles/target/x86_64-unknown-linux-musl/release/translator_sv2 .
COPY --from=build /stratum/roles/translator/config-examples/tproxy-config-local-pool-PoolTest.toml .

CMD ["/app/translator_sv2", "-c", "/app/tproxy-config-local-pool-PoolTest.toml"]
