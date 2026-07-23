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
# The WASM package is built in the previous stage and copied above.
# Run Vite only; `npm run build` would invoke wasm-pack again, which is not installed here.
RUN cd apps/web && npm exec -- vite build

FROM node:22-bookworm AS node-admin-builder
WORKDIR /src

COPY apps/node-admin/package.json apps/node-admin/package-lock.json ./apps/node-admin/
RUN npm ci --prefix apps/node-admin

COPY apps/node-admin ./apps/node-admin

ARG BUILD_REF=container
ENV BUILD_REF=${BUILD_REF}
RUN cd apps/node-admin && NODE_ADMIN_BASE=/admin/ npm exec -- vite build

FROM caddy:2-alpine AS runtime
COPY docker/web/Caddyfile /etc/caddy/Caddyfile
COPY --from=web-builder /src/apps/web/dist /srv
COPY --from=node-admin-builder /src/apps/node-admin/dist /admin

EXPOSE 80
