---
tags:
- file
- kota-db
- ext_prod
---
# Production-ready multi-stage build for KotaDB
FROM rust:1.89-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev openssl-dev

# Set up workspace
WORKDIR /usr/src/kotadb

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create dummy src to build dependencies
RUN mkdir src benches && \
    echo "fn main() {}" > src/main.rs && \
    echo "" > src/lib.rs && \
    echo "fn main() {}" > benches/storage.rs && \
    echo "fn main() {}" > benches/indices.rs && \
    echo "fn main() {}" > benches/queries.rs

# Build dependencies only (cached layer)
RUN cargo build --release --bin kotadb && rm -rf src benches

# Copy actual source code
COPY src ./src
COPY tests ./tests
COPY docs ./docs
COPY examples ./examples
COPY benches ./benches

# Force rebuild of source code
RUN touch src/main.rs src/lib.rs

# Build the actual application with production optimizations
RUN cargo build --release --bin kotadb --locked

# Strip binary to reduce size
RUN strip target/release/kotadb

# ========================================
# Runtime stage - minimal Alpine image
# ========================================
FROM alpine:3.18

# Install runtime dependencies only
RUN apk add --no-cache ca-certificates libgcc curl

# Create non-root user for security
RUN addgroup -g 1001 kotadb && \
    adduser -D -s /bin/sh -u 1001 -G kotadb kotadb

# Set up data and config directories with proper permissions
RUN mkdir -p /data /config && \
    chown -R kotadb:kotadb /data /config && \
    chmod 755 /data /config

# Copy binary from builder stage
COPY --from=builder /usr/src/kotadb/target/release/kotadb /usr/local/bin/kotadb

# Ensure binary is executable
RUN chmod +x /usr/local/bin/kotadb

# Switch to non-root user
USER kotadb

# Set up environment variables for production
ENV KOTADB_PORT=8080
ENV KOTADB_DATA_DIR=/data
ENV KOTADB_LOG_LEVEL=info
ENV RUST_LOG=info
ENV RUST_BACKTRACE=0

# Expose the default port
EXPOSE 8080

# Set up volumes for data persistence
VOLUME ["/data"]

# Health check that tests the actual server endpoint
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:${KOTADB_PORT}/health || exit 1

# Production entrypoint and command
ENTRYPOINT ["kotadb"]
CMD ["--db-path", "/data", "serve", "--port", "8080"]
