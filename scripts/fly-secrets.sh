#!/bin/bash
# Manage Fly.io secrets for KotaDB SaaS API
# Usage: ./scripts/fly-secrets.sh [staging|production] [set|list|unset]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check arguments
if [ $# -lt 2 ]; then
    echo -e "${RED}Usage: $0 [staging|production] [set|list|unset]${NC}"
    echo "Examples:"
    echo "  $0 staging set         # Set secrets for staging"
    echo "  $0 production list      # List production secrets"
    echo "  $0 staging unset KEY    # Remove a secret from staging"
    exit 1
fi

ENVIRONMENT=$1
ACTION=$2

# Determine app name based on environment
if [ "$ENVIRONMENT" = "production" ]; then
    APP_NAME="kotadb-api"
    echo -e "${YELLOW}⚠️  Production environment - be careful!${NC}"
elif [ "$ENVIRONMENT" = "staging" ]; then
    APP_NAME="kotadb-api-staging"
else
    echo -e "${RED}Error: Invalid environment. Use 'staging' or 'production'${NC}"
    exit 1
fi

# Check if flyctl is installed
if ! command -v flyctl &> /dev/null; then
    echo -e "${RED}Error: flyctl CLI is not installed${NC}"
    exit 1
fi

echo -e "${GREEN}=== Fly.io Secrets Management ===${NC}"
echo "App: $APP_NAME"
echo "Action: $ACTION"
echo ""

case $ACTION in
    "set")
        echo -e "${GREEN}Setting secrets for $ENVIRONMENT...${NC}"
        echo -e "${YELLOW}Enter values for each secret (leave blank to skip):${NC}"
        
        # Database URL
        read -p "DATABASE_URL: " -s DATABASE_URL
        echo ""
        
        # API Keys
        read -p "API_KEY: " -s API_KEY
        echo ""
        
        # JWT Secret
        read -p "JWT_SECRET: " -s JWT_SECRET
        echo ""
        
        # Redis URL (optional)
        read -p "REDIS_URL (optional): " -s REDIS_URL
        echo ""
        
        # Sentry DSN (optional)
        read -p "SENTRY_DSN (optional): " -s SENTRY_DSN
        echo ""
        
        # Build the secrets command
        SECRETS_CMD="flyctl secrets set --app $APP_NAME"
        
        if [ ! -z "$DATABASE_URL" ]; then
            SECRETS_CMD="$SECRETS_CMD DATABASE_URL='$DATABASE_URL'"
        fi
        
        if [ ! -z "$API_KEY" ]; then
            SECRETS_CMD="$SECRETS_CMD API_KEY='$API_KEY'"
        fi
        
        if [ ! -z "$JWT_SECRET" ]; then
            SECRETS_CMD="$SECRETS_CMD JWT_SECRET='$JWT_SECRET'"
        fi
        
        if [ ! -z "$REDIS_URL" ]; then
            SECRETS_CMD="$SECRETS_CMD REDIS_URL='$REDIS_URL'"
        fi
        
        if [ ! -z "$SENTRY_DSN" ]; then
            SECRETS_CMD="$SECRETS_CMD SENTRY_DSN='$SENTRY_DSN'"
        fi
        
        # Execute the command
        if [ "$SECRETS_CMD" != "flyctl secrets set --app $APP_NAME" ]; then
            echo -e "${GREEN}Setting secrets...${NC}"
            eval $SECRETS_CMD
            echo -e "${GREEN}✓ Secrets updated${NC}"
            echo -e "${YELLOW}Note: This will trigger a new deployment${NC}"
        else
            echo -e "${YELLOW}No secrets to set${NC}"
        fi
        ;;
    
    "list")
        echo -e "${GREEN}Current secrets for $ENVIRONMENT:${NC}"
        flyctl secrets list --app "$APP_NAME"
        ;;
    
    "unset")
        if [ $# -lt 3 ]; then
            echo -e "${RED}Error: Specify the secret key to unset${NC}"
            echo "Usage: $0 $ENVIRONMENT unset SECRET_KEY"
            exit 1
        fi
        
        SECRET_KEY=$3
        echo -e "${YELLOW}Removing secret: $SECRET_KEY${NC}"
        
        read -p "Are you sure? (yes/no): " CONFIRM
        if [ "$CONFIRM" = "yes" ]; then
            flyctl secrets unset "$SECRET_KEY" --app "$APP_NAME"
            echo -e "${GREEN}✓ Secret removed${NC}"
            echo -e "${YELLOW}Note: This will trigger a new deployment${NC}"
        else
            echo "Cancelled"
        fi
        ;;
    
    *)
        echo -e "${RED}Error: Invalid action. Use 'set', 'list', or 'unset'${NC}"
        exit 1
        ;;
esac

echo -e "\n${GREEN}Done!${NC}"