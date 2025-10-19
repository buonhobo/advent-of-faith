FROM rust:slim AS builder
LABEL authors="buonhobo"

WORKDIR /build
COPY ../ .
ENV SQLX_OFFLINE=true
RUN cargo fetch
RUN cargo build --release

FROM debian:stable-slim
COPY --from=builder /build/target/release/advent-of-faith /app/

CMD ["/app/advent-of-faith"]