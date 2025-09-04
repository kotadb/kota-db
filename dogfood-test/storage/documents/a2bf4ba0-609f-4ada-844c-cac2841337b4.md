---
tags:
- file
- kota-db
- ext_md
---
# Database Design Principles

Good database design is crucial for performance and maintainability.

## Normalization
- **1NF**: Atomic values, no repeating groups
- **2NF**: 1NF + no partial dependencies
- **3NF**: 2NF + no transitive dependencies

## Indexing Strategy
- Primary keys for unique identification
- Foreign keys for relationships
- Composite indices for multi-column queries
- Full-text indices for search

## Performance Considerations
- Query patterns drive index design
- Avoid over-indexing (write performance cost)
- Consider read vs write workload balance

## KotaDB Example
KotaDB uses multiple index types:
- B+ tree for primary access
- Trigram index for full-text search
- Graph index for relationships
- Vector index for semantic similarity
