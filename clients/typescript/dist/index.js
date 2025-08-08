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
exports.connect = exports.KotaDB = void 0;
var client_1 = require("./client");
Object.defineProperty(exports, "KotaDB", { enumerable: true, get: function () { return client_1.KotaDB; } });
Object.defineProperty(exports, "connect", { enumerable: true, get: function () { return client_1.connect; } });
__exportStar(require("./types"), exports);
// Default export for convenience
const client_2 = require("./client");
exports.default = client_2.KotaDB;
//# sourceMappingURL=index.js.map