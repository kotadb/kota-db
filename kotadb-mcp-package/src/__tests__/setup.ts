// Jest setup file for global test configuration

// Increase timeout for integration tests
jest.setTimeout(15000);

// Mock console.error to reduce noise in test output
const originalConsoleError = console.error;
beforeAll(() => {
  console.error = (...args: any[]) => {
    // Only suppress KotaDB server startup messages
    if (typeof args[0] === 'string' && args[0].includes('KotaDB MCP server')) {
      return;
    }
    originalConsoleError(...args);
  };
});

afterAll(() => {
  console.error = originalConsoleError;
});