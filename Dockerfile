# Multi-stage build for OtamoryX
# Licensed under GPL-3.0 License - see LICENSE file for details
FROM node:22-slim AS frontend-builder

WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci

COPY frontend/ ./
RUN npm run build:skip-typecheck

# Rust backend builder
FROM rust:slim AS backend-builder

# Install system dependencies for compilation
RUN apk add --no-cache \
    musl-dev \
    pkgconfig \
    openssl-dev \
    build-base

WORKDIR /app
COPY backend/Cargo.toml ./
# Set SQLx to offline mode for compilation without database
ENV SQLX_OFFLINE=true
# Remove problematic lock file and let Cargo generate a new one with nightly features
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo +nightly build --release -Z unstable-options && rm -rf src

COPY backend/ ./
RUN cargo +nightly build --release -Z unstable-options

# Final runtime image with Nginx
FROM nginx:stable-bookworm

# Install runtime dependencies
RUN apk update && apk add --no-cache \
    openssl \
    ca-certificates \
    curl \
    supervisor

WORKDIR /app

# Copy backend binary
COPY --from=backend-builder /app/target/release/otamoryx-server ./

# Copy frontend build to nginx public directory
COPY --from=frontend-builder /app/frontend/dist ./public

# Copy configuration files
COPY nginx.conf /etc/nginx/nginx.conf
COPY supervisord.conf /etc/supervisor/conf.d/supervisord.conf

# Create necessary directories
RUN mkdir -p data data/comics data/cache

# Set permissions
RUN chmod +x otamoryx-server

EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

CMD ["/usr/bin/supervisord", "-c", "/etc/supervisor/conf.d/supervisord.conf"]