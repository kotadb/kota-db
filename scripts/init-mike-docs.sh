#!/bin/bash
# Initialize Mike versioned documentation for local testing

set -e

echo "🚀 Initializing Mike versioned documentation..."

# Check if we're in the project root
if [ ! -f "mkdocs.yml" ]; then
    echo "❌ Error: mkdocs.yml not found. Please run from project root."
    exit 1
fi

# Install dependencies if needed
echo "📦 Checking dependencies..."
pip install mkdocs-material mike mkdocs-minify-plugin mkdocs-git-revision-date-localized-plugin --quiet

# Configure git for Mike
echo "⚙️ Configuring Git for Mike..."
git config --global user.name "Local Developer"
git config --global user.email "developer@local"

# Fetch gh-pages branch if it exists
echo "🔄 Fetching gh-pages branch..."
git fetch origin gh-pages --depth=1 2>/dev/null || echo "No existing gh-pages branch found"

# Get current version
VERSION=$(cat VERSION)
echo "📌 Current version: $VERSION"

# Deploy current version as latest
echo "📚 Building and deploying documentation..."
mike deploy --update-aliases "$VERSION" latest

# Deploy development docs
echo "🔧 Deploying development documentation..."
mike deploy --update-aliases dev

# Set default version
echo "✨ Setting default version..."
mike set-default latest

# List all versions
echo ""
echo "✅ Documentation initialized successfully!"
echo ""
echo "Available versions:"
mike list

echo ""
echo "📖 To serve documentation locally, run:"
echo "   mike serve"
echo ""
echo "🌐 Documentation will be available at:"
echo "   http://localhost:8000"