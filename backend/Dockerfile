FROM rust:1-bookworm AS builder
WORKDIR /app
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home appuser
WORKDIR /app
ENV APP_ENV=production HOST=0.0.0.0 PORT=8080
COPY --from=builder /app/target/release/newage-api /usr/local/bin/newage-api
USER 10001:10001
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/newage-api"]
