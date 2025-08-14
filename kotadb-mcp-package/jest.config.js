module.exports = {
  preset: 'ts-jest',
  testEnvironment: 'node',
  rootDir: '.',
  roots: ['<rootDir>/src/__tests__'],
  testMatch: ['<rootDir>/src/__tests__/**/*.test.ts'],
  testPathIgnorePatterns: [
    '<rootDir>/node_modules/',
    '<rootDir>/dist/',
    '<rootDir>/../clients/',
    '<rootDir>/../tests/',
  ],
  transform: {
    '^.+\\.ts$': 'ts-jest',
  },
  collectCoverageFrom: [
    'src/**/*.ts',
    '!src/**/*.d.ts',
    '!src/__tests__/**',
    '!src/index.ts', // Skip main entry point in coverage (integration tested)
  ],
  coverageDirectory: 'coverage',
  coverageReporters: ['text', 'lcov', 'html'],
  coverageThreshold: {
    global: {
      branches: 60,
      functions: 60,
      lines: 60,
      statements: 60,
    },
  },
  setupFilesAfterEnv: ['<rootDir>/src/__tests__/setup.ts'],
  testTimeout: 30000, // 30 seconds for comprehensive integration tests
  moduleFileExtensions: ['ts', 'js', 'json'],
  // Run tests in bands for better stability with integration tests
  maxConcurrency: 2,
  // Global setup and teardown for integration tests
  // globalSetup: '<rootDir>/src/__tests__/global-setup.ts',
  // globalTeardown: '<rootDir>/src/__tests__/global-teardown.ts',
};