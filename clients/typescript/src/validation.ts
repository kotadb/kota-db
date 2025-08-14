/**
 * KotaDB validation module for TypeScript client.
 *
 * Mirrors the Rust validation patterns to provide consistent validation
 * across all client libraries.
 */

import { ValidationError } from './types';

export { ValidationError };

// Path validation constants
const MAX_PATH_LENGTH = 4096;
const RESERVED_NAMES = new Set([
  'CON', 'PRN', 'AUX', 'NUL', 'COM1', 'COM2', 'COM3', 'COM4', 'COM5',
  'COM6', 'COM7', 'COM8', 'COM9', 'LPT1', 'LPT2', 'LPT3', 'LPT4',
  'LPT5', 'LPT6', 'LPT7', 'LPT8', 'LPT9'
]);

/**
 * Validate a file path for storage.
 *
 * Ensures path is safe and follows platform conventions.
 *
 * @param path Path to validate
 * @throws ValidationError If path is invalid
 */
export function validateFilePath(path: string): void {
  if (!path || !path.trim()) {
    throw new ValidationError('Path cannot be empty');
  }

  if (path.length >= MAX_PATH_LENGTH) {
    throw new ValidationError(`Path exceeds maximum length of ${MAX_PATH_LENGTH}`);
  }

  if (path.includes('\0')) {
    throw new ValidationError('Path contains null bytes');
  }

  // Decode URL encoding for thorough check
  let decodedPath: string;
  try {
    decodedPath = decodeURIComponent(path);
  } catch {
    decodedPath = path; // If decode fails, use original
  }

  // Check for directory traversal attempts
  if (path.includes('..') || decodedPath.includes('..')) {
    throw new ValidationError('Parent directory references (..) not allowed');
  }

  // Also check for various encoded forms
  if (path.includes('%2e%2e') || path.includes('%252e%252e') || path.includes('..;')) {
    throw new ValidationError('Parent directory references (..) not allowed');
  }

  // Check path parts for traversal
  const pathParts = decodedPath.replace(/\\/g, '/').split('/');
  if (pathParts.includes('..')) {
    throw new ValidationError('Parent directory references (..) not allowed');
  }

  // Check for reserved names (Windows compatibility)
  const filename = path.split(/[/\\]/).pop();
  if (filename) {
    const stem = filename.split('.')[0]?.toUpperCase();
    if (stem && RESERVED_NAMES.has(stem)) {
      throw new ValidationError(`Reserved filename: ${filename}`);
    }
  }

  // Validate UTF-8 encoding (TypeScript strings are already UTF-16, but check for surrogates)
  try {
    new TextEncoder().encode(path);
  } catch (error) {
    throw new ValidationError('Path is not valid UTF-8');
  }
}

/**
 * Validate a directory path.
 *
 * @param path Directory path to validate
 * @throws ValidationError If path is invalid
 */
export function validateDirectoryPath(path: string): void {
  validateFilePath(path);

  // Ensure it's not a file with extension
  const basename = path.split(/[/\\]/).pop() || '';
  if (basename.includes('.')) {
    throw new ValidationError('Directory path should not have file extension');
  }
}

/**
 * Validate a document ID.
 *
 * @param docId Document ID to validate
 * @throws ValidationError If ID is invalid
 */
export function validateDocumentId(docId: string): void {
  if (!docId || !docId.trim()) {
    throw new ValidationError('Document ID cannot be empty');
  }

  // Check for nil UUID first
  if (docId === '00000000-0000-0000-0000-000000000000') {
    throw new ValidationError('Document ID cannot be nil UUID');
  }

  // Check UUID format using regex (more reliable than trying to parse)
  const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;
  if (!uuidRegex.test(docId)) {
    throw new ValidationError('Invalid UUID format');
  }
}

/**
 * Validate a document title.
 *
 * @param title Title to validate
 * @throws ValidationError If title is invalid
 */
export function validateTitle(title: string): void {
  if (!title || !title.trim()) {
    throw new ValidationError('Title cannot be empty');
  }

  if (title.trim().length > 1024) {
    throw new ValidationError('Title exceeds maximum length of 1024 characters');
  }
}

/**
 * Validate a tag.
 *
 * @param tag Tag to validate
 * @throws ValidationError If tag is invalid
 */
export function validateTag(tag: string): void {
  if (!tag || !tag.trim()) {
    throw new ValidationError('Tag cannot be empty');
  }

  if (tag.length > 128) {
    throw new ValidationError('Tag too long (max 128 chars)');
  }

  // Check for valid characters (alphanumeric, dash, underscore, space)
  if (!/^[a-zA-Z0-9\-_ ]+$/.test(tag)) {
    throw new ValidationError('Tag contains invalid characters');
  }
}

/**
 * Validate a search query.
 *
 * @param query Search query to validate
 * @throws ValidationError If query is invalid
 */
export function validateSearchQuery(query: string): void {
  if (!query || !query.trim()) {
    throw new ValidationError('Search query cannot be empty');
  }

  if (query.length > 1024) {
    throw new ValidationError('Search query too long (max 1024 chars)');
  }
}

/**
 * Validate a timestamp.
 *
 * @param timestamp Unix timestamp to validate
 * @throws ValidationError If timestamp is invalid
 */
export function validateTimestamp(timestamp: number): void {
  if (timestamp <= 0) {
    throw new ValidationError('Timestamp must be positive');
  }

  // Check not too far in future (year 3000)
  const YEAR_3000 = 32503680000;
  if (timestamp >= YEAR_3000) {
    throw new ValidationError('Timestamp too far in future');
  }
}

/**
 * Validate a size value.
 *
 * @param size Size to validate
 * @throws ValidationError If size is invalid
 */
export function validateSize(size: number): void {
  if (size <= 0) {
    throw new ValidationError('Size must be greater than zero');
  }

  // Check for reasonable maximum (100MB)
  if (size > 100 * 1024 * 1024) {
    throw new ValidationError('Size exceeds maximum (100MB)');
  }
}