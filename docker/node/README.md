# LM Talk Node container

This directory builds the public `lm_node` HTTP service image.

Published image:

```text
ghcr.io/laomou/lm-talk-node
```

Release tags publish matching `linux/amd64` and `linux/arm64` images. Stable
tags also update `:latest`.

## Run a published image

Create a `node.json` from the repository template
`docs/examples/lm-node.config.example.json`, set a strong control token, and
start the node:

```bash
docker run --rm \
  --name lm-talk-node \
  -p 8787:8787 \
  -v "$PWD/node.json:/app/config.json:ro" \
  -v "$PWD/lm-node-data:/data" \
  ghcr.io/laomou/lm-talk-node:latest
```

Pin a release tag for a persistent deployment, for example
`ghcr.io/laomou/lm-talk-node:0.1.0`.

The service stores its state under `/data`. Keep that volume persistent and
protect the host disk at rest. Do not expose an unauthenticated control API;
configure `control_token_file` (or `control_token`) in `node.json`.

## Build locally

Run from the repository root:

```bash
docker build -f docker/node/Dockerfile -t lm-talk-node:local .
```

The default container command is:

```text
lm_node serve-control --config-file /app/config.json
```

