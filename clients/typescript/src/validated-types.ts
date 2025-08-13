/**
 * Validated types for KotaDB TypeScript client.
 *
 * These types mirror the Rust validated types and provide compile-time
 * safety guarantees by ensuring they cannot be constructed with invalid data.
 */

import {
  validateFilePath,
  validateDirectoryPath,
  validateDocumentId,
  validateTitle,
  validateTimestamp,
  validateSize,
  ValidationError,
} from './validation';

/**
 * A path that has been validated and is guaranteed to be safe.
 *
 * Invariants:
 * - Path is non-empty
 * - No directory traversal (..)
 * - No null bytes
 * - Valid UTF-8
 * - Not a reserved name (Windows compatibility)
 */
export class ValidatedPath {
  private readonly _path: string;

  /**
   * Create a new validated path.
   *
   * @param path Path to validate
   * @throws ValidationError If path is invalid
   */
  constructor(path: string) {
    validateFilePath(path);
    this._path = path;
  }

  /**
   * Alternative constructor for consistency with Rust API.
   *
   * @param path Path to validate
   * @returns ValidatedPath instance
   * @throws ValidationError If path is invalid
   */
  static new(path: string): ValidatedPath {
    return new ValidatedPath(path);
  }

  /**
   * Get the path as a string.
   */
  asStr(): string {
    return this._path;
  }

  toString(): string {
    return this._path;
  }

  valueOf(): string {
    return this._path;
  }

  equals(other: ValidatedPath | string): boolean {
    if (other instanceof ValidatedPath) {
      return this._path === other._path;
    }
    return this._path === other;
  }
}

/**
 * A directory path that has been validated.
 *
 * Additional invariants:
 * - Should not have file extension
 */
export class ValidatedDirectoryPath extends ValidatedPath {
  /**
   * Create a new validated directory path.
   *
   * @param path Directory path to validate
   * @throws ValidationError If path is invalid
   */
  constructor(path: string) {
    validateDirectoryPath(path);
    super(path);
  }
}

/**
 * A document ID that is guaranteed to be valid.
 *
 * Invariants:
 * - Valid UUID format
 * - Not nil UUID
 */
export class ValidatedDocumentId {
  private readonly _id: string;

  /**
   * Create a new validated document ID.
   *
   * @param docId Document ID as string
   * @throws ValidationError If ID is invalid
   */
  constructor(docId: string) {
    validateDocumentId(docId);
    this._id = docId;
  }

  /**
   * Create a new random document ID.
   *
   * @returns ValidatedDocumentId with random UUID
   */
  static new(): ValidatedDocumentId {
    return new ValidatedDocumentId(generateUuid());
  }

  /**
   * Parse from string.
   *
   * @param s String representation of UUID
   * @returns ValidatedDocumentId instance
   * @throws ValidationError If string is invalid UUID
   */
  static parse(s: string): ValidatedDocumentId {
    return new ValidatedDocumentId(s);
  }

  /**
   * Get the ID as a string.
   */
  asStr(): string {
    return this._id;
  }

  toString(): string {
    return this._id;
  }

  valueOf(): string {
    return this._id;
  }

  equals(other: ValidatedDocumentId | string): boolean {
    if (other instanceof ValidatedDocumentId) {
      return this._id === other._id;
    }
    return this._id === other;
  }
}

/**
 * A non-empty title with enforced length limits.
 *
 * Invariants:
 * - Non-empty after trimming
 * - Length <= 1024 characters
 */
export class ValidatedTitle {
  static readonly MAX_LENGTH = 1024;
  private readonly _title: string;

  /**
   * Create a new validated title.
   *
   * @param title Title to validate
   * @throws ValidationError If title is invalid
   */
  constructor(title: string) {
    validateTitle(title);
    this._title = title.trim();
  }

  /**
   * Alternative constructor for consistency with Rust API.
   *
   * @param title Title to validate
   * @returns ValidatedTitle instance
   * @throws ValidationError If title is invalid
   */
  static new(title: string): ValidatedTitle {
    return new ValidatedTitle(title);
  }

  /**
   * Get the title as a string.
   */
  asStr(): string {
    return this._title;
  }

  toString(): string {
    return this._title;
  }

  valueOf(): string {
    return this._title;
  }

  equals(other: ValidatedTitle | string): boolean {
    if (other instanceof ValidatedTitle) {
      return this._title === other._title;
    }
    return this._title === other;
  }
}

/**
 * A non-zero size value.
 *
 * Invariants:
 * - Must be greater than zero
 */
export class NonZeroSize {
  private readonly _size: number;

  /**
   * Create a new non-zero size.
   *
   * @param size Size value
   * @throws ValidationError If size is invalid
   */
  constructor(size: number) {
    validateSize(size);
    this._size = size;
  }

  /**
   * Alternative constructor for consistency with Rust API.
   *
   * @param size Size value
   * @returns NonZeroSize instance
   * @throws ValidationError If size is invalid
   */
  static new(size: number): NonZeroSize {
    return new NonZeroSize(size);
  }

  /**
   * Get the size value.
   */
  get(): number {
    return this._size;
  }

  valueOf(): number {
    return this._size;
  }

  toString(): string {
    return this._size.toString();
  }

  equals(other: NonZeroSize | number): boolean {
    if (other instanceof NonZeroSize) {
      return this._size === other._size;
    }
    return this._size === other;
  }
}

/**
 * A timestamp with validation.
 *
 * Invariants:
 * - Must be positive (after Unix epoch)
 * - Must be reasonable (not in far future)
 */
export class ValidatedTimestamp {
  private readonly _timestamp: number;

  /**
   * Create a new validated timestamp.
   *
   * @param timestamp Unix timestamp
   * @throws ValidationError If timestamp is invalid
   */
  constructor(timestamp: number) {
    validateTimestamp(timestamp);
    this._timestamp = timestamp;
  }

  /**
   * Alternative constructor for consistency with Rust API.
   *
   * @param timestamp Unix timestamp
   * @returns ValidatedTimestamp instance
   * @throws ValidationError If timestamp is invalid
   */
  static new(timestamp: number): ValidatedTimestamp {
    return new ValidatedTimestamp(timestamp);
  }

  /**
   * Create a timestamp for the current time.
   *
   * @returns ValidatedTimestamp with current time
   */
  static now(): ValidatedTimestamp {
    return new ValidatedTimestamp(Math.floor(Date.now() / 1000));
  }

  /**
   * Get the timestamp in seconds.
   */
  asSecs(): number {
    return this._timestamp;
  }

  valueOf(): number {
    return this._timestamp;
  }

  toString(): string {
    return this._timestamp.toString();
  }

  equals(other: ValidatedTimestamp | number): boolean {
    if (other instanceof ValidatedTimestamp) {
      return this._timestamp === other._timestamp;
    }
    return this._timestamp === other;
  }
}

/**
 * Generate a random UUID v4.
 *
 * @returns A random UUID string
 */
function generateUuid(): string {
  // Use crypto.randomUUID if available (Node.js 14.17+ and modern browsers)
  if (typeof crypto !== 'undefined' && crypto.randomUUID) {
    return crypto.randomUUID();
  }

  // Fallback implementation
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function (c) {
    const r = Math.random() * 16 | 0;
    const v = c === 'x' ? r : (r & 0x3 | 0x8);
    return v.toString(16);
  });
}