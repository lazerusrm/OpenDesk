FROM rust:1.85-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY src ./src
COPY templates ./templates
COPY static ./static
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/opendesk /usr/local/bin/opendesk
COPY migrations ./migrations
COPY templates ./templates
COPY static ./static
ENV OPENDESK_LISTEN_ADDR=0.0.0.0:8080
ENV OPENDESK_DATA_DIR=/data
EXPOSE 8080
CMD ["opendesk"]