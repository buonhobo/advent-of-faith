FROM rust:slim AS builder
LABEL authors="buonhobo"

WORKDIR /build
COPY ../ .
ENV SQLX_OFFLINE=true
RUN cargo fetch
RUN cargo build --release

FROM debian:stable-slim
WORKDIR /app
COPY --from=builder /build/target/release/advent-of-faith /app/advent-of-faith
COPY --from=builder /build/static /app/static


CMD ["/app/advent-of-faith"]