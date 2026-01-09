# Stage 1: Build frontend
FROM node:20-alpine AS frontend-builder

WORKDIR /app/webapp

# Copy package files
COPY webapp/package*.json ./

# Install dependencies
RUN npm ci

# Copy source files
COPY webapp/ ./

# Build frontend
RUN npm run build

# Stage 2: Build backend
FROM rust:1.92-bookworm AS backend-builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Create dummy src to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies only
RUN cargo build --release && rm -rf src

# Copy actual source code
COPY src/ src/

# Touch main.rs to rebuild
RUN touch src/main.rs

# Build the application
RUN cargo build --release

# Stage 3: Runtime
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=backend-builder /app/target/release/datadisk /app/datadisk

# Copy frontend build
COPY --from=frontend-builder /app/webapp/dist /app/webapp/dist

# Copy configuration files
COPY etc/casbin_model.conf /app/etc/casbin_model.conf

# Create data directories
RUN mkdir -p /app/data/etc /app/etc

# Expose port
EXPOSE 8080

# Set environment variables
ENV RUST_LOG=info

# Run the application
CMD ["/app/datadisk", "-config", "/app/etc/datadisk.toml"]
