"use strict";
/**
 * KotaDB TypeScript client types and interfaces.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.ServerError = exports.NotFoundError = exports.ValidationError = exports.ConnectionError = exports.KotaDBError = void 0;
// Error types
class KotaDBError extends Error {
    constructor(message, statusCode, responseBody) {
        super(message);
        this.statusCode = statusCode;
        this.responseBody = responseBody;
        this.name = 'KotaDBError';
    }
}
exports.KotaDBError = KotaDBError;
class ConnectionError extends KotaDBError {
    constructor(message) {
        super(message);
        this.name = 'ConnectionError';
    }
}
exports.ConnectionError = ConnectionError;
class ValidationError extends KotaDBError {
    constructor(message) {
        super(message);
        this.name = 'ValidationError';
    }
}
exports.ValidationError = ValidationError;
class NotFoundError extends KotaDBError {
    constructor(message = 'Resource not found') {
        super(message, 404);
        this.name = 'NotFoundError';
    }
}
exports.NotFoundError = NotFoundError;
class ServerError extends KotaDBError {
    constructor(message, statusCode, responseBody) {
        super(message, statusCode, responseBody);
        this.name = 'ServerError';
    }
}
exports.ServerError = ServerError;
//# sourceMappingURL=types.js.map