import { spawn, ChildProcess } from 'child_process';
import { tmpdir } from 'os';
import * as path from 'path';
import * as fs from 'fs/promises';
import { EventEmitter } from 'events';

export interface MCPResponse {
  jsonrpc: string;
  id?: number;
  result?: any;
  error?: any;
}

export interface MCPRequest {
  jsonrpc: string;
  method: string;
  params?: any;
  id?: number;
}

export interface TestDocument {
  path: string;
  title?: string;
  content: string;
  tags?: string[];
}

export class MCPTestClient extends EventEmitter {
  private process: ChildProcess | null = null;
  private tempDir: string = '';
  private initialized = false;
  private requestId = 0;
  private pendingRequests = new Map<number, { resolve: Function; reject: Function; timeout: NodeJS.Timeout }>();

  async initialize(): Promise<void> {
    this.tempDir = await fs.mkdtemp(path.join(tmpdir(), 'kotadb-mcp-integration-'));
    await this.startServer();
    this.initialized = true;
  }

  async cleanup(): Promise<void> {
    if (this.process && !this.process.killed) {
      try {
        // More robust process cleanup to prevent Jest hanging
        this.process.kill('SIGTERM');
        
        // Wait for process to exit with proper timeout handling
        await new Promise<void>((resolve) => {
          if (!this.process || this.process.killed) {
            resolve();
            return;
          }
          
          let resolved = false;
          const resolveOnce = () => {
            if (!resolved) {
              resolved = true;
              resolve();
            }
          };
          
          this.process.on('exit', resolveOnce);
          this.process.on('close', resolveOnce);
          
          // Force kill after timeout to prevent Jest hanging
          setTimeout(() => {
            if (this.process && !this.process.killed) {
              this.process.kill('SIGKILL');
            }
            resolveOnce();
          }, 1500);
        });
      } catch (error) {
        console.warn('Error during process cleanup:', error);
      }
      this.process = null;
    }

    if (this.tempDir) {
      try {
        await fs.rm(this.tempDir, { recursive: true, force: true });
      } catch (error) {
        console.warn('Failed to cleanup test directory:', error);
      }
    }

    // Clean up pending requests
    for (const [id, { reject, timeout }] of this.pendingRequests) {
      clearTimeout(timeout);
      reject(new Error('Client cleanup'));
    }
    this.pendingRequests.clear();

    this.initialized = false;
  }

  private async startServer(): Promise<void> {
    return new Promise((resolve, reject) => {
      const serverPath = path.join(__dirname, '..', '..', '..', 'dist', 'index.js');
      const kotadbBinary = this.findKotaDBBinary();
      
      this.process = spawn('node', [serverPath], {
        stdio: ['pipe', 'pipe', 'pipe'],
        env: {
          ...process.env,
          KOTADB_DATA_DIR: this.tempDir,
          KOTADB_BINARY_PATH: kotadbBinary,
        },
      });

      let serverReady = false;

      // Handle server output
      this.process.stdout?.on('data', (data) => {
        const lines = data.toString().split('\n').filter((line: string) => line.trim());
        
        for (const line of lines) {
          try {
            const message = JSON.parse(line);
            this.handleResponse(message);
          } catch (error) {
            // Not JSON, might be startup message
            if (line.includes('7 tools available') && !serverReady) {
              serverReady = true;
              resolve();
            }
          }
        }
      });

      this.process.stderr?.on('data', (data) => {
        const output = data.toString();
        console.error('MCP Server stderr:', output);
        
        // Check for startup completion
        if (output.includes('7 tools available') && !serverReady) {
          serverReady = true;
          resolve();
        }
      });

      this.process.on('error', (error) => {
        console.error('MCP Server process error:', error);
        reject(error);
      });

      this.process.on('exit', (code, signal) => {
        if (code !== 0 && !serverReady) {
          reject(new Error(`Server exited with code ${code}, signal ${signal}`));
        }
        this.emit('server-exit', code, signal);
      });

      // Startup timeout
      setTimeout(() => {
        if (!serverReady) {
          if (this.process) {
            this.process.kill();
          }
          reject(new Error('Server startup timeout'));
        }
      }, 10000);
    });
  }

  private findKotaDBBinary(): string {
    // Look for kotadb binary in various locations
    const possiblePaths = [
      path.join(__dirname, '..', '..', '..', '..', 'target', 'debug', 'kotadb'),
      path.join(__dirname, '..', '..', '..', '..', 'target', 'release', 'kotadb'),
      '/usr/local/bin/kotadb',
      'kotadb', // In PATH
    ];

    return possiblePaths[0]; // Default to debug build
  }

  private handleResponse(response: MCPResponse): void {
    if (response.id !== undefined && this.pendingRequests.has(response.id)) {
      const { resolve, reject, timeout } = this.pendingRequests.get(response.id)!;
      clearTimeout(timeout);
      this.pendingRequests.delete(response.id);

      if (response.error) {
        reject(new Error(`MCP Error: ${JSON.stringify(response.error)}`));
      } else {
        resolve(response);
      }
    } else {
      // Notification or unmatched response
      this.emit('notification', response);
    }
  }

  async sendRequest(method: string, params?: any, timeoutMs: number = 5000): Promise<MCPResponse> {
    if (!this.initialized || !this.process) {
      throw new Error('Client not initialized');
    }

    const id = ++this.requestId;
    const request: MCPRequest = {
      jsonrpc: '2.0',
      method,
      params: params || {},
      id,
    };

    return new Promise((resolve, reject) => {
      const timeout = setTimeout(() => {
        this.pendingRequests.delete(id);
        reject(new Error(`Request timeout: ${method}`));
      }, timeoutMs);
      // Ensure timer doesn't keep the process alive
      timeout.unref();

      this.pendingRequests.set(id, { resolve, reject, timeout });

      const requestJson = JSON.stringify(request) + '\n';
      this.process!.stdin?.write(requestJson);
    });
  }

  async listTools(): Promise<any[]> {
    const response = await this.sendRequest('tools/list');
    return response.result?.tools || [];
  }

  async callTool(name: string, args: any, timeoutMs: number = 5000): Promise<any> {
    const response = await this.sendRequest('tools/call', {
      name,
      arguments: args,
    }, timeoutMs);
    return response.result;
  }

  async listResources(): Promise<any[]> {
    const response = await this.sendRequest('resources/list');
    return response.result?.resources || [];
  }

  async readResource(uri: string): Promise<any> {
    const response = await this.sendRequest('resources/read', { uri });
    return response.result;
  }

  async createDocument(doc: TestDocument, timeoutMs: number = 5000): Promise<any> {
    const result = await this.callTool('kotadb_document_create', doc, timeoutMs);
    const content = JSON.parse(result.content[0].text);
    if (!content.success) {
      throw new Error(`Create failed: ${content.error}`);
    }
    // Map snake_case to camelCase for consistency
    return {
      ...content.document,
      createdAt: content.document.created_at,
      updatedAt: content.document.updated_at,
    };
  }

  async getDocument(id: string): Promise<any> {
    const result = await this.callTool('kotadb_document_get', { id });
    const content = JSON.parse(result.content[0].text);
    if (!content.success) {
      throw new Error(`Get failed: ${content.error}`);
    }
    // Map snake_case to camelCase for consistency
    return {
      ...content.document,
      createdAt: content.document.created_at || content.document.createdAt,
      updatedAt: content.document.updated_at || content.document.updatedAt,
    };
  }

  async updateDocument(id: string, newContent: string): Promise<any> {
    const result = await this.callTool('kotadb_document_update', { 
      id, 
      content: newContent 
    });
    const content = JSON.parse(result.content[0].text);
    if (!content.success) {
      throw new Error(`Update failed: ${content.error}`);
    }
    // Map snake_case to camelCase for consistency
    return {
      ...content.document,
      createdAt: content.document.created_at,
      updatedAt: content.document.updated_at,
    };
  }

  async deleteDocument(id: string): Promise<boolean> {
    const result = await this.callTool('kotadb_document_delete', { id });
    const content = JSON.parse(result.content[0].text);
    return content.success;
  }

  async searchDocuments(query: string, limit: number = 10): Promise<any[]> {
    const result = await this.callTool('kotadb_search', { query, limit });
    const content = JSON.parse(result.content[0].text);
    if (!content.success) {
      throw new Error(`Search failed: ${content.error}`);
    }
    return content.results;
  }

  async listDocuments(limit: number = 50, offset: number = 0): Promise<any> {
    const result = await this.callTool('kotadb_document_list', { limit, offset });
    const content = JSON.parse(result.content[0].text);
    if (!content.success) {
      throw new Error(`List failed: ${content.error}`);
    }
    return content;
  }

  async getStats(): Promise<any> {
    const result = await this.callTool('kotadb_stats', {});
    const content = JSON.parse(result.content[0].text);
    if (!content.success) {
      throw new Error(`Stats failed: ${content.error}`);
    }
    return content.stats;
  }

  getTempDir(): string {
    return this.tempDir;
  }

  isServerRunning(): boolean {
    return this.process !== null && !this.process.killed;
  }
}

export async function createTestClient(): Promise<MCPTestClient> {
  const client = new MCPTestClient();
  await client.initialize();
  return client;
}

export function createTestDocument(overrides: Partial<TestDocument> = {}): TestDocument {
  return {
    path: `/test-${Date.now()}-${Math.random().toString(36).substr(2, 9)}.md`,
    title: `Test Document ${Date.now()}`,
    content: `Test content generated at ${new Date().toISOString()}`,
    tags: ['test', 'integration'],
    ...overrides,
  };
}

// Performance measurement helpers
export class PerformanceTimer {
  private startTime: number = 0;
  private measurements: number[] = [];

  start(): void {
    this.startTime = Date.now();
  }

  end(): number {
    const duration = Date.now() - this.startTime;
    this.measurements.push(duration);
    return duration;
  }

  getAverage(): number {
    return this.measurements.reduce((a, b) => a + b, 0) / this.measurements.length;
  }

  getMedian(): number {
    const sorted = [...this.measurements].sort((a, b) => a - b);
    const middle = Math.floor(sorted.length / 2);
    return sorted.length % 2 === 0
      ? (sorted[middle - 1] + sorted[middle]) / 2
      : sorted[middle];
  }

  getP95(): number {
    const sorted = [...this.measurements].sort((a, b) => a - b);
    const index = Math.floor(sorted.length * 0.95);
    return sorted[index];
  }

  reset(): void {
    this.measurements = [];
  }
}

// Validation helpers
export function validateMCPResponse(response: any): void {
  expect(response).toBeDefined();
  expect(response.jsonrpc).toBe('2.0');
  expect(response.id).toBeDefined();
}

export function validateToolResponse(response: any): void {
  expect(response).toBeDefined();
  expect(response.content).toBeDefined();
  expect(Array.isArray(response.content)).toBe(true);
  expect(response.content.length).toBeGreaterThan(0);
}

export function validateDocumentStructure(doc: any): void {
  expect(doc).toBeDefined();
  expect(doc.id).toBeDefined();
  expect(typeof doc.id).toBe('string');
  expect(doc.path).toBeDefined();
  expect(typeof doc.path).toBe('string');
  expect(doc.title).toBeDefined();
  expect(typeof doc.title).toBe('string');
  expect(doc.content).toBeDefined();
  expect(typeof doc.content).toBe('string');
  expect(doc.createdAt).toBeDefined();
  expect(doc.updatedAt).toBeDefined();
  expect(Array.isArray(doc.tags)).toBe(true);
}

// Error injection helpers (following anti-mock philosophy)
export class ErrorInjectionClient extends MCPTestClient {
  private failureRate: number = 0;
  private networkDelay: number = 0;

  constructor(failureRate: number = 0, networkDelay: number = 0) {
    super();
    this.failureRate = failureRate;
    this.networkDelay = networkDelay;
  }

  async sendRequest(method: string, params?: any, timeoutMs: number = 5000): Promise<MCPResponse> {
    // Simulate network delay
    if (this.networkDelay > 0) {
      await new Promise(resolve => setTimeout(resolve, this.networkDelay));
    }

    // Simulate failures
    if (this.failureRate > 0 && Math.random() < this.failureRate) {
      throw new Error(`Simulated network failure for ${method}`);
    }

    return super.sendRequest(method, params, timeoutMs);
  }
}