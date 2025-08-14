# KotaDB RAG Pipeline Example

A comprehensive Retrieval-Augmented Generation (RAG) system demonstrating how to use KotaDB for AI applications. This example shows how to build production-ready RAG pipelines with document ingestion, vector search, and AI-powered question answering.

## Features Demonstrated

- **Document Ingestion**: Automatic chunking and preprocessing
- **Vector Embeddings**: Multiple embedding providers (OpenAI, TF-IDF, local models)
- **Semantic Search**: Find relevant content by meaning, not just keywords
- **Question Answering**: Generate contextual answers using retrieved information
- **Knowledge Base Management**: Statistics and monitoring
- **Real Database Integration**: Uses actual KotaDB server (no mocks)

## Architecture

```
┌─────────────────┐    ┌──────────────┐    ┌─────────────────┐
│   Documents     │    │  RAG Pipeline │    │   KotaDB        │
│                 │───►│              │───►│                 │
│ • PDF, Text     │    │ • Chunking   │    │ • Vector Index  │
│ • Web Pages     │    │ • Embeddings │    │ • Full-text     │
│ • Knowledge     │    │ • Retrieval  │    │ • Metadata      │
└─────────────────┘    └──────────────┘    └─────────────────┘
                              │
                              ▼
                       ┌──────────────┐
                       │   AI Model   │
                       │              │
                       │ • OpenAI     │
                       │ • Local LLM  │
                       │ • Fallback   │
                       └──────────────┘
```

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

3. **Optional - AI API Keys**: For best results
   ```bash
   export OPENAI_API_KEY=your_openai_key  # Optional but recommended
   ```

## Quick Start

```bash
# Install dependencies
pip install -r requirements.txt

# Run the complete RAG demo
python rag_demo.py
```

## What the Demo Does

### 1. Document Ingestion
```python
# Ingests sample documents about KotaDB
rag.ingest_document(
    title="KotaDB Overview",
    content="KotaDB is a custom database...",
    source="documentation"
)
```

### 2. Semantic Search
```python
# Find relevant chunks by meaning
results = rag.semantic_search("How does vector search work?")
# Returns chunks ranked by semantic similarity
```

### 3. Question Answering
```python
# Generate contextual answers
answer = rag.ask_question("What makes KotaDB fast?")
# Uses retrieved context to generate informed responses
```

## Embedding Providers

The pipeline supports multiple embedding providers:

### OpenAI (Recommended)
```bash
export OPENAI_API_KEY=your_key
# Uses text-embedding-ada-002 model
```

### TF-IDF (Fallback)
```python
# No API key required
# Uses scikit-learn TfidfVectorizer
```

### Mock Embeddings (Demo)
```python
# Hash-based consistent embeddings
# For testing without external dependencies
```

## Customization

### Adding Your Own Documents

```python
from rag_demo import RAGPipeline
from kotadb import KotaDB

# Initialize
db = KotaDB("http://localhost:8080")
rag = RAGPipeline(db)

# Add your content
rag.ingest_document(
    title="My Document",
    content="Your document content here...",
    source="my-source",
    metadata={"author": "You", "category": "research"}
)
```

### Configuration Options

```python
from rag_demo import RAGConfig

config = RAGConfig(
    chunk_size=500,           # Smaller chunks
    chunk_overlap=50,         # Less overlap
    max_results=10,           # More search results
    embedding_model="text-embedding-ada-002",
    completion_model="gpt-4", # Better AI model
    temperature=0.3           # More focused responses
)

rag = RAGPipeline(db, config)
```

### Advanced Usage

```python
# Batch document ingestion
documents = [
    {"title": "Doc 1", "content": "...", "source": "web"},
    {"title": "Doc 2", "content": "...", "source": "pdf"},
    # ... more documents
]

for doc in documents:
    rag.ingest_document(**doc)

# Advanced search with filters
results = rag.semantic_search(
    query="machine learning",
    max_results=5
)

# Custom answer generation
context_docs = rag.semantic_search("AI applications")
answer_result = rag.generate_answer(
    question="How is AI used in databases?",
    context_docs=context_docs
)
```

## Performance Expectations

Typical performance on modern hardware:

- **Document Ingestion**: 1-10 documents/second (depends on size and embedding provider)
- **Search Queries**: <100ms for semantic search
- **Answer Generation**: 1-5 seconds (depends on AI model)
- **Knowledge Base Size**: Tested with 1000+ document chunks

## Production Deployment

### Environment Variables
```bash
export KOTADB_URL=http://your-kotadb-server:8080
export OPENAI_API_KEY=your_production_key
export RAG_CHUNK_SIZE=1000
export RAG_MAX_RESULTS=5
```

### Docker Deployment
```dockerfile
FROM python:3.11-slim

WORKDIR /app
COPY requirements.txt .
RUN pip install -r requirements.txt

COPY . .
CMD ["python", "rag_demo.py"]
```

### Scaling Considerations

1. **Embedding Caching**: Cache embeddings to avoid regeneration
2. **Batch Processing**: Process documents in batches for efficiency
3. **Index Optimization**: Use KotaDB's vector index optimizations
4. **Load Balancing**: Distribute embedding generation across multiple servers

## Use Cases

### 1. Customer Support Bot
```python
# Ingest support documentation
rag.ingest_document("FAQ", faq_content, "support-docs")
rag.ingest_document("Troubleshooting", troubleshooting_content, "support-docs")

# Answer customer questions
answer = rag.ask_question("How do I reset my password?")
```

### 2. Research Assistant
```python
# Ingest research papers
for paper in research_papers:
    rag.ingest_document(paper.title, paper.content, "research")

# Answer research questions
answer = rag.ask_question("What are the latest developments in quantum computing?")
```

### 3. Code Documentation Search
```python
# Ingest code documentation
rag.ingest_document("API Docs", api_docs, "documentation")
rag.ingest_document("Examples", examples, "documentation")

# Help developers
answer = rag.ask_question("How do I authenticate API requests?")
```

## Common Issues

### No Embeddings Generated
```
⚠️ Using mock embeddings for provider: openai
```
**Solution**: Set your OpenAI API key or install scikit-learn for TF-IDF embeddings.

### KotaDB Connection Failed
```
❌ Connection failed: Connection refused
```
**Solution**: Start KotaDB server: `cargo run --bin kotadb -- serve`

### Poor Answer Quality
```
Answer: I don't have enough information...
```
**Solutions**:
- Ingest more relevant documents
- Use better embeddings (OpenAI vs TF-IDF)
- Adjust chunk size and overlap
- Improve question phrasing

### Slow Performance
```
Document ingestion taking too long
```
**Solutions**:
- Use faster embedding models
- Batch process documents
- Increase chunk size
- Cache embeddings

## Advanced Features

### Custom Document Chunking
```python
from rag_demo import DocumentChunker

chunker = DocumentChunker(chunk_size=500, overlap=50)
chunks = chunker.chunk_document(content, metadata)
```

### Hybrid Search (Text + Semantic)
```python
# Combine text search with semantic search
text_results = db.query("specific keywords")
semantic_results = rag.semantic_search("conceptual query")

# Merge and rank results
combined_results = merge_search_results(text_results, semantic_results)
```

### Knowledge Base Analytics
```python
stats = rag.get_knowledge_base_stats()
print(f"Total chunks: {stats['total_chunks']}")
print(f"Sources: {stats['sources']}")
print(f"Average chunk size: {stats['average_chunk_size']}")
```

## Testing

Run the demo to test all components:

```bash
# Full demo
python rag_demo.py

# Test individual components
python -c "
from rag_demo import RAGPipeline, RAGConfig
from kotadb import KotaDB

db = KotaDB('http://localhost:8080')
rag = RAGPipeline(db)

# Test ingestion
doc_id = rag.ingest_document('Test', 'Test content', 'test')
print(f'Ingested: {doc_id}')

# Test search
results = rag.semantic_search('test')
print(f'Found: {len(results)} results')
"
```

## Next Steps

1. **Integrate with your application**: Use the RAG pipeline in your own projects
2. **Add more embedding providers**: Support for Hugging Face, Cohere, etc.
3. **Implement caching**: Cache embeddings and search results
4. **Add evaluation metrics**: Measure RAG system performance
5. **Scale horizontally**: Distribute across multiple KotaDB instances

## Related Examples

- [Flask Web App](../flask-web-app/) - Web interface for document management
- [Note-Taking App](../note-taking-app/) - Advanced document organization
- [Python Client Examples](../../clients/python/examples/) - Client library usage

This RAG pipeline example demonstrates how to build sophisticated AI applications using KotaDB as the knowledge base backend.