import { spawn, ChildProcess } from 'child_process';
import { tmpdir } from 'os';
import * as path from 'path';
import * as fs from 'fs/promises';

describe('KotaDB MCP Server', () => {
  let serverProcess: ChildProcess;
  let tempDir: string;

  beforeEach(async () => {
    tempDir = await fs.mkdtemp(path.join(tmpdir(), 'kotadb-mcp-test-'));
  });

  afterEach(async () => {
    if (serverProcess) {
      serverProcess.kill();
    }
    try {
      await fs.rm(tempDir, { recursive: true, force: true });
    } catch (error) {
      console.warn('Failed to cleanup test directory:', error);
    }
  });

  const startServer = (): Promise<ChildProcess> => {
    return new Promise((resolve, reject) => {
      const serverPath = path.join(__dirname, '..', '..', 'dist', 'index.js');
      const serverProcess = spawn('node', [serverPath], {
        stdio: ['pipe', 'pipe', 'pipe'],
        env: {
          ...process.env,
          KOTADB_DATA_DIR: tempDir,
        },
      });

      let initialized = false;

      serverProcess.stderr?.on('data', (data: Buffer) => {
        const output = data.toString();
        console.log('Server stderr:', output); // Debug output for CI
        if (output.includes('7 tools available') && !initialized) {
          initialized = true;
          resolve(serverProcess);
        }
      });

      serverProcess.stdout?.on('data', (data: Buffer) => {
        const output = data.toString();
        console.log('Server stdout:', output); // Debug output for CI
      });

      serverProcess.on('error', (error) => {
        console.log('Server process error:', error);
        reject(error);
      });

      serverProcess.on('exit', (code, signal) => {
        if (!initialized) {
          console.log(`Server exited early with code ${code}, signal ${signal}`);
          reject(new Error(`Server process exited early with code ${code}, signal ${signal}`));
        }
      });
      
      // Timeout after 10 seconds (increased for CI)
      setTimeout(() => {
        if (!initialized) {
          serverProcess.kill();
          reject(new Error('Server startup timeout'));
        }
      }, 10000);
    });
  };

  const sendJsonRpc = (process: ChildProcess, method: string, params: any = {}, id: number = 1): Promise<any> => {
    return new Promise((resolve, reject) => {
      const request = JSON.stringify({
        jsonrpc: '2.0',
        method,
        params,
        id,
      });

      let responseReceived = false;

      const responseHandler = (data: Buffer) => {
        if (responseReceived) return;
        
        try {
          const response = JSON.parse(data.toString().trim());
          responseReceived = true;
          process.stdout?.off('data', responseHandler);
          resolve(response);
        } catch (error) {
          // Might be partial JSON, wait for more data
        }
      };

      process.stdout?.on('data', responseHandler);
      
      // Send request
      process.stdin?.write(request + '\n');

      // Timeout after 3 seconds
      setTimeout(() => {
        if (!responseReceived) {
          process.stdout?.off('data', responseHandler);
          reject(new Error('Response timeout'));
        }
      }, 3000);
    });
  };

  test('should start server and list tools', async () => {
    serverProcess = await startServer();
    
    const response = await sendJsonRpc(serverProcess, 'tools/list');
    
    expect(response.result).toBeDefined();
    expect(response.result.tools).toBeDefined();
    expect(Array.isArray(response.result.tools)).toBe(true);
    expect(response.result.tools).toHaveLength(7);
    
    const toolNames = response.result.tools.map((tool: any) => tool.name);
    expect(toolNames).toContain('kotadb_document_create');
    expect(toolNames).toContain('kotadb_document_get');
    expect(toolNames).toContain('kotadb_document_update');
    expect(toolNames).toContain('kotadb_document_delete');
    expect(toolNames).toContain('kotadb_document_list');
    expect(toolNames).toContain('kotadb_search');
    expect(toolNames).toContain('kotadb_stats');
  });

  test('should create and retrieve a document', async () => {
    serverProcess = await startServer();
    
    // Create document
    const createResponse = await sendJsonRpc(serverProcess, 'tools/call', {
      name: 'kotadb_document_create',
      arguments: {
        path: '/test-doc.md',
        title: 'Test Document',
        content: 'This is a test document for MCP server testing',
        tags: ['test', 'mcp'],
      },
    });
    
    expect(createResponse.result).toBeDefined();
    const responseContent = JSON.parse(createResponse.result.content[0].text);
    expect(responseContent.success).toBe(true);
    expect(responseContent.document.id).toBeDefined();
    
    const documentId = responseContent.document.id;
    
    // Retrieve document
    const getResponse = await sendJsonRpc(serverProcess, 'tools/call', {
      name: 'kotadb_document_get',
      arguments: {
        id: documentId,
      },
    }, 2);
    
    expect(getResponse.result).toBeDefined();
    const getContent = JSON.parse(getResponse.result.content[0].text);
    expect(getContent.success).toBe(true);
    expect(getContent.document.id).toBe(documentId);
    expect(getContent.document.title).toBe('Test Document');
    expect(getContent.document.content).toBe('This is a test document for MCP server testing');
  });

  test('should search documents', async () => {
    serverProcess = await startServer();
    
    // Create a document to search for
    await sendJsonRpc(serverProcess, 'tools/call', {
      name: 'kotadb_document_create',
      arguments: {
        path: '/searchable.md',
        title: 'Searchable Document',
        content: 'This document contains searchable content about TypeScript testing',
        tags: ['typescript', 'testing'],
      },
    });
    
    // Search for the document
    const searchResponse = await sendJsonRpc(serverProcess, 'tools/call', {
      name: 'kotadb_search',
      arguments: {
        query: 'TypeScript',
        limit: 5,
      },
    }, 2);
    
    expect(searchResponse.result).toBeDefined();
    const searchContent = JSON.parse(searchResponse.result.content[0].text);
    expect(searchContent.success).toBe(true);
    expect(searchContent.results).toHaveLength(1);
    expect(searchContent.results[0].title).toBe('Searchable Document');
    expect(searchContent.results[0].score).toBeGreaterThan(0);
  });

  test('should handle invalid tool calls', async () => {
    serverProcess = await startServer();
    
    const response = await sendJsonRpc(serverProcess, 'tools/call', {
      name: 'kotadb_nonexistent_tool',
      arguments: {},
    });
    
    expect(response.result).toBeDefined();
    expect(response.result.isError).toBe(true);
    const content = JSON.parse(response.result.content[0].text);
    expect(content.success).toBe(false);
    expect(content.error).toContain('Unknown tool');
  });

  test('should handle missing document errors', async () => {
    serverProcess = await startServer();
    
    const response = await sendJsonRpc(serverProcess, 'tools/call', {
      name: 'kotadb_document_get',
      arguments: {
        id: 'nonexistent-document-id',
      },
    });
    
    expect(response.result).toBeDefined();
    expect(response.result.isError).toBe(true);
    const content = JSON.parse(response.result.content[0].text);
    expect(content.success).toBe(false);
    expect(content.error).toContain('not found');
  });

  test('should list resources', async () => {
    serverProcess = await startServer();
    
    const response = await sendJsonRpc(serverProcess, 'resources/list');
    
    expect(response.result).toBeDefined();
    expect(response.result.resources).toBeDefined();
    expect(Array.isArray(response.result.resources)).toBe(true);
    expect(response.result.resources).toHaveLength(1);
    expect(response.result.resources[0].uri).toBe('kotadb://documents');
  });

  test('should read resources', async () => {
    serverProcess = await startServer();
    
    // Create a document first
    await sendJsonRpc(serverProcess, 'tools/call', {
      name: 'kotadb_document_create',
      arguments: {
        path: '/resource-test.md',
        title: 'Resource Test',
        content: 'Testing resource reading',
      },
    });
    
    // Read resources
    const response = await sendJsonRpc(serverProcess, 'resources/read', {
      uri: 'kotadb://documents',
    }, 2);
    
    expect(response.result).toBeDefined();
    expect(response.result.contents).toBeDefined();
    expect(response.result.contents).toHaveLength(1);
    
    const resourceContent = JSON.parse(response.result.contents[0].text);
    expect(resourceContent.documents).toBeDefined();
    expect(resourceContent.total).toBe(1);
    expect(resourceContent.documents[0].title).toBe('Resource Test');
  });
});