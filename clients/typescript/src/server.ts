/**
 * KotaDB Server Management for Node.js/TypeScript
 * 
 * This module provides functionality to download, install, and manage KotaDB server binaries.
 * It automatically downloads the appropriate binary for the current platform and provides
 * a simple interface to start and stop the server.
 */

import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import * as crypto from 'crypto';
import * as child_process from 'child_process';
import * as https from 'https';
import * as zlib from 'zlib';
import * as tar from 'tar';
import * as unzipper from 'unzipper';
import { promisify } from 'util';

const exec = promisify(child_process.exec);
const mkdir = promisify(fs.mkdir);
const chmod = promisify(fs.chmod);
const access = promisify(fs.access);
const readFile = promisify(fs.readFile);
const writeFile = promisify(fs.writeFile);

// Version should match the KotaDB release version
const KOTADB_VERSION = '0.1.12';

// Binary download configuration
const BINARY_BASE_URL = 'https://github.com/jayminwest/kota-db/releases/download';
const BINARY_MANIFEST_URL = `${BINARY_BASE_URL}/v${KOTADB_VERSION}/manifest.json`;

// Local storage paths
const KOTADB_HOME = path.join(os.homedir(), '.kotadb');
const BINARY_DIR = path.join(KOTADB_HOME, 'bin');
const CONFIG_DIR = path.join(KOTADB_HOME, 'config');
const DATA_DIR = path.join(KOTADB_HOME, 'data');

/**
 * Platform information
 */
interface PlatformInfo {
  platform: string;
  arch: string;
}

/**
 * Binary download information
 */
interface BinaryInfo {
  url: string;
  sha256?: string | undefined;
  extension: string;
}

/**
 * Server configuration options
 */
export interface ServerOptions {
  /** Directory for database files */
  dataDir?: string;
  /** Port to run the server on */
  port?: number;
  /** Automatically download binary if not present */
  autoInstall?: boolean;
  /** Path to custom configuration file */
  configPath?: string;
}

/**
 * Detect the current platform and architecture.
 */
function getPlatformInfo(): PlatformInfo {
  const platform = os.platform();
  const arch = os.arch();
  
  // Map platform names
  let platformName: string;
  switch (platform) {
    case 'darwin':
      platformName = 'macos';
      break;
    case 'linux':
      // TODO: Detect musl vs glibc
      platformName = 'linux';
      break;
    case 'win32':
      platformName = 'windows';
      break;
    default:
      throw new Error(`Unsupported platform: ${platform}`);
  }
  
  // Map architecture names
  let archName: string;
  switch (arch) {
    case 'x64':
    case 'x86_64':
      archName = 'x64';
      break;
    case 'arm64':
    case 'aarch64':
      archName = 'arm64';
      break;
    default:
      throw new Error(`Unsupported architecture: ${arch}`);
  }
  
  return { platform: platformName, arch: archName };
}

/**
 * Get the appropriate binary download information for the current platform.
 */
async function getBinaryInfo(): Promise<BinaryInfo> {
  const { platform, arch } = getPlatformInfo();
  const binaryKey = `${platform}-${arch}`;
  
  // Try to fetch manifest from GitHub release
  try {
    const manifest = await fetchJSON(BINARY_MANIFEST_URL);
    const binaries = manifest.binaries || {};
    
    if (!binaries[binaryKey]) {
      throw new Error(`No binary available for platform: ${binaryKey}`);
    }
    
    const binaryInfo = binaries[binaryKey];
    return {
      url: `${BINARY_BASE_URL}/v${KOTADB_VERSION}/${binaryInfo.url}`,
      sha256: binaryInfo.sha256,
      extension: platform === 'windows' ? 'zip' : 'tar.gz'
    };
  } catch (error) {
    // Fallback to hardcoded URLs
    const fallbackBinaries: Record<string, BinaryInfo> = {
      'linux-x64': {
        url: `${BINARY_BASE_URL}/v${KOTADB_VERSION}/kotadb-linux-x64.tar.gz`,
        sha256: undefined,
        extension: 'tar.gz'
      },
      'macos-x64': {
        url: `${BINARY_BASE_URL}/v${KOTADB_VERSION}/kotadb-macos-x64.tar.gz`,
        sha256: undefined,
        extension: 'tar.gz'
      },
      'macos-arm64': {
        url: `${BINARY_BASE_URL}/v${KOTADB_VERSION}/kotadb-macos-arm64.tar.gz`,
        sha256: undefined,
        extension: 'tar.gz'
      },
      'windows-x64': {
        url: `${BINARY_BASE_URL}/v${KOTADB_VERSION}/kotadb-windows-x64.zip`,
        sha256: undefined,
        extension: 'zip'
      }
    };
    
    if (!fallbackBinaries[binaryKey]) {
      throw new Error(`No binary available for platform: ${binaryKey}`);
    }
    
    return fallbackBinaries[binaryKey];
  }
}

/**
 * Fetch JSON from a URL
 */
function fetchJSON(url: string): Promise<any> {
  return new Promise((resolve, reject) => {
    https.get(url, (res) => {
      let data = '';
      res.on('data', (chunk) => data += chunk);
      res.on('end', () => {
        try {
          resolve(JSON.parse(data));
        } catch (e) {
          reject(e);
        }
      });
    }).on('error', reject);
  });
}

/**
 * Download a file from a URL
 */
function downloadFile(url: string, destPath: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const file = fs.createWriteStream(destPath);
    
    https.get(url, (response) => {
      // Handle redirects
      if (response.statusCode === 302 || response.statusCode === 301) {
        const redirectUrl = response.headers.location;
        if (!redirectUrl) {
          reject(new Error('Redirect without location header'));
          return;
        }
        downloadFile(redirectUrl, destPath).then(resolve).catch(reject);
        return;
      }
      
      if (response.statusCode !== 200) {
        reject(new Error(`Failed to download: ${response.statusCode}`));
        return;
      }
      
      response.pipe(file);
      
      file.on('finish', () => {
        file.close(() => resolve());
      });
    }).on('error', (err) => {
      fs.unlink(destPath, () => {}); // Delete partial file
      reject(err);
    });
  });
}

/**
 * Verify SHA256 checksum of a file
 */
async function verifyChecksum(filePath: string, expectedSha256?: string): Promise<boolean> {
  if (!expectedSha256) {
    console.warn('Warning: No checksum available for verification');
    return true;
  }
  
  const fileBuffer = await readFile(filePath);
  const hash = crypto.createHash('sha256').update(fileBuffer).digest('hex');
  
  if (hash !== expectedSha256) {
    console.error(`Checksum mismatch! Expected: ${expectedSha256}, Got: ${hash}`);
    return false;
  }
  
  return true;
}

/**
 * Extract archive (tar.gz or zip)
 */
async function extractArchive(archivePath: string, destDir: string, extension: string): Promise<void> {
  if (extension === 'zip') {
    // Extract ZIP file
    return new Promise((resolve, reject) => {
      fs.createReadStream(archivePath)
        .pipe(unzipper.Extract({ path: destDir }))
        .on('close', resolve)
        .on('error', reject);
    });
  } else {
    // Extract tar.gz file
    await tar.extract({
      file: archivePath,
      cwd: destDir
    });
  }
}

/**
 * Download and install the KotaDB binary for the current platform.
 */
export async function downloadBinary(force: boolean = false): Promise<string> {
  const platform = os.platform();
  const binaryName = platform === 'win32' ? 'kotadb.exe' : 'kotadb';
  const binaryPath = path.join(BINARY_DIR, binaryName);
  
  // Check if binary already exists
  try {
    await access(binaryPath, fs.constants.F_OK);
    if (!force) {
      console.log(`KotaDB binary already installed at ${binaryPath}`);
      return binaryPath;
    }
  } catch {
    // Binary doesn't exist, continue with download
  }
  
  // Create directories
  await mkdir(BINARY_DIR, { recursive: true });
  
  // Get binary information
  console.log('Detecting platform...');
  const platformInfo = getPlatformInfo();
  console.log(`Platform: ${platformInfo.platform}-${platformInfo.arch}`);
  
  const binaryInfo = await getBinaryInfo();
  
  // Download binary
  console.log(`Downloading KotaDB v${KOTADB_VERSION}...`);
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'kotadb-'));
  const archiveName = `kotadb.${binaryInfo.extension}`;
  const archivePath = path.join(tempDir, archiveName);
  
  try {
    await downloadFile(binaryInfo.url, archivePath);
    
    // Verify checksum
    if (!await verifyChecksum(archivePath, binaryInfo.sha256)) {
      throw new Error('Binary checksum verification failed');
    }
    
    // Extract binary
    console.log('Extracting binary...');
    await extractArchive(archivePath, tempDir, binaryInfo.extension);
    
    // Find and move the binary
    const extractedBinary = path.join(tempDir, binaryName);
    if (!fs.existsSync(extractedBinary)) {
      // Binary might be in a subdirectory
      const files = fs.readdirSync(tempDir, { recursive: true });
      const foundBinary = files.find(f => f.toString().endsWith(binaryName));
      if (!foundBinary) {
        throw new Error(`Binary ${binaryName} not found in archive`);
      }
      fs.renameSync(path.join(tempDir, foundBinary.toString()), binaryPath);
    } else {
      fs.renameSync(extractedBinary, binaryPath);
    }
    
    // Make executable on Unix systems
    if (platform !== 'win32') {
      await chmod(binaryPath, 0o755);
    }
    
    // Also extract MCP server if present
    const mcpName = platform === 'win32' ? 'mcp_server.exe' : 'mcp_server';
    const mcpSource = path.join(tempDir, mcpName);
    if (fs.existsSync(mcpSource)) {
      const mcpDest = path.join(BINARY_DIR, mcpName);
      fs.renameSync(mcpSource, mcpDest);
      if (platform !== 'win32') {
        await chmod(mcpDest, 0o755);
      }
      console.log(`MCP server installed at ${mcpDest}`);
    }
  } finally {
    // Clean up temp directory
    fs.rmSync(tempDir, { recursive: true, force: true });
  }
  
  console.log(`KotaDB binary installed at ${binaryPath}`);
  return binaryPath;
}

/**
 * KotaDB Server Manager
 * 
 * This class provides methods to start, stop, and check the status of a KotaDB server.
 * It handles binary installation, configuration, and process management.
 */
export class KotaDBServer {
  private dataDir: string;
  private port: number;
  private process?: child_process.ChildProcess | undefined;
  private binaryPath?: string;
  
  constructor(options: ServerOptions = {}) {
    this.dataDir = options.dataDir || DATA_DIR;
    this.port = options.port || 8080;
    
    // Ensure directories exist
    fs.mkdirSync(this.dataDir, { recursive: true });
    fs.mkdirSync(CONFIG_DIR, { recursive: true });
    
    // Install binary if needed
    if (options.autoInstall !== false) {
      this.ensureBinary();
    }
  }
  
  /**
   * Ensure the binary is installed
   */
  private async ensureBinary(): Promise<void> {
    try {
      this.binaryPath = await downloadBinary();
    } catch (error) {
      console.warn(`Warning: Could not download binary: ${error}`);
      console.log('Attempting to use system-installed kotadb...');
      
      // Try to find kotadb in PATH
      try {
        const { stdout } = await exec('which kotadb');
        this.binaryPath = stdout.trim();
      } catch {
        throw new Error(
          'KotaDB binary not found. Please install manually or ensure internet connection.'
        );
      }
    }
  }
  
  /**
   * Create a configuration file for the server
   */
  private async createConfig(configPath?: string): Promise<string> {
    if (!configPath) {
      configPath = path.join(CONFIG_DIR, `kotadb-${this.port}.toml`);
    }
    
    const configContent = `
# KotaDB Server Configuration
# Auto-generated by kotadb TypeScript package

[server]
host = "127.0.0.1"
port = ${this.port}

[storage]
data_dir = "${this.dataDir}"
wal_enabled = true
cache_size = 1000

[logging]
level = "info"
format = "pretty"
`;
    
    await writeFile(configPath, configContent);
    return configPath;
  }
  
  /**
   * Start the KotaDB server
   */
  async start(configPath?: string, timeout: number = 10000): Promise<void> {
    if (this.process) {
      console.log('Server is already running');
      return;
    }
    
    // Ensure binary is available
    if (!this.binaryPath) {
      await this.ensureBinary();
    }
    
    if (!this.binaryPath || !fs.existsSync(this.binaryPath)) {
      throw new Error('KotaDB binary not found. Run downloadBinary() first.');
    }
    
    // Create config if not provided
    if (!configPath) {
      configPath = await this.createConfig();
    }
    
    // Start the server process
    console.log(`Starting KotaDB server on port ${this.port}...`);
    
    this.process = child_process.spawn(
      this.binaryPath,
      ['--config', configPath],
      {
        stdio: 'pipe',
        detached: false
      }
    );
    
    // Handle process events
    this.process.on('error', (err) => {
      console.error('Server process error:', err);
    });
    
    this.process.on('exit', (code, signal) => {
      console.log(`Server process exited with code ${code} and signal ${signal}`);
      this.process = undefined;
    });
    
    // Wait for server to be ready
    const startTime = Date.now();
    while (Date.now() - startTime < timeout) {
      if (await this.isRunning()) {
        console.log(`KotaDB server started successfully on port ${this.port}`);
        return;
      }
      await new Promise(resolve => setTimeout(resolve, 500));
    }
    
    // If we get here, server failed to start
    this.stop();
    throw new Error(`Server failed to start within ${timeout}ms`);
  }
  
  /**
   * Stop the KotaDB server
   */
  stop(): void {
    if (!this.process) {
      console.log('Server is not running');
      return;
    }
    
    console.log('Stopping KotaDB server...');
    this.process.kill('SIGTERM');
    
    // Give it time to shut down gracefully
    setTimeout(() => {
      if (this.process) {
        this.process.kill('SIGKILL');
      }
    }, 5000);
    
    this.process = undefined;
    console.log('KotaDB server stopped');
  }
  
  /**
   * Check if the server is running
   */
  async isRunning(): Promise<boolean> {
    if (!this.process) {
      return false;
    }
    
    // Try to connect to the server
    return new Promise((resolve) => {
      const net = require('net');
      const socket = new net.Socket();
      
      socket.setTimeout(1000);
      socket.on('connect', () => {
        socket.destroy();
        resolve(true);
      });
      
      socket.on('error', () => {
        resolve(false);
      });
      
      socket.on('timeout', () => {
        socket.destroy();
        resolve(false);
      });
      
      socket.connect(this.port, '127.0.0.1');
    });
  }
}

/**
 * Start a KotaDB server with default settings
 */
export async function startServer(options: ServerOptions = {}): Promise<KotaDBServer> {
  const server = new KotaDBServer(options);
  await server.start();
  return server;
}

/**
 * Ensure the KotaDB binary is installed
 */
export async function ensureBinaryInstalled(): Promise<string> {
  return downloadBinary();
}