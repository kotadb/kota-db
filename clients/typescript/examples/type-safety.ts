/**
 * KotaDB TypeScript Client - Type Safety Example
 * 
 * Demonstrates the security and type safety features of the validated types.
 */

import { 
  ValidatedPath,
  ValidatedDocumentId,
  ValidatedTitle,
  NonZeroSize,
  ValidatedTimestamp,
  ValidationError,
  validateFilePath,
  validateDocumentId,
  validateTitle,
  validateTag
} from '../src/index';

function typeSafetyExample() {
  console.log('🛡️  KotaDB TypeScript Client - Type Safety Demo\n');

  // 1. Path Validation
  console.log('📁 Path Validation Examples');
  
  // Valid paths
  const validPaths = [
    '/documents/report.md',
    '/notes/meeting-2023.txt',
    'relative/path/file.pdf'
  ];

  for (const path of validPaths) {
    try {
      const validatedPath = new ValidatedPath(path);
      console.log(`✅ Valid path: ${validatedPath.asStr()}`);
    } catch (error) {
      console.log(`❌ Unexpected error: ${error}`);
    }
  }

  // Security threats that are blocked
  const maliciousPaths = [
    '../../../etc/passwd',           // Directory traversal
    '/file\x00injection.txt',        // Null byte injection
    'CON.txt',                       // Windows reserved name
    '/path/../../../root/.ssh/',     // Complex traversal
    '\x00malicious.exe'              // Null byte at start
  ];

  console.log('\n🚨 Security Threats (Blocked)');
  for (const path of maliciousPaths) {
    try {
      new ValidatedPath(path);
      console.log(`❌ SECURITY ISSUE: Path should have been blocked: ${path}`);
    } catch (error) {
      if (error instanceof ValidationError) {
        console.log(`✅ Blocked malicious path: "${path}" - ${error.message}`);
      }
    }
  }

  // 2. Document ID Validation
  console.log('\n🆔 Document ID Validation Examples');

  // Generate new IDs
  const newId1 = ValidatedDocumentId.new();
  const newId2 = ValidatedDocumentId.new();
  console.log(`✅ Generated ID 1: ${newId1.asStr()}`);
  console.log(`✅ Generated ID 2: ${newId2.asStr()}`);
  console.log(`🔄 IDs are unique: ${!newId1.equals(newId2)}`);

  // Parse existing valid IDs
  const validUuids = [
    '123e4567-e89b-12d3-a456-426614174000',
    'f47ac10b-58cc-4372-a567-0e02b2c3d479'
  ];

  for (const uuid of validUuids) {
    try {
      const parsedId = ValidatedDocumentId.parse(uuid);
      console.log(`✅ Valid UUID: ${parsedId.asStr()}`);
    } catch (error) {
      console.log(`❌ Unexpected error: ${error}`);
    }
  }

  // Invalid IDs that are blocked
  const invalidUuids = [
    'not-a-uuid',                                    // Not UUID format
    '123e4567-e89b-12d3-a456',                      // Too short
    '123e4567-e89b-12d3-g456-426614174000',         // Invalid hex
    '00000000-0000-0000-0000-000000000000',         // Nil UUID
    'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'          // Invalid format
  ];

  console.log('\n🚫 Invalid UUIDs (Blocked)');
  for (const uuid of invalidUuids) {
    try {
      ValidatedDocumentId.parse(uuid);
      console.log(`❌ VALIDATION ISSUE: UUID should have been blocked: ${uuid}`);
    } catch (error) {
      if (error instanceof ValidationError) {
        console.log(`✅ Blocked invalid UUID: "${uuid}" - ${error.message}`);
      }
    }
  }

  // 3. Title Validation
  console.log('\n📝 Title Validation Examples');

  // Valid titles
  const validTitles = [
    'Simple Title',
    'Title with Numbers 123',
    'Unicode Title with Émojis 🚀',
    'Special Characters: !@#$%^&*()'
  ];

  for (const title of validTitles) {
    try {
      const validatedTitle = new ValidatedTitle(title);
      console.log(`✅ Valid title: "${validatedTitle.asStr()}"`);
    } catch (error) {
      console.log(`❌ Unexpected error: ${error}`);
    }
  }

  // Invalid titles
  const invalidTitles = [
    '',                          // Empty
    '   ',                       // Whitespace only
    'A'.repeat(1025)             // Too long
  ];

  console.log('\n🚫 Invalid Titles (Blocked)');
  for (const title of invalidTitles) {
    try {
      new ValidatedTitle(title);
      console.log(`❌ VALIDATION ISSUE: Title should have been blocked`);
    } catch (error) {
      if (error instanceof ValidationError) {
        console.log(`✅ Blocked invalid title - ${error.message}`);
      }
    }
  }

  // 4. Size Validation
  console.log('\n📏 Size Validation Examples');

  // Valid sizes
  const validSizes = [1, 1024, 1024 * 1024, 50 * 1024 * 1024]; // 1B to 50MB

  for (const size of validSizes) {
    try {
      const validatedSize = new NonZeroSize(size);
      const sizeStr = formatBytes(validatedSize.get());
      console.log(`✅ Valid size: ${sizeStr}`);
    } catch (error) {
      console.log(`❌ Unexpected error: ${error}`);
    }
  }

  // Invalid sizes
  const invalidSizes = [0, -1, 101 * 1024 * 1024]; // 0, negative, too large

  console.log('\n🚫 Invalid Sizes (Blocked)');
  for (const size of invalidSizes) {
    try {
      new NonZeroSize(size);
      console.log(`❌ VALIDATION ISSUE: Size should have been blocked: ${size}`);
    } catch (error) {
      if (error instanceof ValidationError) {
        console.log(`✅ Blocked invalid size: ${formatBytes(size)} - ${error.message}`);
      }
    }
  }

  // 5. Timestamp Validation
  console.log('\n⏰ Timestamp Validation Examples');

  // Current timestamp
  const now = ValidatedTimestamp.now();
  console.log(`✅ Current timestamp: ${now.asSecs()} (${new Date(now.asSecs() * 1000).toISOString()})`);

  // Valid historical timestamps
  const validTimestamps = [
    1609459200,  // 2021-01-01
    1640995200,  // 2022-01-01
    Math.floor(Date.now() / 1000)  // Now
  ];

  for (const timestamp of validTimestamps) {
    try {
      const validatedTimestamp = new ValidatedTimestamp(timestamp);
      const date = new Date(validatedTimestamp.asSecs() * 1000);
      console.log(`✅ Valid timestamp: ${timestamp} (${date.toISOString()})`);
    } catch (error) {
      console.log(`❌ Unexpected error: ${error}`);
    }
  }

  // Invalid timestamps
  const invalidTimestamps = [
    0,              // Zero
    -1,             // Negative
    32503680000     // Year 3000 (too far future)
  ];

  console.log('\n🚫 Invalid Timestamps (Blocked)');
  for (const timestamp of invalidTimestamps) {
    try {
      new ValidatedTimestamp(timestamp);
      console.log(`❌ VALIDATION ISSUE: Timestamp should have been blocked: ${timestamp}`);
    } catch (error) {
      if (error instanceof ValidationError) {
        console.log(`✅ Blocked invalid timestamp: ${timestamp} - ${error.message}`);
      }
    }
  }

  // 6. Tag Validation
  console.log('\n🏷️  Tag Validation Examples');

  // Valid tags
  const validTags = [
    'simple',
    'tag-with-dashes',
    'tag_with_underscores',
    'tag with spaces',
    'Tag123'
  ];

  for (const tag of validTags) {
    try {
      validateTag(tag);
      console.log(`✅ Valid tag: "${tag}"`);
    } catch (error) {
      console.log(`❌ Unexpected error: ${error}`);
    }
  }

  // Invalid tags
  const invalidTags = [
    '',                    // Empty
    'tag@invalid',         // Invalid character
    'tag#invalid',         // Invalid character
    'A'.repeat(129)        // Too long
  ];

  console.log('\n🚫 Invalid Tags (Blocked)');
  for (const tag of invalidTags) {
    try {
      validateTag(tag);
      console.log(`❌ VALIDATION ISSUE: Tag should have been blocked: "${tag}"`);
    } catch (error) {
      if (error instanceof ValidationError) {
        console.log(`✅ Blocked invalid tag: "${tag}" - ${error.message}`);
      }
    }
  }

  // 7. Type Equality and Comparison
  console.log('\n🔗 Type Equality Examples');

  const path1 = new ValidatedPath('/same/path.md');
  const path2 = new ValidatedPath('/same/path.md');
  const path3 = new ValidatedPath('/different/path.md');

  console.log(`✅ Path equality (same): ${path1.equals(path2)}`);
  console.log(`✅ Path equality (different): ${path1.equals(path3)}`);
  console.log(`✅ Path string comparison: ${path1.equals('/same/path.md')}`);

  const title1 = new ValidatedTitle('Same Title');
  const title2 = new ValidatedTitle('Same Title');
  console.log(`✅ Title equality: ${title1.equals(title2)}`);

  // 8. Summary
  console.log('\n📊 Validation Summary');
  console.log('✅ All security validations working correctly');
  console.log('✅ Path traversal attacks blocked');
  console.log('✅ Null byte injection blocked');  
  console.log('✅ Reserved filenames blocked');
  console.log('✅ Invalid UUIDs blocked');
  console.log('✅ Input length limits enforced');
  console.log('✅ Type safety maintained throughout');
}

// Helper function to format bytes
function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 Bytes';
  
  const k = 1024;
  const sizes = ['Bytes', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(Math.abs(bytes)) / Math.log(k));
  
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

// Run the example
if (require.main === module) {
  try {
    typeSafetyExample();
    console.log('\n🎉 Type safety example completed successfully!');
  } catch (error) {
    console.error('\n💥 Example failed:', error);
    process.exit(1);
  }
}

export { typeSafetyExample };