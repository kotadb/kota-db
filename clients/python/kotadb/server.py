"""
KotaDB Server Management

This module provides functionality to download, install, and manage KotaDB server binaries.
It automatically downloads the appropriate binary for the current platform and provides
a simple interface to start and stop the server.
"""

import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import tarfile
import tempfile
import time
import zipfile
from pathlib import Path
from typing import Optional, Dict, Any, Tuple
from urllib.request import urlopen, urlretrieve
from urllib.error import URLError

# Version should match the KotaDB release version
KOTADB_VERSION = "0.5.0"

# Binary download configuration
BINARY_BASE_URL = "https://github.com/jayminwest/kota-db/releases/download"
BINARY_MANIFEST_URL = f"{BINARY_BASE_URL}/v{KOTADB_VERSION}/manifest.json"

# Local storage paths
KOTADB_HOME = Path.home() / ".kotadb"
BINARY_DIR = KOTADB_HOME / "bin"
CONFIG_DIR = KOTADB_HOME / "config"
DATA_DIR = KOTADB_HOME / "data"


def get_platform_info() -> Tuple[str, str]:
    """
    Detect the current platform and architecture.
    
    Returns:
        Tuple of (platform_name, architecture)
    """
    system = platform.system().lower()
    machine = platform.machine().lower()
    
    # Map platform names
    if system == "darwin":
        platform_name = "macos"
    elif system == "linux":
        # Check if we're on Alpine/musl
        try:
            with open("/etc/os-release", "r") as f:
                if "alpine" in f.read().lower():
                    platform_name = "linux-musl"
                else:
                    platform_name = "linux"
        except:
            platform_name = "linux"
    elif system == "windows":
        platform_name = "windows"
    else:
        raise RuntimeError(f"Unsupported platform: {system}")
    
    # Map architecture names
    if machine in ["x86_64", "amd64"]:
        arch = "x64"
    elif machine in ["aarch64", "arm64"]:
        arch = "arm64"
    else:
        raise RuntimeError(f"Unsupported architecture: {machine}")
    
    return platform_name, arch


def get_binary_info() -> Dict[str, str]:
    """
    Get the appropriate binary download information for the current platform.
    
    Returns:
        Dictionary with binary URL and expected SHA256 hash
    """
    platform_name, arch = get_platform_info()
    
    # Construct the binary identifier
    if platform_name == "linux-musl":
        binary_key = f"linux-musl-{arch}"
    else:
        binary_key = f"{platform_name}-{arch}"
    
    # For development, try to fetch from latest GitHub release
    # In production, these would be bundled or fetched from a CDN
    try:
        # Try to get manifest from GitHub release
        with urlopen(BINARY_MANIFEST_URL) as response:
            manifest = json.loads(response.read())
            
        if binary_key not in manifest.get("binaries", {}):
            raise RuntimeError(f"No binary available for platform: {binary_key}")
            
        binary_info = manifest["binaries"][binary_key]
        return {
            "url": f"{BINARY_BASE_URL}/v{KOTADB_VERSION}/{binary_info['url']}",
            "sha256": binary_info["sha256"],
            "extension": "zip" if platform_name == "windows" else "tar.gz"
        }
    except URLError:
        # Fallback to hardcoded URLs for common platforms
        fallback_binaries = {
            "linux-x64": {
                "url": f"{BINARY_BASE_URL}/v{KOTADB_VERSION}/kotadb-linux-x64.tar.gz",
                "sha256": None,  # Would be hardcoded in production
                "extension": "tar.gz"
            },
            "macos-x64": {
                "url": f"{BINARY_BASE_URL}/v{KOTADB_VERSION}/kotadb-macos-x64.tar.gz",
                "sha256": None,
                "extension": "tar.gz"
            },
            "macos-arm64": {
                "url": f"{BINARY_BASE_URL}/v{KOTADB_VERSION}/kotadb-macos-arm64.tar.gz",
                "sha256": None,
                "extension": "tar.gz"
            },
            "windows-x64": {
                "url": f"{BINARY_BASE_URL}/v{KOTADB_VERSION}/kotadb-windows-x64.zip",
                "sha256": None,
                "extension": "zip"
            }
        }
        
        if binary_key not in fallback_binaries:
            raise RuntimeError(f"No binary available for platform: {binary_key}")
            
        return fallback_binaries[binary_key]


def verify_checksum(file_path: Path, expected_sha256: Optional[str]) -> bool:
    """
    Verify the SHA256 checksum of a file.
    
    Args:
        file_path: Path to the file to verify
        expected_sha256: Expected SHA256 hash (hex string)
        
    Returns:
        True if checksum matches or no expected hash provided
    """
    if not expected_sha256:
        print("Warning: No checksum available for verification")
        return True
        
    sha256_hash = hashlib.sha256()
    with open(file_path, "rb") as f:
        for byte_block in iter(lambda: f.read(4096), b""):
            sha256_hash.update(byte_block)
            
    actual_sha256 = sha256_hash.hexdigest()
    if actual_sha256 != expected_sha256:
        print(f"Checksum mismatch! Expected: {expected_sha256}, Got: {actual_sha256}")
        return False
        
    return True


def download_binary(force: bool = False) -> Path:
    """
    Download and install the KotaDB binary for the current platform.
    
    Args:
        force: Force re-download even if binary exists
        
    Returns:
        Path to the installed binary
    """
    # Determine binary name
    binary_name = "kotadb.exe" if platform.system() == "Windows" else "kotadb"
    binary_path = BINARY_DIR / binary_name
    
    # Check if binary already exists
    if binary_path.exists() and not force:
        print(f"KotaDB binary already installed at {binary_path}")
        return binary_path
    
    # Create directories
    BINARY_DIR.mkdir(parents=True, exist_ok=True)
    
    # Get binary information
    print("Detecting platform...")
    platform_name, arch = get_platform_info()
    print(f"Platform: {platform_name}-{arch}")
    
    binary_info = get_binary_info()
    
    # Download binary
    print(f"Downloading KotaDB v{KOTADB_VERSION}...")
    with tempfile.TemporaryDirectory() as temp_dir:
        temp_path = Path(temp_dir)
        archive_name = f"kotadb.{binary_info['extension']}"
        archive_path = temp_path / archive_name
        
        try:
            urlretrieve(binary_info["url"], archive_path)
        except URLError as e:
            raise RuntimeError(f"Failed to download binary: {e}")
        
        # Verify checksum
        if not verify_checksum(archive_path, binary_info.get("sha256")):
            raise RuntimeError("Binary checksum verification failed")
        
        # Extract binary
        print("Extracting binary...")
        if binary_info["extension"] == "zip":
            with zipfile.ZipFile(archive_path, 'r') as zip_ref:
                zip_ref.extractall(temp_path)
        else:
            with tarfile.open(archive_path, 'r:gz') as tar_ref:
                tar_ref.extractall(temp_path)
        
        # Find and move the binary
        extracted_binary = temp_path / binary_name
        if not extracted_binary.exists():
            # Binary might be in a subdirectory
            for potential_binary in temp_path.rglob(binary_name):
                extracted_binary = potential_binary
                break
        
        if not extracted_binary.exists():
            raise RuntimeError(f"Binary {binary_name} not found in archive")
        
        # Move to final location
        shutil.move(str(extracted_binary), str(binary_path))
        
        # Make executable on Unix systems
        if platform.system() != "Windows":
            os.chmod(binary_path, 0o755)
        
        # Also extract MCP server if present
        mcp_name = "mcp_server.exe" if platform.system() == "Windows" else "mcp_server"
        mcp_source = temp_path / mcp_name
        if not mcp_source.exists():
            for potential_mcp in temp_path.rglob(mcp_name):
                mcp_source = potential_mcp
                break
        
        if mcp_source.exists():
            mcp_dest = BINARY_DIR / mcp_name
            shutil.move(str(mcp_source), str(mcp_dest))
            if platform.system() != "Windows":
                os.chmod(mcp_dest, 0o755)
            print(f"MCP server installed at {mcp_dest}")
    
    print(f"KotaDB binary installed at {binary_path}")
    return binary_path


class KotaDBServer:
    """
    Manages a KotaDB server instance.
    
    This class provides methods to start, stop, and check the status of a KotaDB server.
    It handles binary installation, configuration, and process management.
    """
    
    def __init__(
        self,
        data_dir: Optional[Path] = None,
        port: int = 8080,
        auto_install: bool = True
    ):
        """
        Initialize a KotaDB server manager.
        
        Args:
            data_dir: Directory for database files (default: ~/.kotadb/data)
            port: Port to run the server on
            auto_install: Automatically download binary if not present
        """
        self.data_dir = Path(data_dir) if data_dir else DATA_DIR
        self.port = port
        self.process: Optional[subprocess.Popen] = None
        self.binary_path: Optional[Path] = None
        
        # Ensure directories exist
        self.data_dir.mkdir(parents=True, exist_ok=True)
        CONFIG_DIR.mkdir(parents=True, exist_ok=True)
        
        # Install binary if needed
        if auto_install:
            try:
                self.binary_path = download_binary()
            except Exception as e:
                print(f"Warning: Could not download binary: {e}")
                print("Attempting to use system-installed kotadb...")
                # Try to find kotadb in PATH
                kotadb_path = shutil.which("kotadb")
                if kotadb_path:
                    self.binary_path = Path(kotadb_path)
                else:
                    raise RuntimeError(
                        "KotaDB binary not found. Please install manually or ensure internet connection."
                    )
    
    def create_config(self, config_path: Optional[Path] = None) -> Path:
        """
        Create a configuration file for the server.
        
        Args:
            config_path: Path for the config file (default: auto-generated)
            
        Returns:
            Path to the created configuration file
        """
        if not config_path:
            config_path = CONFIG_DIR / f"kotadb-{self.port}.toml"
        
        config_content = f"""
# KotaDB Server Configuration
# Auto-generated by kotadb Python package

[server]
host = "127.0.0.1"
port = {self.port}

[storage]
data_dir = "{self.data_dir}"
wal_enabled = true
cache_size = 1000

[logging]
level = "info"
format = "pretty"
"""
        
        with open(config_path, "w") as f:
            f.write(config_content)
        
        return config_path
    
    def start(self, config_path: Optional[Path] = None, timeout: int = 10) -> None:
        """
        Start the KotaDB server.
        
        Args:
            config_path: Path to configuration file (optional)
            timeout: Maximum time to wait for server startup (seconds)
        """
        if self.process and self.process.poll() is None:
            print("Server is already running")
            return
        
        if not self.binary_path or not self.binary_path.exists():
            raise RuntimeError("KotaDB binary not found. Run download_binary() first.")
        
        # Create config if not provided
        if not config_path:
            config_path = self.create_config()
        
        # Start the server process
        cmd = [str(self.binary_path), "--config", str(config_path)]
        print(f"Starting KotaDB server on port {self.port}...")
        
        self.process = subprocess.Popen(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )
        
        # Wait for server to be ready
        start_time = time.time()
        while time.time() - start_time < timeout:
            if self.is_running():
                print(f"KotaDB server started successfully on port {self.port}")
                return
            time.sleep(0.5)
        
        # If we get here, server failed to start
        self.stop()
        raise RuntimeError(f"Server failed to start within {timeout} seconds")
    
    def stop(self) -> None:
        """Stop the KotaDB server."""
        if not self.process:
            print("Server is not running")
            return
        
        print("Stopping KotaDB server...")
        self.process.terminate()
        
        # Wait for graceful shutdown
        try:
            self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            # Force kill if necessary
            self.process.kill()
            self.process.wait()
        
        self.process = None
        print("KotaDB server stopped")
    
    def is_running(self) -> bool:
        """
        Check if the server is running.
        
        Returns:
            True if the server is running and responsive
        """
        if not self.process or self.process.poll() is not None:
            return False
        
        # Try to connect to the server
        try:
            import socket
            sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            sock.settimeout(1)
            result = sock.connect_ex(('127.0.0.1', self.port))
            sock.close()
            return result == 0
        except:
            return False
    
    def __enter__(self):
        """Context manager entry."""
        self.start()
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit."""
        self.stop()


# Convenience functions
def start_server(port: int = 8080, data_dir: Optional[Path] = None) -> KotaDBServer:
    """
    Start a KotaDB server with default settings.
    
    Args:
        port: Port to run the server on
        data_dir: Directory for database files
        
    Returns:
        KotaDBServer instance
    """
    server = KotaDBServer(data_dir=data_dir, port=port)
    server.start()
    return server


def ensure_binary_installed() -> Path:
    """
    Ensure the KotaDB binary is installed.
    
    Returns:
        Path to the installed binary
    """
    return download_binary()


if __name__ == "__main__":
    # CLI interface for server management
    import argparse
    
    parser = argparse.ArgumentParser(description="KotaDB Server Management")
    subparsers = parser.add_subparsers(dest="command", help="Commands")
    
    # Install command
    install_parser = subparsers.add_parser("install", help="Download and install KotaDB binary")
    install_parser.add_argument("--force", action="store_true", help="Force re-download")
    
    # Start command
    start_parser = subparsers.add_parser("start", help="Start KotaDB server")
    start_parser.add_argument("--port", type=int, default=8080, help="Server port")
    start_parser.add_argument("--data-dir", type=str, help="Data directory")
    
    # Version command
    version_parser = subparsers.add_parser("version", help="Show version information")
    
    args = parser.parse_args()
    
    if args.command == "install":
        try:
            binary_path = download_binary(force=args.force)
            print(f"✓ KotaDB binary installed at {binary_path}")
        except Exception as e:
            print(f"✗ Installation failed: {e}")
            sys.exit(1)
    
    elif args.command == "start":
        server = KotaDBServer(
            port=args.port,
            data_dir=Path(args.data_dir) if args.data_dir else None
        )
        try:
            server.start()
            print("Press Ctrl+C to stop the server...")
            while True:
                time.sleep(1)
        except KeyboardInterrupt:
            server.stop()
    
    elif args.command == "version":
        print(f"KotaDB Python Client v{KOTADB_VERSION}")
        platform_name, arch = get_platform_info()
        print(f"Platform: {platform_name}-{arch}")
    
    else:
        parser.print_help()