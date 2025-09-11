---
tags:
- file
- kota-db
- ext_production
---
# Production Dockerfile for KotaDB SaaS API
# Single-stage build for guaranteed compatibility and simplified maintenance
# Optimized for reliability over build speed

# Build and runtime stage - using latest Rust with GLIBC compatibility
FROM rust:latest

# Install build dependencies including C++ compiler for native dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    git \
    build-essential \
    clang \
    lld \
    ca-certificates \
    curl \
    netcat-openbsd \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy all source files (single-stage eliminates caching complexity)
COPY . .

# Build the application with all features
RUN echo "Building kotadb-api-server..." && \
    cargo build --release --features tree-sitter-parsing,git-integration --bin kotadb-api-server && \
    echo "Binary size: $(ls -lh target/release/kotadb-api-server | awk '{print $5}')" && \
    test -f target/release/kotadb-api-server || (echo "ERROR: Binary not built!" && exit 1) && \
    echo "Binary dependencies:" && \
    ldd target/release/kotadb-api-server | head -10

# Create non-root user for security
RUN useradd -m -u 1001 -s /bin/bash kotadb

# Create data directory
RUN mkdir -p /data && chown kotadb:kotadb /data

# Copy binary to final location
RUN cp target/release/kotadb-api-server /usr/local/bin/kotadb-api-server && \
    chmod +x /usr/local/bin/kotadb-api-server

# Clean up build artifacts to reduce image size
RUN cargo clean && \
    rm -rf /usr/local/cargo/registry && \
    rm -rf target/release/build && \
    rm -rf target/release/deps && \
    apt-get autoremove -y build-essential pkg-config && \
    apt-get clean

# Switch to non-root user
USER kotadb

# Set working directory for runtime
WORKDIR /data

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:${PORT:-8080}/health || exit 1

# Default environment variables
ENV PORT=8080
ENV KOTADB_DATA_DIR=/data
ENV RUST_LOG=info

# Expose port
EXPOSE 8080

# Run the application with required parameters using exec form with explicit env expansion
ENTRYPOINT ["/bin/sh", "-c", "/usr/local/bin/kotadb-api-server --database-url \"${DATABASE_URL}\" --port \"${PORT}\" --data-dir \"${KOTADB_DATA_DIR}\""]