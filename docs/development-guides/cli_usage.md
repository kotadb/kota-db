# KotaDB CLI Usage Guide

KotaDB provides a simple command-line interface for interacting with the document database.

## Building the CLI

First, build the project:

```bash
cd kota-db
cargo build --release
```

The CLI binary will be available at `target/release/kotadb`.

## Basic Usage

```bash
# Run with default database location (./kota-db-data)
kotadb <command>

# Specify custom database location
kotadb --db-path /path/to/database <command>
```

## Commands

### Insert a Document

```bash
# Insert with inline content
kotadb insert "/docs/readme.md" "Project README" "This is the content"

# Insert with piped content
echo "This is the content" | kotadb insert "/docs/readme.md" "Project README"

# Insert from file
cat document.txt | kotadb insert "/docs/document.md" "My Document"
```

### Get a Document

```bash
# Get by ID (UUID format)
kotadb get "123e4567-e89b-12d3-a456-426614174000"
```

### Update a Document

```bash
# Update title only
kotadb update "123e4567-e89b-12d3-a456-426614174000" --title "New Title"

# Update path only
kotadb update "123e4567-e89b-12d3-a456-426614174000" --path "/docs/new-path.md"

# Update content from stdin
echo "New content" | kotadb update "123e4567-e89b-12d3-a456-426614174000" --content -

# Update everything
kotadb update "123e4567-e89b-12d3-a456-426614174000" \
  --path "/docs/updated.md" \
  --title "Updated Title" \
  --content "New content"
```

### Delete a Document

```bash
kotadb delete "123e4567-e89b-12d3-a456-426614174000"
```

### Search Documents

```bash
# Search all documents (default)
kotadb search

# Search with query text
kotadb search "machine learning"

# Search with limit
kotadb search "rust" --limit 20

# Search with tags
kotadb search --tags "rust,database"

# Combined search
kotadb search "learning" --tags "ml,ai" --limit 5
```

### List All Documents

```bash
# List with default limit (50)
kotadb list

# List with custom limit
kotadb list --limit 100
```

### Database Statistics

```bash
kotadb stats
```

## Examples

### Example 1: Managing Documentation

```bash
# Create a new document
echo "# KotaDB Documentation

## Overview
KotaDB is a custom database designed for distributed human-AI cognition.

## Features
- Document storage with metadata
- Full-text search
- Tag-based filtering
" | kotadb insert "/docs/kotadb-overview.md" "KotaDB Overview"

# Output:
# ✅ Document inserted successfully!
#    ID: f47ac10b-58cc-4372-a567-0e02b2c3d479
#    Path: /docs/kotadb-overview.md
#    Title: KotaDB Overview

# Search for it
kotadb search "cognition"

# Update it
kotadb update "f47ac10b-58cc-4372-a567-0e02b2c3d479" --title "KotaDB Overview - Updated"
```

### Example 2: Batch Import

```bash
# Import multiple markdown files
for file in *.md; do
    title=$(basename "$file" .md | sed 's/-/ /g')
    kotadb insert "/imported/$file" "$title" < "$file"
done
```

### Example 3: Export Document

```bash
# Get document and save to file
kotadb get "f47ac10b-58cc-4372-a567-0e02b2c3d479" > exported-doc.txt
```

## Output Format

### Insert Command
```
✅ Document inserted successfully!
   ID: f47ac10b-58cc-4372-a567-0e02b2c3d479
   Path: /docs/readme.md
   Title: Project README
```

### Get Command
```
📄 Document found:
   ID: f47ac10b-58cc-4372-a567-0e02b2c3d479
   Path: /docs/readme.md
   Title: Project README
   Size: 1024 bytes
   Created: 2024-01-15 10:30:00 UTC
   Updated: 2024-01-15 10:30:00 UTC

--- Content ---
This is the document content...
```

### Search Command
```
🔍 Found 3 documents:

📄 Machine Learning Papers
   ID: f47ac10b-58cc-4372-a567-0e02b2c3d479
   Path: /research/ml-papers.md
   Size: 2048 bytes

📄 Learning Rust
   ID: 550e8400-e29b-41d4-a716-446655440000
   Path: /tutorials/rust.md
   Size: 1536 bytes
```

### Stats Command
```
📊 Database Statistics:
   Total documents: 42
   Total size: 125952 bytes
   Average size: 2998 bytes
```

## Error Handling

The CLI provides clear error messages:

- **Invalid document ID**: "Invalid document ID format"
- **Document not found**: "❌ Document not found"
- **Invalid path**: "Path cannot be empty"
- **Invalid title**: "Title cannot be empty"

## Tips

1. **Use pipes**: The CLI is designed to work well with Unix pipes for content input
2. **UUID format**: Document IDs must be valid UUIDs (e.g., `123e4567-e89b-12d3-a456-426614174000`)
3. **Path format**: Paths should start with `/` (e.g., `/docs/readme.md`)
4. **Tag format**: Multiple tags should be comma-separated without spaces
5. **Content input**: Use `-` with `--content` flag to read from stdin

## Troubleshooting

### Database not found
```bash
# Create the database directory first
mkdir -p ./kota-db-data
```

### Permission denied
```bash
# Ensure you have write permissions
chmod -R u+w ./kota-db-data
```

### Invalid UTF-8 content
The CLI expects UTF-8 encoded text. For binary files, consider base64 encoding first.