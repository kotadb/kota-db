---
tags:
- file
- kota-db
- ext_yml
---
version: '3.8'

services:
  # Main development environment
  kotadb-dev:
    build:
      context: .
      dockerfile: Dockerfile.dev
    container_name: kotadb-dev
    working_dir: /workspace
    volumes:
      # Mount source code for live editing
      - .:/workspace
      # Persist cargo registry and git for faster rebuilds
      - cargo-cache:/usr/local/cargo/registry
      - cargo-git:/usr/local/cargo/git
      # Persist build cache
      - target-cache:/workspace/target
      # Mount SSH keys for git operations
      - ~/.ssh:/home/dev/.ssh:ro
      - ~/.gitconfig:/home/dev/.gitconfig:ro
    ports:
      # HTTP REST API server port
      - "${KOTADB_HTTP_PORT:-8080}:${KOTADB_HTTP_PORT:-8080}"
      # Documentation server
      - "8000:8000"
      # Metrics/monitoring
      - "9090:9090"
    environment:
      - RUST_LOG=debug
      - CARGO_TARGET_DIR=/workspace/target
      - USER_ID=${USER_ID:-1000}
      - GROUP_ID=${GROUP_ID:-1000}
      # HTTP server configuration
      - KOTADB_HTTP_PORT=${KOTADB_HTTP_PORT:-8080}
      - KOTADB_HTTP_HOST=${KOTADB_HTTP_HOST:-0.0.0.0}
    stdin_open: true
    tty: true
    command: /bin/bash
    networks:
      - kotadb-dev

  # Documentation server for live preview
  docs-server:
    image: nginx:alpine
    container_name: kotadb-docs
    ports:
      - "8001:80"
    volumes:
      - ./target/doc:/usr/share/nginx/html:ro
      - ./docs:/usr/share/nginx/html/docs:ro
    depends_on:
      - kotadb-dev
    networks:
      - kotadb-dev

  # Redis for development caching/sessions (optional)
  redis-dev:
    image: redis:7-alpine
    container_name: kotadb-redis
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data
    command: redis-server --appendonly yes
    networks:
      - kotadb-dev

  # PostgreSQL for testing integrations (optional)
  postgres-dev:
    image: postgres:15-alpine
    container_name: kotadb-postgres
    environment:
      POSTGRES_DB: kotadb_test
      POSTGRES_USER: kotadb
      POSTGRES_PASSWORD: development
    ports:
      - "5432:5432"
    volumes:
      - postgres-data:/var/lib/postgresql/data
      - ./scripts/sql:/docker-entrypoint-initdb.d
    networks:
      - kotadb-dev

volumes:
  cargo-cache:
  cargo-git:
  target-cache:
  redis-data:
  postgres-data:

networks:
  kotadb-dev:
    driver: bridge
