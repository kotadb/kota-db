import { execSync } from 'child_process';
import * as path from 'path';
import * as fs from 'fs';

export default async function globalSetup(): Promise<void> {
  console.log('Setting up MCP integration test environment...');
  
  try {
    // Ensure the dist directory exists and contains built files
    const distPath = path.join(__dirname, '..', '..', 'dist');
    const indexJsPath = path.join(distPath, 'index.js');
    
    if (!fs.existsSync(indexJsPath)) {
      console.log('Building MCP package for testing...');
      // Build the TypeScript files
      execSync('npm run build', {
        cwd: path.join(__dirname, '..', '..'),
        stdio: 'inherit'
      });
    }
    
    // Verify the KotaDB binary is available or build it if needed
    const kotadbProjectRoot = path.join(__dirname, '..', '..', '..');
    const debugBinaryPath = path.join(kotadbProjectRoot, 'target', 'debug', 'kotadb');
    
    if (!fs.existsSync(debugBinaryPath)) {
      console.log('Building KotaDB binary for integration tests...');
      try {
        execSync('cargo build', {
          cwd: kotadbProjectRoot,
          stdio: 'inherit'
        });
      } catch (error) {
        console.warn('Failed to build KotaDB binary:', error);
        console.log('Integration tests will attempt to use system kotadb binary');
      }
    }
    
    console.log('MCP integration test environment setup complete');
  } catch (error) {
    console.error('Failed to set up test environment:', error);
    throw error;
  }
}