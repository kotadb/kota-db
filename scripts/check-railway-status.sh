#!/bin/bash

# Check Railway deployment status
echo "Checking Railway deployment status..."

# Try to get the deployment ID
DEPLOYMENT_ID=$(railway status 2>/dev/null | grep -oE '[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}' | head -1)

if [ -z "$DEPLOYMENT_ID" ]; then
    echo "No active deployment found."
    echo "Trying to list recent deployments..."
    railway deployments --json 2>/dev/null | jq -r '.[] | "\(.id) - \(.status) - \(.createdAt)"' | head -5
else
    echo "Found deployment: $DEPLOYMENT_ID"
    railway logs --deployment "$DEPLOYMENT_ID" 2>/dev/null | tail -20
fi

echo ""
echo "Service status:"
railway status

echo ""
echo "Environment variables set:"
railway variables --json 2>/dev/null | jq -r 'keys[]' | sort