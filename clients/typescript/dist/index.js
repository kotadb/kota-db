"use strict";
/**
 * KotaDB TypeScript/JavaScript Client
 *
 * A simple HTTP client for KotaDB that provides PostgreSQL-level ease of use.
 *
 * @example
 * ```typescript
 * import { KotaDB } from 'kotadb-client';
 *
 * const db = new KotaDB({ url: 'http://localhost:8080' });
 * const results = await db.query('rust patterns');
 * const docId = await db.insert({
 *   path: '/notes/meeting.md',
 *   title: 'My Note',
 *   content: '...',
 *   tags: ['work']
 * });
 * ```
 */
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __exportStar = (this && this.__exportStar) || function(m, exports) {
    for (var p in m) if (p !== "default" && !Object.prototype.hasOwnProperty.call(exports, p)) __createBinding(exports, m, p);
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.downloadBinary = exports.ensureBinaryInstalled = exports.startServer = exports.KotaDBServer = exports.validateSize = exports.validateTimestamp = exports.validateSearchQuery = exports.validateTag = exports.validateTitle = exports.validateDocumentId = exports.validateDirectoryPath = exports.validateFilePath = exports.connect = exports.KotaDB = void 0;
var client_1 = require("./client");
Object.defineProperty(exports, "KotaDB", { enumerable: true, get: function () { return client_1.KotaDB; } });
Object.defineProperty(exports, "connect", { enumerable: true, get: function () { return client_1.connect; } });
__exportStar(require("./types"), exports);
__exportStar(require("./validated-types"), exports);
var validation_1 = require("./validation");
Object.defineProperty(exports, "validateFilePath", { enumerable: true, get: function () { return validation_1.validateFilePath; } });
Object.defineProperty(exports, "validateDirectoryPath", { enumerable: true, get: function () { return validation_1.validateDirectoryPath; } });
Object.defineProperty(exports, "validateDocumentId", { enumerable: true, get: function () { return validation_1.validateDocumentId; } });
Object.defineProperty(exports, "validateTitle", { enumerable: true, get: function () { return validation_1.validateTitle; } });
Object.defineProperty(exports, "validateTag", { enumerable: true, get: function () { return validation_1.validateTag; } });
Object.defineProperty(exports, "validateSearchQuery", { enumerable: true, get: function () { return validation_1.validateSearchQuery; } });
Object.defineProperty(exports, "validateTimestamp", { enumerable: true, get: function () { return validation_1.validateTimestamp; } });
Object.defineProperty(exports, "validateSize", { enumerable: true, get: function () { return validation_1.validateSize; } });
__exportStar(require("./builders"), exports);
var server_1 = require("./server");
Object.defineProperty(exports, "KotaDBServer", { enumerable: true, get: function () { return server_1.KotaDBServer; } });
Object.defineProperty(exports, "startServer", { enumerable: true, get: function () { return server_1.startServer; } });
Object.defineProperty(exports, "ensureBinaryInstalled", { enumerable: true, get: function () { return server_1.ensureBinaryInstalled; } });
Object.defineProperty(exports, "downloadBinary", { enumerable: true, get: function () { return server_1.downloadBinary; } });
// Default export for convenience
const client_2 = require("./client");
exports.default = client_2.KotaDB;
//# sourceMappingURL=index.js.map