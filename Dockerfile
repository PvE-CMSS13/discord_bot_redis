
ARG RUST_VERSION=1.76.0

FROM rust:${RUST_VERSION}-alpine AS chef
RUN apk add --no-cache clang lld musl-dev git
RUN cargo install cargo-chef
COPY src/ /app/src
COPY Cargo.toml /app
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin redis_bot

FROM rust:${RUST_VERSION}-alpine AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/redis_bot /bin/redis_bot
CMD ["/bin/redis_bot"]
