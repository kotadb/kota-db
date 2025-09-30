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
    DB_SECRET_KEY="SUPABASE_DB_URL_PRODUCTION"
elif [ "$ENVIRONMENT" = "staging" ]; then
    APP_NAME="kotadb-api-staging"
    DB_SECRET_KEY="SUPABASE_DB_URL_STAGING"
else
    echo -e "${RED}Error: Invalid environment. Use 'staging' or 'production'${NC}"
    exit 1
fi

# Pick matching env file for the environment and load values into an
# associative array when we need to auto-populate secrets.
if [ "$ENVIRONMENT" = "production" ]; then
    ENV_FILE=".env.production"
else
    ENV_FILE=".env.develop"
fi

load_env_file() {
    local file="$1"

    if [ ! -f "$file" ]; then
        echo -e "${RED}Error: expected env file '$file' not found${NC}"
        exit 1
    fi

    while IFS= read -r line || [ -n "$line" ]; do
        line="${line%%#*}"
        line="${line%"${line##*[![:space:]]}"}"
        # shellcheck disable=SC2001
        line=$(echo "$line" | sed 's/^[[:space:]]*//')

        if [ -z "$line" ]; then
            continue
        fi

        IFS='=' read -r key value <<< "$line"
        if [ -z "$key" ]; then
            continue
        fi

        # Trim whitespace around key/value
        # shellcheck disable=SC2001
        key=$(echo "$key" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
        # shellcheck disable=SC2001
        value=$(echo "$value" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')

        if [ -n "$value" ]; then
            len=${#value}
            first_char=${value:0:1}
            last_char="${value:((len-1)):1}"
            if [ "$len" -ge 2 ] && [ "$first_char" = '"' ] && [ "$last_char" = '"' ]; then
                value=${value:1:len-2}
            fi
        fi

        printf -v "$key" '%s' "$value"
    done < "$file"
}

get_env_value() {
    local key="$1"
    # shellcheck disable=SC1083
    eval "printf '%s' \"\${$key-}\""
}

# Check if flyctl is installed
if ! command -v flyctl &> /dev/null; then
    echo -e "${RED}Error: flyctl CLI is not installed${NC}"
    exit 1
fi

echo -e "${GREEN}=== Fly.io Secrets Management ===${NC}"
echo "App: $APP_NAME"
echo "Action: $ACTION"
echo "Env file: $ENV_FILE"
echo ""

case $ACTION in
    "set")
        echo -e "${GREEN}Setting secrets for $ENVIRONMENT using $ENV_FILE...${NC}"
        load_env_file "$ENV_FILE"

        if [ "$ENVIRONMENT" = "production" ]; then
            SECRET_KEYS=(
                DATABASE_URL
                SUPABASE_PROJECT_REF
                SUPABASE_URL
                SUPABASE_ANON_KEY
                SUPABASE_SERVICE_KEY
                SUPABASE_DB_URL
                SUPABASE_DB_URL_PRODUCTION
                SQLX_DISABLE_STATEMENT_CACHE
                DEFAULT_RATE_LIMIT
                DEFAULT_MONTHLY_QUOTA
                INTERNAL_API_KEY
                SAAS_PRODUCTION_API_KEY
                KOTADB_WEBHOOK_BASE_URL
                GITHUB_CLIENT_ID
                GITHUB_CLIENT_SECRET
                GITHUB_WEBHOOK_TOKEN
                GITHUB_APP_PRIVATE_KEY
                REDIS_URL
                SENTRY_DSN
                JWT_SECRET
            )
        else
            SECRET_KEYS=(
                DATABASE_URL
                SUPABASE_PROJECT_REF
                SUPABASE_URL
                SUPABASE_ANON_KEY
                SUPABASE_SERVICE_KEY
                SUPABASE_DB_URL
                SUPABASE_DB_URL_STAGING
                SQLX_DISABLE_STATEMENT_CACHE
                DEFAULT_RATE_LIMIT
                DEFAULT_MONTHLY_QUOTA
                INTERNAL_API_KEY
                SAAS_STAGING_API_KEY
                KOTADB_WEBHOOK_BASE_URL
                GITHUB_CLIENT_ID
                GITHUB_CLIENT_SECRET
                GITHUB_WEBHOOK_TOKEN
                GITHUB_APP_PRIVATE_KEY
                REDIS_URL
                SENTRY_DSN
                JWT_SECRET
            )
        fi

        SECRET_ARGS=(flyctl secrets set --app "$APP_NAME")
        missing=()

        for key in "${SECRET_KEYS[@]}"; do
            value="$(get_env_value "$key")"
            if [ -n "$value" ] && [[ $value != \<* ]]; then
                SECRET_ARGS+=("$key=$value")
            else
                missing+=("$key")
            fi
        done

        if [ ${#SECRET_ARGS[@]} -gt 5 ]; then
            echo -e "${GREEN}Applying secrets via flyctl...${NC}"
            "${SECRET_ARGS[@]}"
            echo -e "${GREEN}✓ Secrets updated${NC}"
            echo -e "${YELLOW}Note: Updating secrets triggers a new deployment${NC}"
        else
            echo -e "${YELLOW}No secrets were set (env file missing required keys)${NC}"
        fi

        if [ ${#missing[@]} -gt 0 ]; then
            echo -e "${YELLOW}Skipped unset keys:${NC} ${missing[*]}"
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
