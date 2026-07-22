# Build a public lm_node container for bootstrap/mailbox/DHT HTTP deployments.
FROM rust:1-bookworm AS builder
WORKDIR /src
COPY . .
RUN cargo build --locked --release -p lm_node

FROM debian:bookworm-slim AS runtime
RUN useradd --system --create-home --uid 10001 lmnode \
  && apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /src/target/release/lm_node /usr/local/bin/lm_node
RUN mkdir -p /data /run/secrets \
  && chown -R lmnode:lmnode /data /run/secrets /app
USER lmnode
EXPOSE 8787
ENTRYPOINT ["/usr/local/bin/lm_node"]
CMD ["serve-control", "--config-file", "/app/config.json"]
