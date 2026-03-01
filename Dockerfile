# Multi-stage build for OtamoryX
# Licensed under GPL-3.0 License - see LICENSE file for details
FROM node:24-bookworm-slim AS frontend-builder

# Install pnpm
RUN npm install -g pnpm@9

WORKDIR /app/frontend
COPY frontend/package*.json frontend/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile

COPY frontend/ ./
RUN pnpm run build:skip-typecheck

# Rust backend builder
FROM rust:slim-bookworm AS backend-builder

# Install system dependencies for compilation
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy dependency files first for better caching
COPY backend/Cargo.toml backend/Cargo.lock ./

# Set SQLx to offline mode for compilation without database
ENV SQLX_OFFLINE=true

# Create dummy main.rs and build dependencies only (for better layer caching)
RUN mkdir src && echo "fn main() {}" > src/main.rs && \
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse cargo build --release && \
    rm -rf src

# Copy source code and build the actual application
COPY backend .
RUN cargo build --release

# Final runtime image with Nginx
FROM nginx:stable-bookworm

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    openssl \
    ca-certificates \
    curl \
    supervisor \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy backend binary
COPY --from=backend-builder /app/target/release/otamoryx-server ./

# Copy frontend build to nginx public directory
COPY --from=frontend-builder /app/frontend/dist ./public

# Copy configuration files
COPY nginx.conf /etc/nginx/nginx.conf
COPY supervisord.conf /etc/supervisor/conf.d/supervisord.conf

# Create necessary directories and set permissions
RUN mkdir -p data data/comics data/cache && chmod +x otamoryx-server

EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

CMD ["/usr/bin/supervisord", "-c", "/etc/supervisor/conf.d/supervisord.conf"]