FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app
RUN rustup toolchain install nightly-x86_64-unknown-linux-gnu

FROM chef AS planner

COPY . .
RUN cargo +nightly chef prepare --recipe-path recipe.json

FROM chef AS builder 

COPY --from=planner /app/recipe.json recipe.json
RUN cargo +nightly chef cook --release --recipe-path recipe.json

COPY . .
RUN cargo +nightly build --release

FROM debian:bookworm-slim AS runtime

WORKDIR /app
COPY --from=builder /app/target/release/url-shortener /app
RUN touch /app/db.sqlite

EXPOSE 8080

ENTRYPOINT ["/app/url-shortener"]
