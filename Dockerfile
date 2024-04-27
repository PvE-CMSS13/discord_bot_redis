
ARG RUST_VERSION=1.76.0

FROM rust:${RUST_VERSION}-alpine AS builder
WORKDIR /app
COPY src/ /app/src
COPY Cargo.toml /app
RUN apk add --no-cache clang lld musl-dev git
RUN cargo install cargo-chef
RUN cargo chef prepare --recipe-path recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
RUN cargo build --release && \
    cp ./target/release/redis_bot /bin/redis_bot

CMD ["/bin/redis_bot"]
