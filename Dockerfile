# Multi-stage build for optimal size
FROM rust:1.70-alpine AS builder

# Install dependencies
RUN apk add --no-cache musl-dev openssl-dev

# Set up workspace
WORKDIR /usr/src/kotadb

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create dummy src and benches directories to build dependencies
RUN mkdir src benches && \
    echo "fn main() {}" > src/main.rs && \
    echo "" > src/lib.rs && \
    echo "fn main() {}" > benches/storage.rs && \
    echo "fn main() {}" > benches/indices.rs && \
    echo "fn main() {}" > benches/queries.rs
RUN cargo build --release && rm -rf src benches

# Copy actual source code
COPY src ./src
COPY tests ./tests
COPY docs ./docs
COPY examples ./examples
COPY benches ./benches

# Build the actual application
RUN touch src/main.rs src/lib.rs  # Force rebuild
RUN cargo build --release --bin kotadb

# Runtime stage
FROM alpine:3.18

# Install runtime dependencies
RUN apk add --no-cache ca-certificates libgcc

# Create kotadb user
RUN addgroup -g 1001 kotadb && \
    adduser -D -s /bin/sh -u 1001 -G kotadb kotadb

# Set up directories
RUN mkdir -p /data /config && \
    chown -R kotadb:kotadb /data /config

# Copy binary
COPY --from=builder /usr/src/kotadb/target/release/kotadb /usr/local/bin/kotadb

# Switch to non-root user
USER kotadb

# Expose ports
EXPOSE 8080 8081

# Set up volumes
VOLUME ["/data", "/config"]

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD kotadb --help > /dev/null || exit 1

# Default command
ENTRYPOINT ["kotadb"]
CMD ["--help"]
