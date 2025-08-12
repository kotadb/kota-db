# Installation Guide

This guide covers all installation methods for KotaDB, from quick setup to production deployments.

## System Requirements

### Minimum Requirements
- **CPU**: 2 cores
- **RAM**: 512MB
- **Disk**: 100MB for binaries + data storage
- **OS**: Linux, macOS, or Windows

### Recommended Requirements
- **CPU**: 4+ cores
- **RAM**: 2GB+
- **Disk**: SSD with 10GB+ free space
- **OS**: Linux (Ubuntu 22.04+ or similar)

## Installation Methods

### 1. Build from Source

#### Prerequisites
- Rust 1.75.0+ ([Install Rust](https://rustup.rs/))
- Git
- C compiler (gcc/clang)

#### Steps

```bash
# Clone the repository
git clone https://github.com/jayminwest/kota-db.git
cd kota-db

# Build in release mode
cargo build --release

# The binary will be at ./target/release/kotadb
./target/release/kotadb --version
```

#### Development Build

For development with debug symbols and faster compilation:

```bash
cargo build
./target/debug/kotadb --version
```

### 2. Docker Installation

#### Using Docker Hub

```bash
# Pull the latest image
docker pull kotadb/kotadb:latest

# Run with default configuration
docker run -d \
  --name kotadb \
  -p 8080:8080 \
  -v $(pwd)/data:/data \
  kotadb/kotadb:latest
```

#### Building Docker Image Locally

```bash
# Build the image
docker build -t kotadb:local .

# Run the locally built image
docker run -d \
  --name kotadb \
  -p 8080:8080 \
  -v $(pwd)/data:/data \
  kotadb:local
```

### 3. Using Cargo Install

```bash
# Install directly from crates.io (when published)
cargo install kotadb

# Or install from GitHub
cargo install --git https://github.com/jayminwest/kota-db.git
```

### 4. Pre-built Binaries

Download pre-built binaries from the [GitHub Releases](https://github.com/jayminwest/kota-db/releases) page:

```bash
# Linux x86_64
wget https://github.com/jayminwest/kota-db/releases/latest/download/kotadb-linux-x86_64.tar.gz
tar -xzf kotadb-linux-x86_64.tar.gz
sudo mv kotadb /usr/local/bin/

# macOS
wget https://github.com/jayminwest/kota-db/releases/latest/download/kotadb-darwin-x86_64.tar.gz
tar -xzf kotadb-darwin-x86_64.tar.gz
sudo mv kotadb /usr/local/bin/

# Windows
# Download kotadb-windows-x86_64.zip from releases page
# Extract and add to PATH
```

## Platform-Specific Instructions

### Linux

#### Ubuntu/Debian

```bash
# Install build dependencies
sudo apt-get update
sudo apt-get install -y build-essential git curl

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Build KotaDB
git clone https://github.com/jayminwest/kota-db.git
cd kota-db
cargo build --release
```

#### Fedora/RHEL

```bash
# Install build dependencies
sudo dnf install -y gcc git curl

# Install Rust and build (same as Ubuntu)
```

#### Arch Linux

```bash
# Install from AUR (when available)
yay -S kotadb

# Or build manually
sudo pacman -S base-devel git rust
git clone https://github.com/jayminwest/kota-db.git
cd kota-db
cargo build --release
```

### macOS

```bash
# Install Xcode Command Line Tools
xcode-select --install

# Install Homebrew (if not installed)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install Rust
brew install rust

# Build KotaDB
git clone https://github.com/jayminwest/kota-db.git
cd kota-db
cargo build --release
```

### Windows

#### Using WSL2 (Recommended)

```powershell
# Install WSL2
wsl --install

# Inside WSL2, follow Linux instructions
```

#### Native Windows

```powershell
# Install Rust (download from https://rustup.rs)
# Install Git for Windows
# Install Visual Studio Build Tools

# Clone and build
git clone https://github.com/jayminwest/kota-db.git
cd kota-db
cargo build --release
```

## Client Libraries

### Python Client

```bash
# Install from PyPI
pip install kotadb-client

# Or install from source
git clone https://github.com/jayminwest/kota-db.git
cd kota-db/clients/python
pip install -e .
```

### TypeScript/JavaScript Client

```bash
# Install from npm
npm install kotadb-client

# Or using yarn
yarn add kotadb-client

# Or install from source
git clone https://github.com/jayminwest/kota-db.git
cd kota-db/clients/typescript
npm install
npm run build
```

## Verification

After installation, verify KotaDB is working:

```bash
# Check version
kotadb --version

# Run tests
cargo test --lib

# Start with default configuration
kotadb --config kotadb-dev.toml

# Check server health
curl http://localhost:8080/health
```

## Development Setup

For contributors and developers:

```bash
# Install development dependencies
cargo install just
cargo install cargo-watch
cargo install cargo-audit
cargo install cargo-tarpaulin

# Setup pre-commit hooks
just setup-dev

# Run development server with auto-reload
just dev

# Run all checks before committing
just check
```

## Client Libraries

### Python Client

```bash
pip install kotadb
```

### TypeScript/JavaScript Client

```bash
npm install @kotadb/client
# or
yarn add @kotadb/client
```

### Rust Client

Add to your `Cargo.toml`:

```toml
[dependencies]
kotadb-client = "0.1.0"
```

## Configuration

Create a configuration file `kotadb.toml`:

```toml
[storage]
path = "./data"
cache_size = 1000

[server]
host = "0.0.0.0"
port = 8080

[logging]
level = "info"
```

See [Configuration Guide](getting-started/configuration.md) for all options.

## Troubleshooting

### Common Issues

#### Port Already in Use

```bash
# Find process using port 8080
lsof -i :8080  # Linux/macOS
netstat -ano | findstr :8080  # Windows

# Use a different port
kotadb --port 8081
```

#### Permission Denied

```bash
# Fix permissions for data directory
chmod -R 755 ./data
chown -R $USER:$USER ./data
```

#### Build Failures

```bash
# Clean build cache
cargo clean

# Update Rust
rustup update

# Try building with verbose output
cargo build --release --verbose
```

## Next Steps

- [Configuration Guide](getting-started/configuration.md) - Customize your setup
- [First Database](getting-started/first-database.md) - Create your first database
- [Basic Operations](getting-started/basic-operations.md) - Learn CRUD operations
- [API Reference](api/index.md) - Explore the APIs