
ARG RUST_VERSION=1.76.0
ARG APP_NAME=redis_bot

FROM rust:${RUST_VERSION}-alpine AS builder
ARG APP_NAME
WORKDIR /app
COPY src/ /app/src
COPY Cargo.toml /app
RUN apk add --no-cache clang lld musl-dev git
RUN cargo install cargo-chef
RUN cargo chef prepare --recipe-path recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
RUN cargo build --release && \
    cp ./target/release/$APP_NAME /bin/$APP_NAME

CMD ["sh", "-c", "/bin/${APP_NAME}"]
