# Build the frontends
FROM node:20-alpine AS frontend-builder

WORKDIR /app

COPY frontends/admin-frontend/package.json frontends/admin-frontend/package-lock.json ./frontends/admin-frontend/
RUN cd frontends/admin-frontend && npm install

COPY frontends/client-frontend/package.json frontends/client-frontend/package-lock.json ./frontends/client-frontend/
RUN cd frontends/client-frontend && npm install

COPY frontends/ ./frontends/

RUN cd frontends/admin-frontend && npm run build
RUN cd frontends/client-frontend && npm run build

# Build the backend, use cargo chef to cache parts of the build
FROM rust:1.87-slim AS planner
WORKDIR /usr/src/app
RUN cargo install cargo-chef
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM rust:1.87-slim AS rust-builder
WORKDIR /usr/src/app

# We may need these for the app
RUN apt-get update && apt-get install -y libpq-dev build-essential pkg-config && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-chef

COPY --from=planner /usr/src/app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .

# Import the frontend builds here, we need the templates there
COPY --from=frontend-builder /app/frontends/client-frontend/dist ./frontends/client-frontend/dist/
COPY --from=frontend-builder /app/frontends/admin-frontend/dist ./frontends/admin-frontend/dist/

RUN cargo build --release --bin schedule-voter-backend-axum

# Final minimal runtime
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y libpq5 ca-certificates && rm -rf /var/lib/apt/lists/*

RUN addgroup --system --gid 1001 appgroup && \
    adduser --system --uid 1001 --ingroup appgroup appuser

WORKDIR /usr/src/app

COPY --from=rust-builder --chown=root:root /usr/src/app/target/release/schedule-voter-backend-axum .

# Copy static files from the frontends
COPY --from=rust-builder --chown=root:root /usr/src/app/frontends/client-frontend/dist/static ./frontends/client-frontend/dist/static
COPY --from=rust-builder --chown=root:root /usr/src/app/frontends/admin-frontend/dist/static ./frontends/admin-frontend/dist/static
COPY --from=rust-builder --chown=root:root /usr/src/app/frontends/admin-frontend/dist/oidc-callback.html ./frontends/admin-frontend/dist/

USER appuser

ENV RUST_LOG=info
ENV HOST=0.0.0.0
ENV PORT=8080

EXPOSE 8080

CMD ["./schedule-voter-backend-axum"]