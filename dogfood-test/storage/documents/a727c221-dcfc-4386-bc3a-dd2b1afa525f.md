---
tags:
- file
- kota-db
- ext_sh
---
#!/bin/bash
# KotaDB Quick Start Installer
# Downloads and runs KotaDB in under 60 seconds

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
KOTADB_VERSION="latest"
KOTADB_PORT=${KOTADB_PORT:-8080}
INSTALL_DIR="$HOME/.kotadb"
DATA_DIR="$INSTALL_DIR/data"

echo -e "${BLUE}🚀 KotaDB Quick Start Installer${NC}"
echo -e "${BLUE}=================================${NC}"

# Check if Docker is available
if command -v docker >/dev/null 2>&1; then
    echo -e "${GREEN}✅ Docker found - using Docker method${NC}"
    USE_DOCKER=true
else
    echo -e "${YELLOW}⚠️  Docker not found - checking for pre-built binary${NC}"
    USE_DOCKER=false
fi

# Function to install via Docker
install_docker() {
    echo -e "${BLUE}📦 Starting KotaDB with Docker...${NC}"
    
    # Create data directory
    mkdir -p "$DATA_DIR"
    
    # Pull and run KotaDB
    docker pull ghcr.io/jayminwest/kota-db:$KOTADB_VERSION
    
    # Stop any existing instance
    docker stop kotadb-quickstart 2>/dev/null || true
    docker rm kotadb-quickstart 2>/dev/null || true
    
    # Run KotaDB
    docker run -d \
        --name kotadb-quickstart \
        -p $KOTADB_PORT:8080 \
        -v "$DATA_DIR:/data" \
        -e RUST_LOG=info \
        ghcr.io/jayminwest/kota-db:$KOTADB_VERSION \
        kotadb serve --port 8080
    
    echo -e "${GREEN}✅ KotaDB started in Docker container${NC}"
}

# Function to detect OS and architecture
detect_platform() {
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)
    
    case $OS in
        darwin)
            OS="macos"
            ;;
        linux)
            OS="linux"
            ;;
        msys*|mingw*|cygwin*)
            OS="windows"
            ;;
        *)
            echo -e "${RED}❌ Unsupported OS: $OS${NC}"
            exit 1
            ;;
    esac
    
    case $ARCH in
        x86_64|amd64)
            ARCH="x86_64"
            ;;
        arm64|aarch64)
            ARCH="aarch64"
            ;;
        *)
            echo -e "${RED}❌ Unsupported architecture: $ARCH${NC}"
            exit 1
            ;;
    esac
    
    PLATFORM="$OS-$ARCH"
}

# Function to install binary
install_binary() {
    echo -e "${BLUE}📦 Installing KotaDB binary for $PLATFORM...${NC}"
    
    # Detect platform
    detect_platform
    
    # Create directories
    mkdir -p "$INSTALL_DIR/bin"
    mkdir -p "$DATA_DIR"
    
    # Download URL (GitHub releases)
    BINARY_NAME="kotadb"
    if [ "$OS" = "windows" ]; then
        BINARY_NAME="kotadb.exe"
    fi
    
    DOWNLOAD_URL="https://github.com/jayminwest/kota-db/releases/latest/download/kotadb-$PLATFORM"
    
    # Download binary
    echo -e "${BLUE}⬇️  Downloading from $DOWNLOAD_URL${NC}"
    if command -v curl >/dev/null 2>&1; then
        curl -L -o "$INSTALL_DIR/bin/$BINARY_NAME" "$DOWNLOAD_URL"
    elif command -v wget >/dev/null 2>&1; then
        wget -O "$INSTALL_DIR/bin/$BINARY_NAME" "$DOWNLOAD_URL"
    else
        echo -e "${RED}❌ Neither curl nor wget found. Please install one of them.${NC}"
        exit 1
    fi
    
    # Make executable
    chmod +x "$INSTALL_DIR/bin/$BINARY_NAME"
    
    echo -e "${GREEN}✅ KotaDB binary installed to $INSTALL_DIR/bin/$BINARY_NAME${NC}"
    
    # Add to PATH for this session
    export PATH="$INSTALL_DIR/bin:$PATH"
    
    # Start KotaDB server in background
    echo -e "${BLUE}🚀 Starting KotaDB server...${NC}"
    cd "$DATA_DIR"
    nohup "$INSTALL_DIR/bin/$BINARY_NAME" serve --port $KOTADB_PORT > kotadb.log 2>&1 &
    KOTADB_PID=$!
    echo $KOTADB_PID > kotadb.pid
    
    echo -e "${GREEN}✅ KotaDB server started (PID: $KOTADB_PID)${NC}"
}

# Function to wait for server
wait_for_server() {
    echo -e "${BLUE}⏳ Waiting for KotaDB to be ready...${NC}"
    
    for i in {1..30}; do
        if curl -s "http://localhost:$KOTADB_PORT/health" >/dev/null 2>&1; then
            echo -e "${GREEN}✅ KotaDB is ready!${NC}"
            return 0
        fi
        sleep 1
        echo -n "."
    done
    
    echo -e "\n${RED}❌ KotaDB failed to start within 30 seconds${NC}"
    exit 1
}

# Function to create sample data
create_sample_data() {
    echo -e "${BLUE}📝 Creating sample documents...${NC}"
    
    # Sample document 1
    curl -s -X POST "http://localhost:$KOTADB_PORT/documents" \
        -H "Content-Type: application/json" \
        -d '{
            "path": "/quickstart/welcome.md",
            "title": "Welcome to KotaDB",
            "content": "# Welcome to KotaDB\n\nYou have successfully installed and started KotaDB!\n\n## What you can do:\n- Store and search documents\n- Use full-text search\n- Build applications with our client libraries\n\nEnjoy exploring!",
            "tags": ["welcome", "quickstart"]
        }' >/dev/null
    
    # Sample document 2
    curl -s -X POST "http://localhost:$KOTADB_PORT/documents" \
        -H "Content-Type: application/json" \
        -d '{
            "path": "/quickstart/features.md", 
            "title": "KotaDB Features",
            "content": "# KotaDB Features\n\n## Storage\n- Custom storage engine\n- ACID compliance\n- Zero external dependencies\n\n## Indexing\n- B+ tree for path lookups\n- Trigram for full-text search\n- Vector search for AI applications\n\n## Performance\n- Sub-10ms query latency\n- 3,600+ operations per second\n- Memory efficient\n\n## Safety\n- Type-safe client libraries\n- Runtime validation\n- Comprehensive testing",
            "tags": ["features", "overview"]
        }' >/dev/null
    
    echo -e "${GREEN}✅ Sample documents created${NC}"
}

# Function to test the installation
test_installation() {
    echo -e "${BLUE}🧪 Testing KotaDB installation...${NC}"
    
    # Test health endpoint
    HEALTH=$(curl -s "http://localhost:$KOTADB_PORT/health")
    if [ "$?" -eq 0 ]; then
        echo -e "${GREEN}✅ Health check passed${NC}"
    else
        echo -e "${RED}❌ Health check failed${NC}"
        exit 1
    fi
    
    # Test search
    SEARCH_RESULT=$(curl -s "http://localhost:$KOTADB_PORT/search?q=welcome")
    if echo "$SEARCH_RESULT" | grep -q "documents"; then
        echo -e "${GREEN}✅ Search functionality working${NC}"
    else
        echo -e "${RED}❌ Search test failed${NC}"
        exit 1
    fi
    
    # Test stats
    STATS=$(curl -s "http://localhost:$KOTADB_PORT/stats")
    if echo "$STATS" | grep -q "document_count"; then
        echo -e "${GREEN}✅ Statistics endpoint working${NC}"
    else
        echo -e "${RED}❌ Statistics test failed${NC}"
        exit 1
    fi
}

# Function to show next steps
show_next_steps() {
    echo -e "\n${GREEN}🎉 KotaDB Quick Start Complete!${NC}"
    echo -e "${GREEN}================================${NC}"
    echo -e "${BLUE}📍 Server running at: http://localhost:$KOTADB_PORT${NC}"
    echo -e "${BLUE}📂 Data directory: $DATA_DIR${NC}"
    
    echo -e "\n${YELLOW}🚀 Try these commands:${NC}"
    echo -e "   curl \"http://localhost:$KOTADB_PORT/health\""
    echo -e "   curl \"http://localhost:$KOTADB_PORT/search?q=welcome\""
    echo -e "   curl \"http://localhost:$KOTADB_PORT/stats\""
    
    echo -e "\n${YELLOW}📱 Install client libraries:${NC}"
    echo -e "   pip install kotadb-client      # Python"
    echo -e "   npm install kotadb-client      # JavaScript/TypeScript"
    echo -e "   cargo add kotadb               # Rust"
    
    echo -e "\n${YELLOW}📚 Next steps:${NC}"
    echo -e "   - Try the Python/TypeScript examples"
    echo -e "   - Read the documentation at github.com/jayminwest/kota-db"
    echo -e "   - Build your application!"
    
    if [ "$USE_DOCKER" = false ]; then
        echo -e "\n${YELLOW}🛑 To stop KotaDB:${NC}"
        echo -e "   kill \$(cat $DATA_DIR/kotadb.pid)"
        echo -e "   # Or: pkill kotadb"
    else
        echo -e "\n${YELLOW}🛑 To stop KotaDB:${NC}"
        echo -e "   docker stop kotadb-quickstart"
    fi
}

# Main installation flow
main() {
    echo -e "${BLUE}⚙️  Starting installation...${NC}"
    
    # Install KotaDB
    if [ "$USE_DOCKER" = true ]; then
        install_docker
    else
        install_binary
    fi
    
    # Wait for server to be ready
    wait_for_server
    
    # Create sample data
    create_sample_data
    
    # Test installation
    test_installation
    
    # Show next steps
    show_next_steps
    
    echo -e "\n${GREEN}Total time: ~60 seconds ⚡${NC}"
}

# Run main function
main