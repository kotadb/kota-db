---
tags:
- file
- kota-db
- ext_sh
---
#!/bin/bash
# One-liner KotaDB installer
# Usage: curl -sSL https://raw.githubusercontent.com/jayminwest/kota-db/main/quickstart/install.sh | bash

set -e

# Download and run the full installer
INSTALLER_URL="https://raw.githubusercontent.com/jayminwest/kota-db/main/quickstart/shell-installer.sh"

echo "🚀 Downloading KotaDB Quick Start installer..."

if command -v curl >/dev/null 2>&1; then
    curl -sSL "$INSTALLER_URL" | bash
elif command -v wget >/dev/null 2>&1; then
    wget -qO- "$INSTALLER_URL" | bash
else
    echo "❌ Neither curl nor wget found. Please install one of them."
    exit 1
fi