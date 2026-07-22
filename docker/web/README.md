# LM Talk Web container

This directory builds the static browser application image. The final image
uses Caddy to serve the compiled Web files on port `80`.

Published image:

```text
ghcr.io/laomou/lm-talk-web
```

Release tags publish matching `linux/amd64` and `linux/arm64` images. Stable
tags also update `:latest`.

## Run a published image

```bash
docker run --rm \
  --name lm-talk-web \
  -p 8080:80 \
  ghcr.io/laomou/lm-talk-web:latest
```

Open `http://127.0.0.1:8080`. For a public deployment, put this container
behind an HTTPS reverse proxy. The browser app needs a secure context for
WebCrypto, so production deployments should use HTTPS.

The sync service URL and token are configured in the Web application; the Web
image does not bundle node credentials.

## Build locally

Run from the repository root:

```bash
docker build \
  -f docker/web/Dockerfile \
  --build-arg BUILD_REF=local \
  -t lm-talk-web:local \
  .
```

`BUILD_REF` is shown by the application as its build identifier. The release
workflow sets it to the release tag.

