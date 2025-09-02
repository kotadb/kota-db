#!/bin/bash
# Deploy KotaDB SaaS API to Fly.io
# Usage: ./scripts/deploy-fly.sh [staging|production]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default to staging if no environment specified
ENVIRONMENT=${1:-staging}

echo -e "${GREEN}=== KotaDB SaaS API Deployment to Fly.io ===${NC}"
echo -e "Environment: ${YELLOW}$ENVIRONMENT${NC}"

# Check if flyctl is installed
if ! command -v flyctl &> /dev/null; then
    echo -e "${RED}Error: flyctl CLI is not installed${NC}"
    echo "Install it from: https://fly.io/docs/hands-on/install-flyctl/"
    exit 1
fi

# Check if authenticated
if ! flyctl auth whoami &> /dev/null; then
    echo -e "${RED}Error: Not authenticated with Fly.io${NC}"
    echo "Run: flyctl auth login"
    exit 1
fi

# Determine config file and app name based on environment
if [ "$ENVIRONMENT" = "production" ]; then
    CONFIG_FILE="fly.toml"
    APP_NAME="kotadb-api"
    DEPLOY_STRATEGY="rolling"
    HA_FLAG="--ha=true"
    echo -e "${YELLOW}⚠️  Production deployment - this will affect live users!${NC}"
    read -p "Are you sure you want to deploy to production? (yes/no): " CONFIRM
    if [ "$CONFIRM" != "yes" ]; then
        echo "Deployment cancelled"
        exit 0
    fi
elif [ "$ENVIRONMENT" = "staging" ]; then
    CONFIG_FILE="fly.staging.toml"
    APP_NAME="kotadb-api-staging"
    DEPLOY_STRATEGY="immediate"
    HA_FLAG="--ha=false"
else
    echo -e "${RED}Error: Invalid environment. Use 'staging' or 'production'${NC}"
    exit 1
fi

# Check if config file exists
if [ ! -f "$CONFIG_FILE" ]; then
    echo -e "${RED}Error: Config file $CONFIG_FILE not found${NC}"
    exit 1
fi

echo -e "${GREEN}Pre-deployment checks...${NC}"

# Run tests first
echo "Running tests..."
cargo test --features tree-sitter-parsing,git-integration --bin kotadb-api-server --quiet || {
    echo -e "${RED}Tests failed! Aborting deployment${NC}"
    exit 1
}

echo "Running clippy..."
cargo clippy --features tree-sitter-parsing,git-integration --bin kotadb-api-server -- -D warnings 2>/dev/null || {
    echo -e "${RED}Clippy check failed! Aborting deployment${NC}"
    exit 1
}

echo -e "${GREEN}✓ All checks passed${NC}"

# Check if app exists, create if it doesn't
if ! flyctl apps list | grep -q "$APP_NAME"; then
    echo -e "${YELLOW}App $APP_NAME doesn't exist. Creating...${NC}"
    flyctl apps create "$APP_NAME" --org personal || {
        echo -e "${RED}Failed to create app${NC}"
        exit 1
    }
    
    # Create volume for persistent data
    echo "Creating persistent volume..."
    if [ "$ENVIRONMENT" = "production" ]; then
        flyctl volumes create kotadb_data --size 10 --app "$APP_NAME" --region iad -y
    else
        flyctl volumes create kotadb_staging_data --size 5 --app "$APP_NAME" --region iad -y
    fi
    
    # Set initial secrets (you'll need to set these manually)
    echo -e "${YELLOW}Remember to set secrets:${NC}"
    echo "  flyctl secrets set DATABASE_URL='your-database-url' --app $APP_NAME"
    echo "  flyctl secrets set API_KEY='your-api-key' --app $APP_NAME"
fi

# Deploy
echo -e "${GREEN}Starting deployment...${NC}"
echo "Config: $CONFIG_FILE"
echo "Strategy: $DEPLOY_STRATEGY"

flyctl deploy \
    --config "$CONFIG_FILE" \
    --app "$APP_NAME" \
    $HA_FLAG \
    --strategy "$DEPLOY_STRATEGY" \
    --wait-timeout 600 || {
    echo -e "${RED}Deployment failed!${NC}"
    
    # Show recent logs for debugging
    echo -e "${YELLOW}Recent logs:${NC}"
    flyctl logs --app "$APP_NAME" -n 50
    
    exit 1
}

echo -e "${GREEN}✓ Deployment successful!${NC}"

# Verify deployment
echo "Verifying deployment..."
if [ "$ENVIRONMENT" = "production" ]; then
    URL="https://kotadb-api.fly.dev"
else
    URL="https://kotadb-api-staging.fly.dev"
fi

sleep 10  # Give the app time to start

if curl -f "$URL/health" &> /dev/null; then
    echo -e "${GREEN}✓ Health check passed${NC}"
    echo -e "App is running at: ${GREEN}$URL${NC}"
else
    echo -e "${YELLOW}⚠️  Health check failed or is still initializing${NC}"
    echo "Check logs with: flyctl logs --app $APP_NAME"
fi

# Show app status
echo -e "\n${GREEN}App Status:${NC}"
flyctl status --app "$APP_NAME"

echo -e "\n${GREEN}Deployment complete!${NC}"
echo "View logs: flyctl logs --app $APP_NAME"
echo "SSH into app: flyctl ssh console --app $APP_NAME"
echo "Open app: flyctl open --app $APP_NAME"