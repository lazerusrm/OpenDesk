FROM rust:1.88-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY migrations ./migrations
COPY src ./src
COPY templates ./templates
COPY static ./static
RUN cargo build --release --bins

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/opendesk /usr/local/bin/opendesk
COPY --from=builder /app/target/release/opendesk-migration-apply /usr/local/bin/opendesk-migration-apply
COPY --from=builder /app/target/release/opendesk-migration-credentials-attach /usr/local/bin/opendesk-migration-credentials-attach
COPY --from=builder /app/target/release/opendesk-user-password-reset /usr/local/bin/opendesk-user-password-reset
COPY --from=builder /app/target/release/opendesk-migration-dry-run /usr/local/bin/opendesk-migration-dry-run
COPY --from=builder /app/target/release/opendesk-migration-preflight /usr/local/bin/opendesk-migration-preflight
COPY migrations ./migrations
COPY templates ./templates
COPY static ./static
ENV OPENDESK_LISTEN_ADDR=0.0.0.0:8080
ENV OPENDESK_DATA_DIR=/data
EXPOSE 8080
CMD ["opendesk"]