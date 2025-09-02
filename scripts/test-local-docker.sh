#!/bin/bash

echo "🐳 Testing KotaDB Docker container locally with Railway environment..."

# Export Railway environment variables locally
echo "Fetching Railway environment variables..."
eval $(railway variables --json | jq -r 'to_entries | .[] | "export \(.key)=\"\(.value)\""')

# Override DATABASE_URL to use localhost if it's using internal Railway networking
if [[ "$DATABASE_URL" == *"railway.internal"* ]]; then
    echo "⚠️  DATABASE_URL uses Railway internal networking. You'll need to:"
    echo "   1. Use 'railway run' to test with Railway's network"
    echo "   2. Or set up a local PostgreSQL database"
    echo ""
    echo "For local testing, let's use a local PostgreSQL:"
    export DATABASE_URL="postgresql://postgres:password@localhost:5432/kotadb_test"
    echo "Using local DATABASE_URL: $DATABASE_URL"
fi

# Build the Docker image
echo ""
echo "Building Docker image..."
docker build -f Dockerfile.production -t kotadb-api-server:local . || {
    echo "❌ Docker build failed"
    exit 1
}

# Stop any existing container
docker stop kotadb-test 2>/dev/null
docker rm kotadb-test 2>/dev/null

# Run the container with Railway environment variables
echo ""
echo "Running container..."
docker run -d \
    --name kotadb-test \
    -p 8080:8080 \
    -e DATABASE_URL="$DATABASE_URL" \
    -e PORT="${PORT:-8080}" \
    -e KOTADB_DATA_DIR="${KOTADB_DATA_DIR:-/data}" \
    -e RUST_LOG="${RUST_LOG:-info,kotadb=debug}" \
    -e DEFAULT_RATE_LIMIT="${DEFAULT_RATE_LIMIT:-60}" \
    -e DEFAULT_MONTHLY_QUOTA="${DEFAULT_MONTHLY_QUOTA:-1000000}" \
    -e INTERNAL_API_KEY="${INTERNAL_API_KEY}" \
    kotadb-api-server:local

# Wait for startup
echo "Waiting for container to start..."
sleep 5

# Check container logs
echo ""
echo "📋 Container logs:"
docker logs kotadb-test

# Check if container is still running
if [ "$(docker ps -q -f name=kotadb-test)" ]; then
    echo ""
    echo "✅ Container is running!"
    echo ""
    echo "Testing health endpoint..."
    curl -f http://localhost:8080/health && echo "" || echo "❌ Health check failed"
    
    echo ""
    echo "📊 Container status:"
    docker ps -f name=kotadb-test
    
    echo ""
    echo "To view logs: docker logs -f kotadb-test"
    echo "To stop: docker stop kotadb-test && docker rm kotadb-test"
else
    echo ""
    echo "❌ Container stopped! Check logs above for errors."
    exit 1
fi