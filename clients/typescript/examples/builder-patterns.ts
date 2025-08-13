/**
 * KotaDB TypeScript Client - Builder Patterns Example
 * 
 * Demonstrates type-safe document and query construction using builder patterns.
 */

import { 
  KotaDB, 
  DocumentBuilder, 
  QueryBuilder, 
  UpdateBuilder,
  ValidatedPath,
  ValidatedTitle,
  ValidationError 
} from '../src/index';

async function builderPatternsExample() {
  // Connect to KotaDB
  const db = new KotaDB({ 
    url: process.env.KOTADB_URL || 'http://localhost:8080' 
  });

  try {
    // Test connection
    await db.testConnection();
    console.log('✅ Connected to KotaDB');
  } catch (error) {
    console.error('❌ Failed to connect to KotaDB:', error);
    return;
  }

  // 1. Document Builder Example
  console.log('\n📄 Document Builder Example');
  
  try {
    // Create a document using the builder pattern with validation
    const documentBuilder = new DocumentBuilder()
      .path('/examples/quarterly-report.md')
      .title('Q4 2023 Business Report')
      .content(`
# Quarterly Business Report - Q4 2023

## Executive Summary
This report covers the business performance for Q4 2023...

## Key Metrics
- Revenue: $2.4M
- Growth: 15% YoY
- Customer satisfaction: 4.8/5

## Next Steps
1. Expand into new markets
2. Increase R&D investment
3. Improve customer onboarding
      `.trim())
      .addTag('business')
      .addTag('quarterly')
      .addTag('2023')
      .addMetadata('author', 'jane.doe@company.com')
      .addMetadata('department', 'finance')
      .addMetadata('quarter', 'Q4')
      .addMetadata('year', 2023)
      .autoId(); // Generate secure UUID

    // Insert using the builder
    const docId = await db.insertWithBuilder(documentBuilder);
    console.log(`✅ Created document with ID: ${docId}`);

    // Retrieve and verify
    const doc = await db.get(docId);
    console.log(`📖 Retrieved document: "${doc.title}"`);
    console.log(`📊 Tags: ${doc.tags.join(', ')}`);
    console.log(`👤 Author: ${doc.metadata?.author}`);

    // 2. Query Builder Example
    console.log('\n🔍 Query Builder Example');

    // Search using query builder with validation
    const searchBuilder = new QueryBuilder()
      .text('quarterly business report')
      .limit(5)
      .offset(0)
      .tagFilter('business')
      .pathFilter('/examples/*');

    const searchResults = await db.queryWithBuilder(searchBuilder);
    console.log(`📋 Found ${searchResults.total_count} documents matching search`);
    
    for (const result of searchResults.results) {
      console.log(`  📄 ${result.document.title} (score: ${result.score})`);
    }

    // 3. Semantic Search with Builder (if available)
    console.log('\n🧠 Semantic Search Example');
    
    try {
      const semanticBuilder = new QueryBuilder()
        .text('financial performance metrics')
        .limit(3)
        .semanticWeight(0.8);

      const semanticResults = await db.semanticSearchWithBuilder(semanticBuilder);
      console.log(`🎯 Semantic search found ${semanticResults.total_count} relevant documents`);
    } catch (error) {
      console.log('ℹ️  Semantic search not available in this instance');
    }

    // 4. Update Builder Example
    console.log('\n✏️  Update Builder Example');

    const updateBuilder = new UpdateBuilder()
      .title('Q4 2023 Business Report - FINAL')
      .addTag('final')
      .addTag('approved')
      .removeTag('draft')
      .addMetadata('approved_by', 'ceo@company.com')
      .addMetadata('approval_date', new Date().toISOString());

    const updatedDoc = await db.update(docId, updateBuilder.build());
    console.log(`✅ Updated document: "${updatedDoc.title}"`);
    console.log(`🏷️  New tags: ${updatedDoc.tags.join(', ')}`);

    // 5. Validation Examples
    console.log('\n🛡️  Validation Examples');

    // Demonstrate path validation
    try {
      new DocumentBuilder().path('../../../etc/passwd');
    } catch (error) {
      if (error instanceof ValidationError) {
        console.log('✅ Directory traversal blocked:', error.message);
      }
    }

    // Demonstrate title validation
    try {
      new DocumentBuilder().title('');
    } catch (error) {
      if (error instanceof ValidationError) {
        console.log('✅ Empty title blocked:', error.message);
      }
    }

    // Demonstrate tag validation
    try {
      new DocumentBuilder().addTag('invalid@tag');
    } catch (error) {
      if (error instanceof ValidationError) {
        console.log('✅ Invalid tag blocked:', error.message);
      }
    }

    // Demonstrate query parameter validation
    try {
      new QueryBuilder().limit(-1);
    } catch (error) {
      if (error instanceof ValidationError) {
        console.log('✅ Invalid limit blocked:', error.message);
      }
    }

    // 6. Working with Validated Types Directly
    console.log('\n🔒 Validated Types Example');

    // Create validated types explicitly
    const safePath = new ValidatedPath('/documents/secure-file.md');
    const safeTitle = new ValidatedTitle('Secure Document Title');
    
    const secureDoc = new DocumentBuilder()
      .path(safePath)           // Use pre-validated path
      .title(safeTitle)         // Use pre-validated title
      .content('Secure content with validated inputs')
      .addTag('secure')
      .build();

    console.log('✅ Created document with pre-validated types');
    console.log(`📁 Path: ${safePath.asStr()}`);
    console.log(`📝 Title: ${safeTitle.asStr()}`);

    // Cleanup
    console.log('\n🧹 Cleanup');
    await db.delete(docId);
    console.log('✅ Deleted test document');

  } catch (error) {
    if (error instanceof ValidationError) {
      console.error('❌ Validation Error:', error.message);
    } else {
      console.error('❌ Unexpected Error:', error);
    }
  }
}

// Run the example
if (require.main === module) {
  builderPatternsExample()
    .then(() => {
      console.log('\n🎉 Builder patterns example completed successfully!');
      process.exit(0);
    })
    .catch((error) => {
      console.error('\n💥 Example failed:', error);
      process.exit(1);
    });
}

export { builderPatternsExample };