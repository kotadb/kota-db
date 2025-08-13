/**
 * Validated Types Tests for KotaDB TypeScript Client
 * 
 * Comprehensive tests for validated types that ensure type safety
 * and security validation across all client libraries.
 */

import {
  ValidatedPath,
  ValidatedDirectoryPath,
  ValidatedDocumentId,
  ValidatedTitle,
  NonZeroSize,
  ValidatedTimestamp,
} from '../src/validated-types';
import { ValidationError } from '../src/validation';

describe('ValidatedPath', () => {
  test('should create valid paths', () => {
    const validPaths = [
      '/test.md',
      '/notes/meeting.md',
      '/deep/nested/path/document.txt',
      'relative/path.md',
      'simple.txt'
    ];

    for (const path of validPaths) {
      const validatedPath = new ValidatedPath(path);
      expect(validatedPath.asStr()).toBe(path);
      expect(validatedPath.toString()).toBe(path);
      expect(validatedPath.valueOf()).toBe(path);
    }
  });

  test('should reject empty paths', () => {
    expect(() => new ValidatedPath('')).toThrow(ValidationError);
    expect(() => new ValidatedPath('   ')).toThrow(ValidationError);
  });

  test('should reject directory traversal attempts', () => {
    const maliciousPaths = [
      '../etc/passwd',
      '/legitimate/../../../etc/passwd',
      'good/path/../../bad/path',
      '..\\windows\\system32',
      '/app/../../../root/.ssh/id_rsa'
    ];

    for (const path of maliciousPaths) {
      expect(() => new ValidatedPath(path)).toThrow(ValidationError);
    }
  });

  test('should reject null bytes', () => {
    const nullBytePaths = [
      '/test\x00.md',
      '/legitimate/path\x00/../etc/passwd',
      '\x00malicious.txt'
    ];

    for (const path of nullBytePaths) {
      expect(() => new ValidatedPath(path)).toThrow(ValidationError);
    }
  });

  test('should reject reserved Windows names', () => {
    const reservedNames = [
      'CON.txt',
      'PRN.md',
      'AUX',
      'NUL.log',
      'COM1.txt',
      'LPT1.md',
      '/path/to/CON.txt',
      '/folder/PRN'
    ];

    for (const path of reservedNames) {
      expect(() => new ValidatedPath(path)).toThrow(ValidationError);
    }
  });

  test('should reject paths that are too long', () => {
    const longPath = 'a'.repeat(5000);
    expect(() => new ValidatedPath(longPath)).toThrow(ValidationError);
  });

  test('should support static new() constructor', () => {
    const path = ValidatedPath.new('/test.md');
    expect(path.asStr()).toBe('/test.md');
  });

  test('should support equality comparisons', () => {
    const path1 = new ValidatedPath('/test.md');
    const path2 = new ValidatedPath('/test.md');
    const path3 = new ValidatedPath('/different.md');

    expect(path1.equals(path2)).toBe(true);
    expect(path1.equals('/test.md')).toBe(true);
    expect(path1.equals(path3)).toBe(false);
    expect(path1.equals('/different.md')).toBe(false);
  });
});

describe('ValidatedDirectoryPath', () => {
  test('should create valid directory paths', () => {
    const validDirPaths = [
      '/notes',
      '/deep/nested/directory',
      'relative/directory',
      'simple'
    ];

    for (const path of validDirPaths) {
      const validatedPath = new ValidatedDirectoryPath(path);
      expect(validatedPath.asStr()).toBe(path);
    }
  });

  test('should reject paths with file extensions', () => {
    const pathsWithExtensions = [
      '/notes.md',
      '/directory/file.txt',
      'folder.pdf'
    ];

    for (const path of pathsWithExtensions) {
      expect(() => new ValidatedDirectoryPath(path)).toThrow(ValidationError);
    }
  });

  test('should inherit all ValidatedPath validations', () => {
    expect(() => new ValidatedDirectoryPath('../etc')).toThrow(ValidationError);
    expect(() => new ValidatedDirectoryPath('/path\x00')).toThrow(ValidationError);
    expect(() => new ValidatedDirectoryPath('CON')).toThrow(ValidationError);
  });
});

describe('ValidatedDocumentId', () => {
  test('should create valid document IDs', () => {
    const validIds = [
      '123e4567-e89b-12d3-a456-426614174000',
      '550e8400-e29b-41d4-a716-446655440000',
      'f47ac10b-58cc-4372-a567-0e02b2c3d479'
    ];

    for (const id of validIds) {
      const validatedId = new ValidatedDocumentId(id);
      expect(validatedId.asStr()).toBe(id);
      expect(validatedId.toString()).toBe(id);
      expect(validatedId.valueOf()).toBe(id);
    }
  });

  test('should reject invalid UUID formats', () => {
    const invalidIds = [
      'not-a-uuid',
      '123e4567-e89b-12d3-a456',  // Too short
      '123e4567-e89b-12d3-a456-426614174000-extra',  // Too long
      '123e4567-e89b-12d3-g456-426614174000',  // Invalid character
      '123e4567e89b12d3a456426614174000',  // Missing dashes
      ''
    ];

    for (const id of invalidIds) {
      expect(() => new ValidatedDocumentId(id)).toThrow(ValidationError);
    }
  });

  test('should reject nil UUID', () => {
    expect(() => new ValidatedDocumentId('00000000-0000-0000-0000-000000000000'))
      .toThrow(ValidationError);
  });

  test('should generate new random UUIDs', () => {
    const id1 = ValidatedDocumentId.new();
    const id2 = ValidatedDocumentId.new();
    
    expect(id1.asStr()).not.toBe(id2.asStr());
    expect(id1.asStr()).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i);
    expect(id2.asStr()).toMatch(/^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i);
  });

  test('should support parse() constructor', () => {
    const id = ValidatedDocumentId.parse('123e4567-e89b-12d3-a456-426614174000');
    expect(id.asStr()).toBe('123e4567-e89b-12d3-a456-426614174000');
  });

  test('should support equality comparisons', () => {
    const id1 = new ValidatedDocumentId('123e4567-e89b-12d3-a456-426614174000');
    const id2 = new ValidatedDocumentId('123e4567-e89b-12d3-a456-426614174000');
    const id3 = new ValidatedDocumentId('550e8400-e29b-41d4-a716-446655440000');

    expect(id1.equals(id2)).toBe(true);
    expect(id1.equals('123e4567-e89b-12d3-a456-426614174000')).toBe(true);
    expect(id1.equals(id3)).toBe(false);
    expect(id1.equals('550e8400-e29b-41d4-a716-446655440000')).toBe(false);
  });
});

describe('ValidatedTitle', () => {
  test('should create valid titles', () => {
    const validTitles = [
      'Simple Title',
      'Title with Numbers 123',
      'Title with Special Characters!@#$%^&*()',
      'Unicode Title with Émojis 🚀',
      'A'.repeat(1024)  // Max length
    ];

    for (const title of validTitles) {
      const validatedTitle = new ValidatedTitle(title);
      expect(validatedTitle.asStr()).toBe(title.trim());
      expect(validatedTitle.toString()).toBe(title.trim());
    }
  });

  test('should reject empty titles', () => {
    const emptyTitles = [
      '',
      '   ',
      '\t\n\r'
    ];

    for (const title of emptyTitles) {
      expect(() => new ValidatedTitle(title)).toThrow(ValidationError);
    }
  });

  test('should reject titles that are too long', () => {
    const longTitle = 'A'.repeat(1025);
    expect(() => new ValidatedTitle(longTitle)).toThrow(ValidationError);
  });

  test('should trim whitespace from titles', () => {
    const title = new ValidatedTitle('  Title with spaces  ');
    expect(title.asStr()).toBe('Title with spaces');
  });

  test('should support static new() constructor', () => {
    const title = ValidatedTitle.new('Test Title');
    expect(title.asStr()).toBe('Test Title');
  });

  test('should support equality comparisons', () => {
    const title1 = new ValidatedTitle('Test Title');
    const title2 = new ValidatedTitle('Test Title');
    const title3 = new ValidatedTitle('Different Title');

    expect(title1.equals(title2)).toBe(true);
    expect(title1.equals('Test Title')).toBe(true);
    expect(title1.equals(title3)).toBe(false);
    expect(title1.equals('Different Title')).toBe(false);
  });

  test('should have MAX_LENGTH constant', () => {
    expect(ValidatedTitle.MAX_LENGTH).toBe(1024);
  });
});

describe('NonZeroSize', () => {
  test('should create valid sizes', () => {
    const validSizes = [1, 100, 1024, 1024 * 1024];

    for (const size of validSizes) {
      const validatedSize = new NonZeroSize(size);
      expect(validatedSize.get()).toBe(size);
      expect(validatedSize.valueOf()).toBe(size);
      expect(validatedSize.toString()).toBe(size.toString());
    }
  });

  test('should reject zero and negative sizes', () => {
    const invalidSizes = [0, -1, -100];

    for (const size of invalidSizes) {
      expect(() => new NonZeroSize(size)).toThrow(ValidationError);
    }
  });

  test('should reject sizes that are too large', () => {
    const tooLarge = 100 * 1024 * 1024 + 1; // 100MB + 1 byte
    expect(() => new NonZeroSize(tooLarge)).toThrow(ValidationError);
  });

  test('should support static new() constructor', () => {
    const size = NonZeroSize.new(1024);
    expect(size.get()).toBe(1024);
  });

  test('should support equality comparisons', () => {
    const size1 = new NonZeroSize(1024);
    const size2 = new NonZeroSize(1024);
    const size3 = new NonZeroSize(2048);

    expect(size1.equals(size2)).toBe(true);
    expect(size1.equals(1024)).toBe(true);
    expect(size1.equals(size3)).toBe(false);
    expect(size1.equals(2048)).toBe(false);
  });
});

describe('ValidatedTimestamp', () => {
  test('should create valid timestamps', () => {
    const now = Math.floor(Date.now() / 1000);
    const validTimestamps = [1, 1000000000, now, 2000000000]; // Various valid timestamps

    for (const timestamp of validTimestamps) {
      const validatedTimestamp = new ValidatedTimestamp(timestamp);
      expect(validatedTimestamp.asSecs()).toBe(timestamp);
      expect(validatedTimestamp.valueOf()).toBe(timestamp);
      expect(validatedTimestamp.toString()).toBe(timestamp.toString());
    }
  });

  test('should reject zero and negative timestamps', () => {
    const invalidTimestamps = [0, -1, -1000000000];

    for (const timestamp of invalidTimestamps) {
      expect(() => new ValidatedTimestamp(timestamp)).toThrow(ValidationError);
    }
  });

  test('should reject timestamps too far in the future', () => {
    const year3000 = 32503680000;
    expect(() => new ValidatedTimestamp(year3000)).toThrow(ValidationError);
    expect(() => new ValidatedTimestamp(year3000 + 1)).toThrow(ValidationError);
  });

  test('should create timestamp for current time', () => {
    const before = Math.floor(Date.now() / 1000);
    const timestamp = ValidatedTimestamp.now();
    const after = Math.floor(Date.now() / 1000);

    expect(timestamp.asSecs()).toBeGreaterThanOrEqual(before);
    expect(timestamp.asSecs()).toBeLessThanOrEqual(after);
  });

  test('should support static new() constructor', () => {
    const timestamp = ValidatedTimestamp.new(1609459200); // 2021-01-01
    expect(timestamp.asSecs()).toBe(1609459200);
  });

  test('should support equality comparisons', () => {
    const timestamp1 = new ValidatedTimestamp(1609459200);
    const timestamp2 = new ValidatedTimestamp(1609459200);
    const timestamp3 = new ValidatedTimestamp(1640995200);

    expect(timestamp1.equals(timestamp2)).toBe(true);
    expect(timestamp1.equals(1609459200)).toBe(true);
    expect(timestamp1.equals(timestamp3)).toBe(false);
    expect(timestamp1.equals(1640995200)).toBe(false);
  });
});

describe('Security Validation Edge Cases', () => {
  test('should handle various directory traversal techniques', () => {
    const traversalAttempts = [
      '../',
      '..\\',
      '..;',
      '..//..//',
      '..\\..\\',
      './../',
      '.\\..\\.\\',
      '%2e%2e%2f',
      '%2e%2e\\',
      '%252e%252e%252f',
      'file:///../../../etc/passwd',
      '/var/www/../../etc/passwd',
      '/app/public/../../../root/.ssh/'
    ];

    for (const attempt of traversalAttempts) {
      expect(() => new ValidatedPath(attempt)).toThrow(ValidationError);
    }
  });

  test('should handle null byte injection variations', () => {
    const nullByteAttempts = [
      '/legitimate/file.txt\x00.jpg',
      '/file\x00/../../etc/passwd',
      'normal.txt\x00<script>alert(1)</script>',
      '\x00/etc/passwd',
      '/etc/\x00passwd'
    ];

    for (const attempt of nullByteAttempts) {
      expect(() => new ValidatedPath(attempt)).toThrow(ValidationError);
    }
  });

  test('should handle all Windows reserved names case variations', () => {
    const allReservedNames = [
      'CON', 'con', 'Con', 'coN',
      'PRN', 'prn', 'Prn', 
      'AUX', 'aux', 'Aux',
      'NUL', 'nul', 'Nul',
      'COM1', 'com1', 'Com1',
      'COM9', 'com9', 'Com9',
      'LPT1', 'lpt1', 'Lpt1',
      'LPT9', 'lpt9', 'Lpt9'
    ];

    for (const name of allReservedNames) {
      expect(() => new ValidatedPath(`/path/${name}.txt`)).toThrow(ValidationError);
      expect(() => new ValidatedPath(`${name}`)).toThrow(ValidationError);
    }
  });

  test('should handle malformed UUIDs with various formats', () => {
    const malformedUuids = [
      '123e4567-e89b-12d3-a456-42661417400g', // Invalid hex character
      '123e4567-e89b-12d3-a456-4266141740000', // Too many characters
      '123e4567-e89b-12d3-a456-42661417400', // Too few characters
      '123e4567e89b12d3a456426614174000', // Missing separators
      '123e4567-e89b-12d3-a456_426614174000', // Wrong separator
      'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx', // Invalid format string
      'G23e4567-e89b-12d3-a456-426614174000', // Invalid first character
      '', // Empty string
      '   ', // Whitespace only
      'not-a-uuid-at-all'
    ];

    for (const uuid of malformedUuids) {
      expect(() => new ValidatedDocumentId(uuid)).toThrow(ValidationError);
    }
  });
});