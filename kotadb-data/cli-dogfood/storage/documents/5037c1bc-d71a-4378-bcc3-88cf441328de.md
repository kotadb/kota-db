---
tags:
- file
- kota-db
- ext_yml
---
version: '3.8'

services:
  # KotaDB Server with sample data
  kotadb-server:
    image: ghcr.io/jayminwest/kota-db:latest
    container_name: kotadb-quickstart
    restart: unless-stopped
    ports:
      - "8080:8080"
    environment:
      - RUST_LOG=info
      - KOTADB_HTTP_HOST=0.0.0.0
      - KOTADB_HTTP_PORT=8080
    volumes:
      - ./quickstart-data:/data
      - ./quickstart/demo-data.sh:/docker-entrypoint-initdb.d/demo-data.sh
    command: ["kotadb", "serve", "--port", "8080"]
    healthcheck:
      test: ["CMD-SHELL", "curl -f http://localhost:8080/health || exit 1"]
      interval: 10s
      timeout: 5s
      retries: 5
      start_period: 30s
    networks:
      - kotadb

  # Python client demo
  python-demo:
    build:
      context: .
      dockerfile_inline: |
        FROM python:3.11-slim
        WORKDIR /app
        COPY quickstart/python-demo.py .
        RUN pip install kotadb-client requests
        CMD ["python", "python-demo.py"]
    container_name: kotadb-python-demo
    depends_on:
      kotadb-server:
        condition: service_healthy
    environment:
      - KOTADB_URL=http://kotadb-server:8080
    networks:
      - kotadb
    profiles:
      - demo

  # TypeScript client demo  
  typescript-demo:
    build:
      context: .
      dockerfile_inline: |
        FROM node:18-slim
        WORKDIR /app
        COPY quickstart/package.json quickstart/typescript-demo.ts ./
        RUN npm install
        RUN npm install -g ts-node
        CMD ["ts-node", "typescript-demo.ts"]
    container_name: kotadb-typescript-demo
    depends_on:
      kotadb-server:
        condition: service_healthy
    environment:
      - KOTADB_URL=http://kotadb-server:8080
    networks:
      - kotadb
    profiles:
      - demo

  # Web UI for interactive exploration (optional)
  web-ui:
    build:
      context: .
      dockerfile_inline: |
        FROM nginx:alpine
        COPY quickstart/web-ui/ /usr/share/nginx/html/
        COPY quickstart/nginx.conf /etc/nginx/conf.d/default.conf
    container_name: kotadb-web-ui
    ports:
      - "3000:80"
    depends_on:
      kotadb-server:
        condition: service_healthy
    networks:
      - kotadb
    profiles:
      - ui

volumes:
  quickstart-data:

networks:
  kotadb:
    driver: bridge