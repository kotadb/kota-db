#!/usr/bin/env node

import * as fs from 'fs/promises';
import * as path from 'path';
import * as os from 'os';
import { createWriteStream } from 'fs';
import { pipeline } from 'stream/promises';
import fetch from 'node-fetch';
import { extract } from 'tar';

interface BinaryInfo {
  platform: string;
  arch: string;
  filename: string;
  url: string;
}

class BinaryInstaller {
  private readonly binDir: string;
  private readonly version: string = '0.3.1'; // Should match KotaDB version
  private readonly baseUrl: string = 'https://github.com/jayminwest/kota-db/releases/download';

  constructor() {
    this.binDir = path.join(__dirname, '..', 'bin');
  }

  async install(): Promise<void> {
    console.log('🔧 Installing KotaDB binary...');
    
    try {
      const binaryInfo = this.getBinaryInfo();
      await this.ensureBinDirectory();
      
      // For now, we'll build from source if available, or download from GitHub releases
      const binaryPath = await this.installBinary(binaryInfo);
      
      // Validate the installed binary
      const isValid = await BinaryInstaller.validateBinaryPath(binaryPath);
      if (!isValid) {
        throw new Error(`Installed binary at ${binaryPath} is not executable or valid`);
      }

      // Test that the binary actually works
      try {
        await this.testBinary(binaryPath);
        console.log(`✅ KotaDB binary installed and verified at: ${binaryPath}`);
        console.log('🚀 kotadb-mcp is ready to use!');
      } catch (testError) {
        console.warn(`⚠️  Binary installed but failed verification test: ${testError instanceof Error ? testError.message : testError}`);
        console.log(`📍 Binary location: ${binaryPath}`);
        console.log('🔍 The binary may work for some operations but encountered issues during testing');
      }
      
    } catch (error) {
      console.warn('⚠️  Binary installation failed:', error instanceof Error ? error.message : error);
      console.log('📝 You can manually install the KotaDB binary and ensure it\'s in your PATH');
      console.log('   See: https://github.com/jayminwest/kota-db#installation');
      // Don't fail the install - users can manually install the binary
    }
  }

  /**
   * Test that the installed binary works correctly
   */
  private async testBinary(binaryPath: string): Promise<void> {
    const { spawn } = await import('child_process');
    const { promisify } = await import('util');
    
    return new Promise((resolve, reject) => {
      const child = spawn(binaryPath, ['--version'], {
        stdio: ['ignore', 'pipe', 'pipe'],
        timeout: 5000, // 5 second timeout
      });

      let stdout = '';
      let stderr = '';

      child.stdout?.on('data', (data) => {
        stdout += data.toString();
      });

      child.stderr?.on('data', (data) => {
        stderr += data.toString();
      });

      child.on('close', (code) => {
        if (code === 0) {
          // Check if output looks like a version string
          if (stdout.includes('kotadb') || stdout.match(/\d+\.\d+\.\d+/)) {
            resolve();
          } else {
            reject(new Error(`Binary test produced unexpected output: ${stdout}`));
          }
        } else {
          reject(new Error(`Binary test failed with code ${code}. stderr: ${stderr}`));
        }
      });

      child.on('error', (error) => {
        reject(new Error(`Failed to execute binary: ${error.message}`));
      });

      // Handle timeout
      setTimeout(() => {
        child.kill();
        reject(new Error('Binary test timed out after 5 seconds'));
      }, 5000);
    });
  }

  private getBinaryInfo(): BinaryInfo {
    const platform = os.platform();
    const arch = os.arch();
    
    let filename: string;
    let platformName: string;
    
    switch (platform) {
      case 'darwin':
        platformName = 'macos';
        filename = `kotadb-${platformName}`;
        break;
      case 'linux':
        platformName = 'linux';
        filename = `kotadb-${platformName}`;
        break;
      case 'win32':
        platformName = 'windows';
        filename = `kotadb-${platformName}.exe`;
        break;
      default:
        throw new Error(`Unsupported platform: ${platform}`);
    }

    if (arch !== 'x64' && arch !== 'arm64') {
      throw new Error(`Unsupported architecture: ${arch}`);
    }

    const url = `${this.baseUrl}/v${this.version}/${filename}`;
    
    return {
      platform: platformName,
      arch,
      filename,
      url,
    };
  }

  private async ensureBinDirectory(): Promise<void> {
    try {
      await fs.mkdir(this.binDir, { recursive: true });
    } catch (error) {
      throw new Error(`Failed to create bin directory: ${error}`);
    }
  }

  private async installBinary(binaryInfo: BinaryInfo): Promise<string> {
    const binaryPath = path.join(this.binDir, 'kotadb');
    
    // Try to build from source first if we're in development
    const sourcePath = path.join(__dirname, '..', '..', '..', 'target', 'release', 'kotadb');
    try {
      await fs.access(sourcePath);
      console.log('📦 Found local KotaDB binary, copying...');
      await fs.copyFile(sourcePath, binaryPath);
      await fs.chmod(binaryPath, 0o755);
      return binaryPath;
    } catch {
      // Local binary not found, try to download
    }

    // Try debug build
    const debugPath = path.join(__dirname, '..', '..', '..', 'target', 'debug', 'kotadb');
    try {
      await fs.access(debugPath);
      console.log('🔨 Found local KotaDB debug binary, copying...');
      await fs.copyFile(debugPath, binaryPath);
      await fs.chmod(binaryPath, 0o755);
      return binaryPath;
    } catch {
      // Local debug binary not found, try to download
    }

    // Download from GitHub releases
    console.log(`⬇️  Downloading KotaDB binary from ${binaryInfo.url}`);
    const response = await fetch(binaryInfo.url);
    
    if (!response.ok) {
      throw new Error(`Failed to download binary: ${response.statusText}`);
    }

    if (!response.body) {
      throw new Error('No response body received');
    }

    const fileStream = createWriteStream(binaryPath);
    await pipeline(response.body, fileStream);
    
    // Make binary executable
    await fs.chmod(binaryPath, 0o755);
    
    return binaryPath;
  }

  /**
   * Find the KotaDB binary path with comprehensive validation
   */
  static async findBinary(): Promise<string> {
    const packageDir = path.join(__dirname, '..');
    const binaryPath = path.join(packageDir, 'bin', 'kotadb');
    
    // 1. Check if binary exists in package bin directory
    if (await BinaryInstaller.validateBinaryPath(binaryPath)) {
      return binaryPath;
    }
    
    // 2. Check in PATH environment variable
    const pathBinary = await BinaryInstaller.findInPath('kotadb');
    if (pathBinary && await BinaryInstaller.validateBinaryPath(pathBinary)) {
      return pathBinary;
    }
    
    // 3. Check common installation directories
    const commonPaths = BinaryInstaller.getCommonBinaryPaths();
    for (const commonPath of commonPaths) {
      if (await BinaryInstaller.validateBinaryPath(commonPath)) {
        return commonPath;
      }
    }
    
    // 4. Fall back to 'kotadb' and let the system handle it
    return 'kotadb';
  }

  /**
   * Validate that a binary path exists and is executable
   */
  static async validateBinaryPath(binaryPath: string): Promise<boolean> {
    try {
      const stat = await fs.stat(binaryPath);
      
      // Check if it's a file (not a directory)
      if (!stat.isFile()) {
        return false;
      }
      
      // Check if it's executable (on Unix-like systems)
      if (process.platform !== 'win32') {
        await fs.access(binaryPath, fs.constants.X_OK);
      } else {
        // On Windows, just check if it exists and has .exe extension
        if (!binaryPath.toLowerCase().endsWith('.exe')) {
          const exePath = binaryPath + '.exe';
          try {
            await fs.access(exePath);
            return true;
          } catch {
            return false;
          }
        }
      }
      
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Find a binary in the system PATH
   */
  static async findInPath(binaryName: string): Promise<string | null> {
    const pathEnv = process.env.PATH || '';
    const pathSeparator = process.platform === 'win32' ? ';' : ':';
    const pathDirs = pathEnv.split(pathSeparator);
    
    for (const dir of pathDirs) {
      if (!dir.trim()) continue;
      
      const binaryPath = path.join(dir, binaryName);
      
      // Try with and without .exe on Windows
      const candidates = process.platform === 'win32' 
        ? [binaryPath, binaryPath + '.exe']
        : [binaryPath];
      
      for (const candidate of candidates) {
        if (await BinaryInstaller.validateBinaryPath(candidate)) {
          return candidate;
        }
      }
    }
    
    return null;
  }

  /**
   * Get common binary installation paths based on platform
   */
  static getCommonBinaryPaths(): string[] {
    const platform = process.platform;
    const binaryName = platform === 'win32' ? 'kotadb.exe' : 'kotadb';
    
    const commonPaths: string[] = [];
    
    if (platform === 'darwin') {
      // macOS common paths
      commonPaths.push(
        path.join(os.homedir(), '.local', 'bin', binaryName),
        path.join('/usr', 'local', 'bin', binaryName),
        path.join('/opt', 'homebrew', 'bin', binaryName),
        path.join('/usr', 'bin', binaryName)
      );
    } else if (platform === 'linux') {
      // Linux common paths
      commonPaths.push(
        path.join(os.homedir(), '.local', 'bin', binaryName),
        path.join('/usr', 'local', 'bin', binaryName),
        path.join('/usr', 'bin', binaryName),
        path.join('/snap', 'bin', binaryName)
      );
    } else if (platform === 'win32') {
      // Windows common paths
      const programFiles = process.env.PROGRAMFILES || 'C:\\Program Files';
      const programFilesX86 = process.env['PROGRAMFILES(X86)'] || 'C:\\Program Files (x86)';
      
      commonPaths.push(
        path.join(programFiles, 'KotaDB', binaryName),
        path.join(programFilesX86, 'KotaDB', binaryName),
        path.join(os.homedir(), 'AppData', 'Local', 'KotaDB', binaryName)
      );
    }
    
    return commonPaths;
  }

  /**
   * Synchronous version for backwards compatibility
   */
  static findBinarySync(): string {
    const packageDir = path.join(__dirname, '..');
    const binaryPath = path.join(packageDir, 'bin', 'kotadb');
    
    // Check if binary exists in package bin directory
    try {
      require('fs').accessSync(binaryPath, require('fs').constants.X_OK);
      return binaryPath;
    } catch {
      // Fall back to PATH
      return 'kotadb';
    }
  }
}

// Run installation when this script is executed directly
if (require.main === module) {
  const installer = new BinaryInstaller();
  installer.install().catch((error) => {
    console.error('Installation failed:', error);
    process.exit(1);
  });
}

export { BinaryInstaller };