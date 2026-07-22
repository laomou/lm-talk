# Build the browser app and serve the generated static files with Caddy.
FROM rust:1-bookworm AS wasm-builder
WORKDIR /src

RUN rustup target add wasm32-unknown-unknown \
  && cargo install wasm-pack --locked

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY apps/web ./apps/web
RUN wasm-pack build crates/lm_wasm --target web --out-dir ../../apps/web/src/wasm --no-opt

FROM node:22-bookworm AS web-builder
WORKDIR /src

COPY apps/web/package.json apps/web/package-lock.json ./apps/web/
RUN npm ci --prefix apps/web

COPY apps/web ./apps/web
COPY --from=wasm-builder /src/apps/web/src/wasm ./apps/web/src/wasm

ARG BUILD_REF=container
ENV BUILD_REF=${BUILD_REF}
RUN npm run build --prefix apps/web

FROM caddy:2-alpine AS runtime
COPY docker/web/Caddyfile /etc/caddy/Caddyfile
COPY --from=web-builder /src/apps/web/dist /srv

EXPOSE 80
