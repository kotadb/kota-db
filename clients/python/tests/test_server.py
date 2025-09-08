"""
Test suite for KotaDB server management functionality.

Tests platform detection, binary downloading, server lifecycle management,
and error handling.
"""

import json
import os
import platform
import tempfile
import unittest
from pathlib import Path
from unittest.mock import MagicMock, Mock, patch, mock_open
import sys
import subprocess
import time

# Import the server module
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from kotadb.server import (
    KotaDBServer,
    get_platform_info,
    get_binary_info,
    verify_checksum,
    download_binary,
    start_server,
    ensure_binary_installed,
)


class TestPlatformDetection(unittest.TestCase):
    """Test platform and architecture detection."""
    
    def test_platform_detection_macos_arm64(self):
        """Test macOS ARM64 detection."""
        with patch('platform.system', return_value='Darwin'):
            with patch('platform.machine', return_value='arm64'):
                platform_name, arch = get_platform_info()
                self.assertEqual(platform_name, 'macos')
                self.assertEqual(arch, 'arm64')
    
    def test_platform_detection_macos_x64(self):
        """Test macOS x64 detection."""
        with patch('platform.system', return_value='Darwin'):
            with patch('platform.machine', return_value='x86_64'):
                platform_name, arch = get_platform_info()
                self.assertEqual(platform_name, 'macos')
                self.assertEqual(arch, 'x64')
    
    def test_platform_detection_linux(self):
        """Test Linux detection."""
        with patch('platform.system', return_value='Linux'):
            with patch('platform.machine', return_value='x86_64'):
                # Mock /etc/os-release to simulate non-Alpine Linux
                mock_file = mock_open(read_data='ID=ubuntu\nNAME="Ubuntu"')
                with patch('builtins.open', mock_file):
                    platform_name, arch = get_platform_info()
                    self.assertEqual(platform_name, 'linux')
                    self.assertEqual(arch, 'x64')
    
    def test_platform_detection_alpine(self):
        """Test Alpine Linux (musl) detection."""
        with patch('platform.system', return_value='Linux'):
            with patch('platform.machine', return_value='x86_64'):
                # Mock /etc/os-release to simulate Alpine Linux
                mock_file = mock_open(read_data='ID=alpine\nNAME="Alpine Linux"')
                with patch('builtins.open', mock_file):
                    platform_name, arch = get_platform_info()
                    self.assertEqual(platform_name, 'linux-musl')
                    self.assertEqual(arch, 'x64')
    
    def test_platform_detection_windows(self):
        """Test Windows detection."""
        with patch('platform.system', return_value='Windows'):
            with patch('platform.machine', return_value='AMD64'):
                platform_name, arch = get_platform_info()
                self.assertEqual(platform_name, 'windows')
                self.assertEqual(arch, 'x64')
    
    def test_unsupported_platform(self):
        """Test unsupported platform raises error."""
        with patch('platform.system', return_value='FreeBSD'):
            with self.assertRaises(RuntimeError) as context:
                get_platform_info()
            self.assertIn('Unsupported platform', str(context.exception))
    
    def test_unsupported_architecture(self):
        """Test unsupported architecture raises error."""
        with patch('platform.system', return_value='Linux'):
            with patch('platform.machine', return_value='armv7l'):
                with self.assertRaises(RuntimeError) as context:
                    get_platform_info()
                self.assertIn('Unsupported architecture', str(context.exception))


class TestBinaryInfo(unittest.TestCase):
    """Test binary download information retrieval."""
    
    @patch('kotadb.server.urlopen')
    def test_get_binary_info_from_manifest(self, mock_urlopen):
        """Test getting binary info from GitHub manifest."""
        # Mock manifest response
        manifest = {
            "version": "0.1.12",
            "binaries": {
                "linux-x64": {
                    "url": "kotadb-linux-x64.tar.gz",
                    "sha256": "abc123",
                    "platform": "linux",
                    "arch": "x64"
                }
            }
        }
        
        mock_response = MagicMock()
        mock_response.read.return_value = json.dumps(manifest).encode()
        mock_urlopen.return_value.__enter__.return_value = mock_response
        
        with patch('kotadb.server.get_platform_info', return_value=('linux', 'x64')):
            binary_info = get_binary_info()
            self.assertIn('kotadb-linux-x64.tar.gz', binary_info['url'])
            self.assertEqual(binary_info['sha256'], 'abc123')
            self.assertEqual(binary_info['extension'], 'tar.gz')
    
    @patch('kotadb.server.urlopen')
    def test_get_binary_info_fallback(self, mock_urlopen):
        """Test fallback when manifest is unavailable."""
        # Mock URLError for manifest fetch
        from urllib.error import URLError
        mock_urlopen.side_effect = URLError("Connection failed")
        
        with patch('kotadb.server.get_platform_info', return_value=('macos', 'arm64')):
            binary_info = get_binary_info()
            self.assertIn('kotadb-macos-arm64.tar.gz', binary_info['url'])
            self.assertEqual(binary_info['extension'], 'tar.gz')
            self.assertIsNone(binary_info['sha256'])
    
    @patch('kotadb.server.urlopen')
    def test_get_binary_info_unsupported_platform(self, mock_urlopen):
        """Test error for unsupported platform in manifest."""
        manifest = {
            "version": "0.1.12",
            "binaries": {
                "linux-x64": {
                    "url": "kotadb-linux-x64.tar.gz",
                    "sha256": "abc123"
                }
            }
        }
        
        mock_response = MagicMock()
        mock_response.read.return_value = json.dumps(manifest).encode()
        mock_urlopen.return_value.__enter__.return_value = mock_response
        
        with patch('kotadb.server.get_platform_info', return_value=('freebsd', 'x64')):
            with self.assertRaises(RuntimeError) as context:
                get_binary_info()
            self.assertIn('No binary available', str(context.exception))


class TestChecksumVerification(unittest.TestCase):
    """Test SHA256 checksum verification."""
    
    def test_verify_checksum_success(self):
        """Test successful checksum verification."""
        with tempfile.NamedTemporaryFile(delete=False) as tmp:
            tmp.write(b"test content")
            tmp_path = Path(tmp.name)
        
        try:
            # Calculate actual SHA256
            import hashlib
            expected = hashlib.sha256(b"test content").hexdigest()
            
            result = verify_checksum(tmp_path, expected)
            self.assertTrue(result)
        finally:
            tmp_path.unlink()
    
    def test_verify_checksum_mismatch(self):
        """Test checksum mismatch detection."""
        with tempfile.NamedTemporaryFile(delete=False) as tmp:
            tmp.write(b"test content")
            tmp_path = Path(tmp.name)
        
        try:
            wrong_sha256 = "0" * 64  # Invalid SHA256
            result = verify_checksum(tmp_path, wrong_sha256)
            self.assertFalse(result)
        finally:
            tmp_path.unlink()
    
    def test_verify_checksum_no_expected(self):
        """Test when no expected checksum is provided."""
        with tempfile.NamedTemporaryFile(delete=False) as tmp:
            tmp.write(b"test content")
            tmp_path = Path(tmp.name)
        
        try:
            result = verify_checksum(tmp_path, None)
            self.assertTrue(result)  # Should pass with warning
        finally:
            tmp_path.unlink()


class TestKotaDBServer(unittest.TestCase):
    """Test KotaDB server lifecycle management."""
    
    def setUp(self):
        """Set up test fixtures."""
        self.temp_dir = tempfile.mkdtemp()
        self.data_dir = Path(self.temp_dir) / "data"
        self.port = 18080  # Use non-standard port for testing
    
    def tearDown(self):
        """Clean up test fixtures."""
        import shutil
        shutil.rmtree(self.temp_dir, ignore_errors=True)
    
    @patch('kotadb.server.download_binary')
    def test_server_initialization(self, mock_download):
        """Test server initialization."""
        mock_download.return_value = Path("/fake/binary/path")
        
        server = KotaDBServer(
            data_dir=self.data_dir,
            port=self.port,
            auto_install=True
        )
        
        self.assertEqual(server.port, self.port)
        self.assertEqual(str(server.data_dir), str(self.data_dir))
        self.assertTrue(self.data_dir.exists())
        mock_download.assert_called_once()
    
    @patch('kotadb.server.download_binary')
    def test_server_no_auto_install(self, mock_download):
        """Test server without auto-installation."""
        server = KotaDBServer(
            data_dir=self.data_dir,
            port=self.port,
            auto_install=False
        )
        
        mock_download.assert_not_called()
        self.assertIsNone(server.binary_path)
    
    @patch('subprocess.Popen')
    @patch('kotadb.server.download_binary')
    def test_server_start_stop(self, mock_download, mock_popen):
        """Test server start and stop."""
        mock_download.return_value = Path("/fake/binary/kotadb")
        mock_process = MagicMock()
        mock_process.poll.return_value = None  # Process is running
        mock_popen.return_value = mock_process
        
        server = KotaDBServer(port=self.port)
        
        # Mock is_running to return True after start
        with patch.object(server, 'is_running', return_value=True):
            server.start(timeout=1)
        
        self.assertIsNotNone(server.process)
        mock_popen.assert_called_once()
        
        # Test stop
        server.stop()
        mock_process.terminate.assert_called_once()
    
    @patch('subprocess.Popen')
    @patch('kotadb.server.download_binary')
    def test_server_context_manager(self, mock_download, mock_popen):
        """Test server as context manager."""
        mock_download.return_value = Path("/fake/binary/kotadb")
        mock_process = MagicMock()
        mock_process.poll.return_value = None
        mock_popen.return_value = mock_process
        
        with patch('kotadb.server.KotaDBServer.is_running', return_value=True):
            with KotaDBServer(port=self.port) as server:
                self.assertIsNotNone(server.process)
                mock_popen.assert_called_once()
            
            # Should be stopped after context exit
            mock_process.terminate.assert_called_once()
    
    @patch('kotadb.server.download_binary')
    def test_server_is_running(self, mock_download):
        """Test server running status check."""
        mock_download.return_value = Path("/fake/binary/kotadb")
        
        server = KotaDBServer(port=self.port)
        
        # No process - should return False
        self.assertFalse(server.is_running())
        
        # Mock process that has exited
        server.process = MagicMock()
        server.process.poll.return_value = 0  # Process exited
        self.assertFalse(server.is_running())
        
        # Mock running process but socket not responding
        server.process.poll.return_value = None  # Process running
        with patch('socket.socket') as mock_socket:
            mock_sock_instance = MagicMock()
            mock_sock_instance.connect_ex.return_value = 1  # Connection failed
            mock_socket.return_value = mock_sock_instance
            self.assertFalse(server.is_running())
        
        # Mock running process with responding socket
        with patch('socket.socket') as mock_socket:
            mock_sock_instance = MagicMock()
            mock_sock_instance.connect_ex.return_value = 0  # Connection success
            mock_socket.return_value = mock_sock_instance
            self.assertTrue(server.is_running())
    
    @patch('kotadb.server.download_binary')
    def test_create_config(self, mock_download):
        """Test configuration file creation."""
        mock_download.return_value = Path("/fake/binary/kotadb")
        
        server = KotaDBServer(
            data_dir=self.data_dir,
            port=self.port
        )
        
        config_path = server.create_config()
        self.assertTrue(config_path.exists())
        
        # Verify config content
        with open(config_path, 'r') as f:
            config = f.read()
            self.assertIn(f'port = {self.port}', config)
            self.assertIn(str(self.data_dir), config)
            self.assertIn('wal_enabled = true', config)


class TestConvenienceFunctions(unittest.TestCase):
    """Test convenience functions."""
    
    @patch('kotadb.server.KotaDBServer')
    def test_start_server_function(self, mock_server_class):
        """Test start_server convenience function."""
        mock_server = MagicMock()
        mock_server_class.return_value = mock_server
        
        result = start_server(port=9090, data_dir=Path("/custom/dir"))
        
        mock_server_class.assert_called_once_with(
            data_dir=Path("/custom/dir"),
            port=9090
        )
        mock_server.start.assert_called_once()
        self.assertEqual(result, mock_server)
    
    @patch('kotadb.server.download_binary')
    def test_ensure_binary_installed(self, mock_download):
        """Test ensure_binary_installed function."""
        expected_path = Path("/fake/binary/kotadb")
        mock_download.return_value = expected_path
        
        result = ensure_binary_installed()
        
        mock_download.assert_called_once_with()
        self.assertEqual(result, expected_path)


class TestBinaryDownload(unittest.TestCase):
    """Test binary download functionality."""
    
    @patch('kotadb.server.urlretrieve')
    @patch('kotadb.server.get_binary_info')
    @patch('kotadb.server.get_platform_info')
    @patch('os.chmod')
    @patch('shutil.move')
    def test_download_binary_success(self, mock_move, mock_chmod, 
                                    mock_platform, mock_binary_info, 
                                    mock_urlretrieve):
        """Test successful binary download."""
        mock_platform.return_value = ('linux', 'x64')
        mock_binary_info.return_value = {
            'url': 'https://example.com/kotadb.tar.gz',
            'sha256': None,
            'extension': 'tar.gz'
        }
        
        with patch('kotadb.server.verify_checksum', return_value=True):
            with patch('tarfile.open') as mock_tar:
                with patch('tempfile.mkdtemp', return_value='/tmp/test'):
                    with patch('os.path.exists', return_value=True):
                        with patch('kotadb.server.BINARY_DIR', Path('/fake/bin')):
                            result = download_binary(force=True)
                            
                            mock_urlretrieve.assert_called_once()
                            mock_tar.assert_called_once()
                            self.assertEqual(result, Path('/fake/bin/kotadb'))
    
    @patch('os.path.exists')
    def test_download_binary_already_exists(self, mock_exists):
        """Test download_binary when binary already exists."""
        mock_exists.return_value = True
        
        with patch('kotadb.server.BINARY_DIR', Path('/fake/bin')):
            result = download_binary(force=False)
            self.assertEqual(result, Path('/fake/bin/kotadb'))
    
    @patch('kotadb.server.urlretrieve')
    @patch('kotadb.server.get_binary_info')
    @patch('kotadb.server.get_platform_info')
    def test_download_binary_network_error(self, mock_platform, 
                                          mock_binary_info, mock_urlretrieve):
        """Test download_binary with network error."""
        from urllib.error import URLError
        
        mock_platform.return_value = ('linux', 'x64')
        mock_binary_info.return_value = {
            'url': 'https://example.com/kotadb.tar.gz',
            'sha256': None,
            'extension': 'tar.gz'
        }
        mock_urlretrieve.side_effect = URLError("Network error")
        
        with patch('tempfile.mkdtemp', return_value='/tmp/test'):
            with patch('kotadb.server.BINARY_DIR', Path('/fake/bin')):
                with self.assertRaises(RuntimeError) as context:
                    download_binary(force=True)
                self.assertIn('Failed to download', str(context.exception))


class TestIntegration(unittest.TestCase):
    """Integration tests for the complete workflow."""
    
    @unittest.skipIf(
        os.environ.get('SKIP_INTEGRATION_TESTS', 'true').lower() == 'true',
        "Skipping integration tests (set SKIP_INTEGRATION_TESTS=false to run)"
    )
    def test_full_server_lifecycle(self):
        """Test complete server lifecycle with real binary."""
        # This test requires actual network access and binary availability
        # It's skipped by default but can be enabled for CI/CD
        
        with tempfile.TemporaryDirectory() as temp_dir:
            data_dir = Path(temp_dir) / "data"
            
            # Download binary
            binary_path = ensure_binary_installed()
            self.assertTrue(binary_path.exists())
            
            # Start server
            server = KotaDBServer(
                data_dir=data_dir,
                port=18081,
                auto_install=False  # Already installed
            )
            
            try:
                server.start(timeout=15)
                time.sleep(2)  # Give server time to fully start
                
                # Check if running
                self.assertTrue(server.is_running())
                
                # Try to connect with client
                from kotadb import KotaDB
                client = KotaDB("http://localhost:18081")
                
                # Perform basic operation
                doc_id = client.insert({
                    "path": "/test/doc.md",
                    "title": "Test Document",
                    "content": "Integration test content"
                })
                self.assertIsNotNone(doc_id)
                
            finally:
                server.stop()
                self.assertFalse(server.is_running())


if __name__ == '__main__':
    unittest.main()