#!/usr/bin/env bash

# KotaDB Version Bump Script
# Usage: ./scripts/version-bump.sh [major|minor|patch|prerelease] [--preview]

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
PREVIEW=false

# Parse arguments
BUMP_TYPE="${1:-patch}"
if [ "${2:-}" = "--preview" ]; then
    PREVIEW=true
fi

cd "$PROJECT_ROOT"

# Get current version from Cargo.toml
CURRENT_VERSION=$(grep '^version = ' Cargo.toml | head -1 | cut -d'"' -f2)
echo -e "${BLUE}Current version: $CURRENT_VERSION${NC}"

# Parse version components
if [[ $CURRENT_VERSION =~ ^([0-9]+)\.([0-9]+)\.([0-9]+)(-([a-zA-Z0-9.]+))?$ ]]; then
    MAJOR="${BASH_REMATCH[1]}"
    MINOR="${BASH_REMATCH[2]}"
    PATCH="${BASH_REMATCH[3]}"
    PRERELEASE="${BASH_REMATCH[5]}"
else
    echo -e "${RED}Error: Unable to parse version${NC}"
    exit 1
fi

# Calculate new version
case "$BUMP_TYPE" in
    major)
        NEW_MAJOR=$((MAJOR + 1))
        NEW_MINOR=0
        NEW_PATCH=0
        NEW_PRERELEASE=""
        ;;
    minor)
        NEW_MAJOR=$MAJOR
        NEW_MINOR=$((MINOR + 1))
        NEW_PATCH=0
        NEW_PRERELEASE=""
        ;;
    patch)
        NEW_MAJOR=$MAJOR
        NEW_MINOR=$MINOR
        NEW_PATCH=$((PATCH + 1))
        NEW_PRERELEASE=""
        ;;
    prerelease)
        NEW_MAJOR=$MAJOR
        NEW_MINOR=$MINOR
        NEW_PATCH=$PATCH
        
        # Handle prerelease versioning
        if [ -z "$PRERELEASE" ]; then
            # No existing prerelease, start with beta.1
            NEW_PRERELEASE="beta.1"
        elif [[ $PRERELEASE =~ ^([a-zA-Z]+)\.([0-9]+)$ ]]; then
            # Increment existing prerelease number
            PRERELEASE_TYPE="${BASH_REMATCH[1]}"
            PRERELEASE_NUM="${BASH_REMATCH[2]}"
            NEW_PRERELEASE="$PRERELEASE_TYPE.$((PRERELEASE_NUM + 1))"
        else
            echo -e "${RED}Error: Unable to parse prerelease version${NC}"
            exit 1
        fi
        ;;
    *)
        echo -e "${RED}Error: Invalid bump type${NC}"
        echo "Usage: $0 [major|minor|patch|prerelease] [--preview]"
        exit 1
        ;;
esac

# Construct new version string
if [ -n "$NEW_PRERELEASE" ]; then
    NEW_VERSION="$NEW_MAJOR.$NEW_MINOR.$NEW_PATCH-$NEW_PRERELEASE"
else
    NEW_VERSION="$NEW_MAJOR.$NEW_MINOR.$NEW_PATCH"
fi

echo -e "${GREEN}New version: $NEW_VERSION${NC}"

if [ "$PREVIEW" = true ]; then
    echo -e "${YELLOW}Preview mode - no changes made${NC}"
    exit 0
fi

# Update version using the release script
echo -e "\n${BLUE}Running release script...${NC}"
"$SCRIPT_DIR/release.sh" "$NEW_VERSION"