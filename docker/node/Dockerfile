# Package the Linux x86_64 binary built and uploaded by the release workflow.
FROM debian:bookworm-slim AS runtime
RUN useradd --system --create-home --uid 10001 lmnode \
  && apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY dist/lm_node-linux-x86_64 /usr/local/bin/lm_node
RUN chmod 0755 /usr/local/bin/lm_node \
  && mkdir -p /data /run/secrets \
  && chown -R lmnode:lmnode /data /run/secrets /app
USER lmnode
EXPOSE 8787
ENTRYPOINT ["/usr/local/bin/lm_node"]
CMD ["serve-control", "--config-file", "/app/config.json"]
