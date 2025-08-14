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
      
      console.log(`✅ KotaDB binary installed at: ${binaryPath}`);
      console.log('🚀 kotadb-mcp is ready to use!');
      
    } catch (error) {
      console.warn('⚠️  Binary installation failed:', error instanceof Error ? error.message : error);
      console.log('📝 You can manually install the KotaDB binary and ensure it\'s in your PATH');
      console.log('   See: https://github.com/jayminwest/kota-db#installation');
      // Don't fail the install - users can manually install the binary
    }
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
   * Find the KotaDB binary path
   */
  static findBinary(): string {
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