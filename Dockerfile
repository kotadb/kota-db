# Multi-stage build for optimal size
FROM rust:1.89-alpine AS builder

# Install dependencies
RUN apk add --no-cache musl-dev openssl-dev pkgconfig openssl-libs-static

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
    echo "fn main() {}" > benches/queries.rs && \
    echo "fn main() {}" > benches/storage_stress.rs
# Build dependencies only
RUN cargo build --release --lib --no-default-features && rm -rf src benches

# Copy actual source code
COPY src ./src
COPY tests ./tests
COPY docs ./docs
COPY examples ./examples
COPY benches ./benches

# Build the actual application
RUN touch src/main.rs src/lib.rs  # Force rebuild
# Temporary: Build without ONNX to fix Alpine Linux compilation issues (Issue #168)
# TODO: Re-enable embeddings-onnx when ONNX Runtime 2.0 stabilizes on Alpine
# Build only main binary to avoid MCP server binary source file issues
RUN cargo build --release --bin kotadb --no-default-features

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
