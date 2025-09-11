---
tags:
- file
- kota-db
- ext_json
---
{
  "name": "kotadb-mcp",
  "version": "0.5.0",
  "description": "Model Context Protocol server for KotaDB - Enable Claude Desktop to search and manage your KotaDB documents",
  "keywords": [
    "kotadb",
    "mcp",
    "claude",
    "ai",
    "database",
    "search",
    "knowledge-management"
  ],
  "author": "KotaDB Team",
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "https://github.com/jayminwest/kota-db.git",
    "directory": "kotadb-mcp-package"
  },
  "bugs": {
    "url": "https://github.com/jayminwest/kota-db/issues"
  },
  "homepage": "https://github.com/jayminwest/kota-db#readme",
  "type": "module",
  "main": "dist/index.js",
  "bin": {
    "kotadb-mcp": "dist/index.js",
    "kotadb-mcp-setup": "dist/setup.js"
  },
  "scripts": {
    "build": "tsc",
    "dev": "ts-node src/index.ts",
    "setup-claude": "ts-node src/setup.ts",
    "test": "jest",
    "test:unit": "jest src/__tests__ --testPathIgnorePatterns=integration",
    "test:integration": "jest src/__tests__/integration",
    "test:watch": "jest --watch",
    "test:coverage": "jest --coverage",
    "test:all": "npm run test:unit && npm run test:integration",
    "postinstall": "node dist/install-binary.js",
    "prepublishOnly": "npm run build && npm run test:all"
  },
  "dependencies": {
    "@modelcontextprotocol/sdk": "^0.6.0",
    "commander": "^12.0.0",
    "node-fetch": "^3.3.0",
    "tar": "^6.1.0",
    "zod": "^3.22.0"
  },
  "devDependencies": {
    "@types/jest": "^29.0.0",
    "@types/node": "^20.0.0",
    "@types/tar": "^6.1.0",
    "jest": "^29.0.0",
    "ts-jest": "^29.0.0",
    "ts-node": "^10.9.0",
    "typescript": "^5.0.0"
  },
  "engines": {
    "node": ">=18.0.0"
  },
  "files": [
    "dist/",
    "bin/",
    "README.md",
    "LICENSE"
  ]
}
