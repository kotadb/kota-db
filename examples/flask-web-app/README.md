# KotaDB Flask Web Application Example

A complete web application demonstrating KotaDB integration with Flask. This example shows how to build a real-world document management system using KotaDB as the backend database.

## Features Demonstrated

- **Document CRUD Operations**: Create, read, update, delete documents
- **Full-text Search**: Search across all documents with relevance ranking
- **RESTful API**: Complete REST API for document operations
- **Web User Interface**: Interactive web UI for document management
- **Type Safety**: Builder patterns for safe document creation
- **Real Database**: Uses actual KotaDB server (no mocks)

## Prerequisites

1. **KotaDB Server**: Must be running on http://localhost:8080
   ```bash
   # Start KotaDB server (from project root)
   cargo run --bin kotadb -- serve
   ```

2. **Python**: Version 3.8 or higher
   ```bash
   python --version  # Should be 3.8+
   ```

## Quick Start

```bash
# Install dependencies
pip install -r requirements.txt

# Start the web application
python app.py

# Visit http://localhost:5000 in your browser
```

## API Endpoints

The Flask app provides a complete REST API:

### Documents
- `GET /api/documents` - List all documents (with optional search)
- `POST /api/documents` - Create new document
- `GET /api/documents/<id>` - Get specific document
- `PUT /api/documents/<id>` - Update document
- `DELETE /api/documents/<id>` - Delete document

### Search
- `GET /api/search?q=query&tag=filter&limit=20` - Advanced search

### Health
- `GET /api/health` - Health check and connection status

## API Usage Examples

### Create Document
```bash
curl -X POST http://localhost:5000/api/documents \
  -H "Content-Type: application/json" \
  -d '{
    "path": "/docs/example.md",
    "title": "Example Document",
    "content": "This is example content",
    "tags": ["example", "demo"]
  }'
```

### Search Documents
```bash
# Text search
curl "http://localhost:5000/api/search?q=example&limit=10"

# Tag filter
curl "http://localhost:5000/api/search?tag=demo&limit=10"

# Combined search
curl "http://localhost:5000/api/search?q=document&tag=example"
```

### List Documents
```bash
# All documents
curl "http://localhost:5000/api/documents"

# Search while listing
curl "http://localhost:5000/api/documents?q=example"
```

## Web Interface Features

Visit http://localhost:5000 to use the web interface:

1. **Home Page**: View recent documents and database statistics
2. **Create Document**: Form-based document creation with validation
3. **Document View**: Read individual documents with full content
4. **Search**: Full-text search with tag filtering
5. **Real-time Stats**: Live database statistics

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Web Browser   │    │   Flask App     │    │   KotaDB Server │
│                 │◄──►│                 │◄──►│                 │
│ HTML/CSS/JS     │    │ Python + REST   │    │ Rust Database   │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Key Components

1. **Flask Application** (`app.py`):
   - Web routes for HTML pages
   - REST API endpoints
   - Error handling and validation
   - Template rendering

2. **KotaDB Integration**:
   - Uses `kotadb-client` Python library
   - Builder patterns for type safety
   - Real database operations (no mocks)
   - Proper error handling

3. **Frontend** (embedded templates):
   - Responsive HTML/CSS
   - JavaScript for interactivity
   - Form validation
   - Real-time updates

## Type Safety Features

This example demonstrates KotaDB's type safety features:

```python
# Type-safe document creation
builder = (DocumentBuilder()
          .path(ValidatedPath("/docs/safe.md"))  # Validates path format
          .title("Safe Document")                # Validates title
          .content("Content here"))              # Validates content

for tag in tags:
    builder.add_tag(tag)  # Validates each tag

doc_id = db.insert_with_builder(builder)
```

## Error Handling

The app includes comprehensive error handling:

- **Validation Errors**: Path format, required fields
- **Connection Errors**: KotaDB server unavailable
- **Database Errors**: Document not found, conflicts
- **HTTP Errors**: 404, 500 with proper JSON responses

## Development

### Environment Variables

- `KOTADB_URL`: KotaDB server URL (default: http://localhost:8080)
- `FLASK_SECRET_KEY`: Flask session secret (default: dev key)

### Running in Development

```bash
export FLASK_ENV=development
export KOTADB_URL=http://localhost:8080
python app.py
```

### Production Deployment

```bash
# Install production WSGI server
pip install gunicorn

# Run with Gunicorn
gunicorn -w 4 -b 0.0.0.0:5000 app:app
```

## Testing

Test the application manually:

1. **Create Documents**: Use the web form or API
2. **Search Documents**: Try different search queries
3. **View Documents**: Click on document titles
4. **API Testing**: Use curl or Postman

## Performance Expectations

On typical hardware:
- **Document Creation**: <100ms per document
- **Search Queries**: <200ms for most searches
- **Document Retrieval**: <50ms per document
- **Concurrent Users**: 10-50 users comfortably

## Common Issues

### KotaDB Connection Failed
```
❌ Warning: Could not connect to KotaDB
```
**Solution**: Make sure KotaDB server is running:
```bash
cargo run --bin kotadb -- serve
```

### Port Already in Use
```
❌ Address already in use
```
**Solution**: Change the Flask port:
```bash
export FLASK_PORT=5001
python app.py
```

### Import Errors
```
❌ No module named 'kotadb'
```
**Solution**: Install the client library:
```bash
pip install kotadb-client
```

## Next Steps

1. **Extend the API**: Add more endpoints for advanced features
2. **Add Authentication**: User login/logout functionality
3. **Add Caching**: Redis caching for better performance
4. **Add Tests**: Unit and integration tests
5. **Production Deployment**: Docker containerization

## Related Examples

- [Note-Taking App](../note-taking-app/) - More advanced UI
- [RAG Pipeline](../rag-pipeline/) - AI integration
- [Python Client Examples](../../clients/python/examples/) - Client library usage

This Flask example demonstrates how to build production-ready web applications with KotaDB as the backend database.