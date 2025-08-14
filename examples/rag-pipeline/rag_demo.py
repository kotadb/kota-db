#!/usr/bin/env python3
"""
KotaDB RAG (Retrieval-Augmented Generation) Pipeline Example

A complete RAG system demonstrating how to use KotaDB for AI applications:
- Document ingestion and preprocessing
- Vector embeddings storage
- Semantic search and retrieval
- AI-powered question answering
- Real KotaDB integration (no mocks)
- Multiple embedding providers

This example shows how to build production-ready RAG systems with KotaDB
as the vector database backend.

Usage:
    pip install -r requirements.txt
    export OPENAI_API_KEY=your_key  # or use local models
    python rag_demo.py
"""

import os
import json
import re
from typing import List, Dict, Any, Optional, Tuple
from dataclasses import dataclass
from datetime import datetime

# Core dependencies
from kotadb import KotaDB, DocumentBuilder, QueryBuilder, ValidatedPath
from kotadb.exceptions import KotaDBError, ValidationError

# AI/ML dependencies (with fallbacks for demo)
try:
    import openai
    HAS_OPENAI = True
except ImportError:
    print("⚠️  OpenAI not available - using mock responses")
    HAS_OPENAI = False

try:
    import numpy as np
    from sklearn.feature_extraction.text import TfidfVectorizer
    from sklearn.metrics.pairwise import cosine_similarity
    HAS_SKLEARN = True
except ImportError:
    print("⚠️  scikit-learn not available - using basic text search")
    HAS_SKLEARN = False

@dataclass
class RAGConfig:
    """Configuration for the RAG pipeline."""
    chunk_size: int = 1000
    chunk_overlap: int = 100
    max_context_length: int = 4000
    embedding_model: str = "text-embedding-ada-002"
    completion_model: str = "gpt-3.5-turbo"
    temperature: float = 0.7
    max_results: int = 5

class DocumentChunker:
    """Split documents into chunks for embedding."""
    
    def __init__(self, chunk_size: int = 1000, overlap: int = 100):
        self.chunk_size = chunk_size
        self.overlap = overlap
    
    def chunk_document(self, content: str, metadata: Dict[str, Any] = None) -> List[Dict[str, Any]]:
        """Split document into overlapping chunks."""
        metadata = metadata or {}
        
        # Clean and normalize content
        content = self._clean_content(content)
        
        if len(content) <= self.chunk_size:
            return [{
                'content': content,
                'metadata': metadata,
                'chunk_index': 0,
                'chunk_count': 1
            }]
        
        chunks = []
        start = 0
        chunk_index = 0
        
        while start < len(content):
            end = start + self.chunk_size
            
            # Try to break at sentence boundary
            if end < len(content):
                # Look for sentence endings in the last 200 characters
                sentence_end = self._find_sentence_boundary(content[start:end])
                if sentence_end != -1:
                    end = start + sentence_end + 1
            
            chunk_content = content[start:end].strip()
            if chunk_content:
                chunk_metadata = metadata.copy()
                chunk_metadata.update({
                    'chunk_index': chunk_index,
                    'start_char': start,
                    'end_char': end
                })
                
                chunks.append({
                    'content': chunk_content,
                    'metadata': chunk_metadata,
                    'chunk_index': chunk_index,
                    'chunk_count': 0  # Will be updated later
                })
            
            start = end - self.overlap
            chunk_index += 1
        
        # Update chunk counts
        for chunk in chunks:
            chunk['chunk_count'] = len(chunks)
        
        return chunks
    
    def _clean_content(self, content: str) -> str:
        """Clean and normalize content for chunking."""
        # Remove excessive whitespace
        content = re.sub(r'\s+', ' ', content)
        # Remove control characters
        content = re.sub(r'[\x00-\x1f\x7f]', '', content)
        return content.strip()
    
    def _find_sentence_boundary(self, text: str) -> int:
        """Find the best sentence boundary for chunk splitting."""
        # Look for sentence endings
        sentence_endings = ['. ', '! ', '? ', '.\n', '!\n', '?\n']
        
        best_pos = -1
        for ending in sentence_endings:
            pos = text.rfind(ending)
            if pos > best_pos and pos > len(text) * 0.8:  # In last 20%
                best_pos = pos + len(ending) - 1
        
        return best_pos

class EmbeddingProvider:
    """Abstraction for different embedding providers."""
    
    def __init__(self, provider: str = "openai", model: str = None):
        self.provider = provider
        self.model = model or "text-embedding-ada-002"
        
        if provider == "openai" and HAS_OPENAI:
            openai.api_key = os.getenv('OPENAI_API_KEY')
        elif provider == "tfidf" and HAS_SKLEARN:
            self.vectorizer = TfidfVectorizer(max_features=512, stop_words='english')
            self._is_fitted = False
        else:
            print(f"⚠️  Using mock embeddings for provider: {provider}")
    
    def embed_text(self, text: str) -> List[float]:
        """Generate embeddings for text."""
        if self.provider == "openai" and HAS_OPENAI:
            return self._openai_embed(text)
        elif self.provider == "tfidf" and HAS_SKLEARN:
            return self._tfidf_embed(text)
        else:
            return self._mock_embed(text)
    
    def embed_texts(self, texts: List[str]) -> List[List[float]]:
        """Generate embeddings for multiple texts."""
        if self.provider == "tfidf" and HAS_SKLEARN:
            return self._tfidf_embed_batch(texts)
        else:
            return [self.embed_text(text) for text in texts]
    
    def _openai_embed(self, text: str) -> List[float]:
        """Generate OpenAI embeddings."""
        try:
            response = openai.Embedding.create(
                input=text,
                model=self.model
            )
            return response['data'][0]['embedding']
        except Exception as e:
            print(f"OpenAI embedding error: {e}")
            return self._mock_embed(text)
    
    def _tfidf_embed(self, text: str) -> List[float]:
        """Generate TF-IDF embeddings."""
        if not self._is_fitted:
            # For single text, create a basic vector
            return self._mock_embed(text)
        
        vector = self.vectorizer.transform([text])
        return vector.toarray()[0].tolist()
    
    def _tfidf_embed_batch(self, texts: List[str]) -> List[List[float]]:
        """Generate TF-IDF embeddings for batch."""
        if not self._is_fitted:
            vectors = self.vectorizer.fit_transform(texts)
            self._is_fitted = True
        else:
            vectors = self.vectorizer.transform(texts)
        
        return vectors.toarray().tolist()
    
    def _mock_embed(self, text: str) -> List[float]:
        """Generate mock embeddings for demo purposes."""
        # Simple hash-based embedding for consistency
        hash_val = hash(text) % (2**31)
        np.random.seed(hash_val)
        return np.random.normal(0, 1, 384).tolist() if HAS_SKLEARN else [0.0] * 384

class RAGPipeline:
    """Complete RAG pipeline using KotaDB."""
    
    def __init__(self, kotadb_client: KotaDB, config: RAGConfig = None):
        self.db = kotadb_client
        self.config = config or RAGConfig()
        self.chunker = DocumentChunker(
            chunk_size=self.config.chunk_size,
            overlap=self.config.chunk_overlap
        )
        self.embedder = EmbeddingProvider("openai" if HAS_OPENAI else "tfidf")
        self.knowledge_base_path = "/knowledge"
    
    def ingest_document(self, title: str, content: str, source: str = None, 
                       metadata: Dict[str, Any] = None) -> List[str]:
        """Ingest a document into the RAG knowledge base."""
        try:
            metadata = metadata or {}
            metadata.update({
                'source': source or 'unknown',
                'ingested_at': datetime.now().isoformat(),
                'content_length': len(content)
            })
            
            # Chunk the document
            chunks = self.chunker.chunk_document(content, metadata)
            print(f"📄 Split '{title}' into {len(chunks)} chunks")
            
            # Generate embeddings and store chunks
            chunk_ids = []
            for i, chunk in enumerate(chunks):
                try:
                    # Generate embedding
                    embedding = self.embedder.embed_text(chunk['content'])
                    
                    # Create chunk document path
                    safe_title = re.sub(r'[^\w\-_]', '_', title)
                    chunk_path = f"{self.knowledge_base_path}/{safe_title}/chunk_{i:03d}.md"
                    
                    # Store chunk in KotaDB
                    builder = (DocumentBuilder()
                              .path(ValidatedPath(chunk_path))
                              .title(f"{title} - Chunk {i+1}")
                              .content(chunk['content']))
                    
                    # Add tags
                    builder.add_tag("knowledge-base")
                    builder.add_tag("chunk")
                    builder.add_tag(f"source:{source or 'unknown'}")
                    
                    # Store embedding as metadata (JSON string)
                    # Note: In production, you'd use KotaDB's vector index
                    chunk_metadata = chunk['metadata'].copy()
                    chunk_metadata['embedding'] = embedding
                    builder.add_tag(f"metadata:{json.dumps(chunk_metadata)}")
                    
                    chunk_id = self.db.insert_with_builder(builder)
                    chunk_ids.append(chunk_id)
                    
                except Exception as e:
                    print(f"⚠️  Error processing chunk {i}: {e}")
                    continue
            
            print(f"✅ Ingested '{title}': {len(chunk_ids)} chunks stored")
            return chunk_ids
            
        except Exception as e:
            print(f"❌ Error ingesting document '{title}': {e}")
            return []
    
    def semantic_search(self, query: str, max_results: int = None) -> List[Dict[str, Any]]:
        """Perform semantic search over the knowledge base."""
        max_results = max_results or self.config.max_results
        
        try:
            # Generate query embedding
            query_embedding = self.embedder.embed_text(query)
            
            # Get all knowledge base chunks
            results = self.db.query_with_builder(
                QueryBuilder()
                .tag_filter("knowledge-base")
                .tag_filter("chunk")
                .limit(1000)  # Get all for similarity comparison
            )
            
            chunks = results.get('documents', [])
            if not chunks:
                return []
            
            # Calculate similarities
            similarities = []
            for chunk in chunks:
                try:
                    # Extract embedding from metadata
                    metadata_tags = [tag for tag in chunk.get('tags', []) 
                                   if tag.startswith('metadata:')]
                    
                    if not metadata_tags:
                        continue
                    
                    metadata_str = metadata_tags[0][9:]  # Remove 'metadata:' prefix
                    metadata = json.loads(metadata_str)
                    chunk_embedding = metadata.get('embedding', [])
                    
                    if chunk_embedding:
                        similarity = self._cosine_similarity(query_embedding, chunk_embedding)
                        similarities.append({
                            'document': chunk,
                            'similarity': similarity,
                            'metadata': metadata
                        })
                        
                except Exception as e:
                    print(f"⚠️  Error processing chunk similarity: {e}")
                    continue
            
            # Sort by similarity and return top results
            similarities.sort(key=lambda x: x['similarity'], reverse=True)
            return similarities[:max_results]
            
        except Exception as e:
            print(f"❌ Semantic search error: {e}")
            # Fallback to text search
            return self._fallback_text_search(query, max_results)
    
    def generate_answer(self, question: str, context_docs: List[Dict[str, Any]] = None) -> Dict[str, Any]:
        """Generate an answer using retrieved context."""
        # Get context if not provided
        if context_docs is None:
            context_docs = self.semantic_search(question)
        
        # Build context string
        context_parts = []
        sources = []
        
        for doc_info in context_docs:
            doc = doc_info['document']
            content = doc.get('content', '').strip()
            title = doc.get('title', 'Unknown')
            
            if content:
                context_parts.append(f"From '{title}':\n{content}")
                sources.append(title)
        
        context = "\n\n".join(context_parts[:3])  # Limit context length
        
        # Generate answer
        if HAS_OPENAI and os.getenv('OPENAI_API_KEY'):
            answer = self._generate_openai_answer(question, context)
        else:
            answer = self._generate_fallback_answer(question, context)
        
        return {
            'answer': answer,
            'context_used': context,
            'sources': list(set(sources)),
            'context_docs_count': len(context_docs)
        }
    
    def ask_question(self, question: str) -> Dict[str, Any]:
        """Complete RAG pipeline: retrieve context and generate answer."""
        print(f"🤔 Question: {question}")
        
        # Retrieve relevant context
        print("🔍 Searching knowledge base...")
        context_docs = self.semantic_search(question)
        print(f"📚 Found {len(context_docs)} relevant chunks")
        
        # Generate answer
        print("🧠 Generating answer...")
        result = self.generate_answer(question, context_docs)
        
        return result
    
    def get_knowledge_base_stats(self) -> Dict[str, Any]:
        """Get statistics about the knowledge base."""
        try:
            results = self.db.query_with_builder(
                QueryBuilder()
                .tag_filter("knowledge-base")
                .limit(1000)
            )
            
            chunks = results.get('documents', [])
            
            # Count sources
            sources = set()
            total_content_length = 0
            
            for chunk in chunks:
                total_content_length += len(chunk.get('content', ''))
                
                # Extract source from tags
                for tag in chunk.get('tags', []):
                    if tag.startswith('source:'):
                        sources.add(tag[7:])
            
            return {
                'total_chunks': len(chunks),
                'unique_sources': len(sources),
                'total_content_length': total_content_length,
                'average_chunk_size': total_content_length // len(chunks) if chunks else 0,
                'sources': list(sources)
            }
            
        except Exception as e:
            print(f"Error getting knowledge base stats: {e}")
            return {}
    
    def _cosine_similarity(self, vec1: List[float], vec2: List[float]) -> float:
        """Calculate cosine similarity between two vectors."""
        if HAS_SKLEARN:
            return cosine_similarity([vec1], [vec2])[0][0]
        else:
            # Basic dot product similarity
            dot_product = sum(a * b for a, b in zip(vec1, vec2))
            magnitude1 = sum(a * a for a in vec1) ** 0.5
            magnitude2 = sum(a * a for a in vec2) ** 0.5
            
            if magnitude1 == 0 or magnitude2 == 0:
                return 0.0
            
            return dot_product / (magnitude1 * magnitude2)
    
    def _fallback_text_search(self, query: str, max_results: int) -> List[Dict[str, Any]]:
        """Fallback to basic text search when embeddings fail."""
        try:
            results = self.db.query_with_builder(
                QueryBuilder()
                .text(query)
                .tag_filter("knowledge-base")
                .limit(max_results)
            )
            
            chunks = results.get('documents', [])
            return [{'document': chunk, 'similarity': 0.5} for chunk in chunks]
            
        except Exception as e:
            print(f"Fallback search error: {e}")
            return []
    
    def _generate_openai_answer(self, question: str, context: str) -> str:
        """Generate answer using OpenAI API."""
        try:
            prompt = f"""Based on the following context, please answer the question. If the context doesn't contain enough information, say so.

Context:
{context}

Question: {question}

Answer:"""
            
            response = openai.ChatCompletion.create(
                model=self.config.completion_model,
                messages=[{"role": "user", "content": prompt}],
                temperature=self.config.temperature,
                max_tokens=500
            )
            
            return response.choices[0].message.content.strip()
            
        except Exception as e:
            print(f"OpenAI generation error: {e}")
            return self._generate_fallback_answer(question, context)
    
    def _generate_fallback_answer(self, question: str, context: str) -> str:
        """Generate a basic answer when AI is not available."""
        if not context.strip():
            return "I don't have enough information in the knowledge base to answer this question."
        
        # Simple keyword-based answer extraction
        question_words = set(question.lower().split())
        context_sentences = re.split(r'[.!?]+', context)
        
        # Find sentences with the most question words
        best_sentences = []
        for sentence in context_sentences:
            sentence = sentence.strip()
            if len(sentence) < 10:
                continue
            
            sentence_words = set(sentence.lower().split())
            overlap = len(question_words.intersection(sentence_words))
            
            if overlap > 0:
                best_sentences.append((sentence, overlap))
        
        if best_sentences:
            best_sentences.sort(key=lambda x: x[1], reverse=True)
            return f"Based on the knowledge base: {best_sentences[0][0]}"
        else:
            return "The knowledge base contains related information, but I cannot generate a specific answer without AI capabilities."

def demo_rag_pipeline():
    """Demonstrate the complete RAG pipeline."""
    print("🚀 KotaDB RAG Pipeline Demo")
    print("=" * 50)
    
    # Connect to KotaDB
    kotadb_url = os.getenv('KOTADB_URL', 'http://localhost:8080')
    print(f"📡 Connecting to KotaDB at {kotadb_url}")
    
    try:
        db = KotaDB(kotadb_url)
        stats = db.stats()
        print(f"✅ Connected! Database has {stats.get('document_count', 0)} documents")
    except Exception as e:
        print(f"❌ Connection failed: {e}")
        print("   Make sure KotaDB server is running")
        return
    
    # Initialize RAG pipeline
    rag = RAGPipeline(db)
    
    print("\n1️⃣ INGESTING SAMPLE DOCUMENTS")
    print("-" * 40)
    
    # Sample documents for the knowledge base
    sample_docs = [
        {
            'title': 'KotaDB Overview',
            'content': """KotaDB is a custom database designed for distributed human-AI cognition. 
            It features a unique storage engine with multiple index types including B+ trees for fast lookups, 
            trigram indices for full-text search, and vector indices for semantic search. 
            The system is built in Rust for performance and safety, offering sub-10ms query latency 
            and supporting over 3,600 operations per second. KotaDB requires zero external database 
            dependencies and provides ACID compliance through Write-Ahead Logging.""",
            'source': 'documentation'
        },
        {
            'title': 'Vector Search in KotaDB',
            'content': """KotaDB implements vector search using HNSW (Hierarchical Navigable Small World) 
            algorithms for efficient similarity search. This enables semantic search capabilities for 
            AI applications, allowing users to find documents based on meaning rather than just keywords. 
            The vector index supports multiple embedding models and provides fast approximate nearest 
            neighbor search with configurable accuracy-speed tradeoffs.""",
            'source': 'technical-guide'
        },
        {
            'title': 'RAG Applications with KotaDB',
            'content': """Retrieval-Augmented Generation (RAG) systems can leverage KotaDB as a 
            knowledge base backend. The database's fast vector search capabilities make it ideal 
            for retrieving relevant context documents for large language models. KotaDB's client 
            libraries provide easy integration with popular AI frameworks, enabling developers to 
            build production-ready RAG applications with minimal setup.""",
            'source': 'ai-guide'
        }
    ]
    
    # Ingest documents
    for doc in sample_docs:
        rag.ingest_document(doc['title'], doc['content'], doc['source'])
    
    print("\n2️⃣ KNOWLEDGE BASE STATISTICS")
    print("-" * 40)
    
    stats = rag.get_knowledge_base_stats()
    print(f"📊 Knowledge Base Stats:")
    for key, value in stats.items():
        print(f"   - {key}: {value}")
    
    print("\n3️⃣ SEMANTIC SEARCH DEMO")
    print("-" * 40)
    
    search_queries = [
        "How does vector search work?",
        "What is KotaDB performance?",
        "RAG implementation guide"
    ]
    
    for query in search_queries:
        print(f"\n🔍 Query: '{query}'")
        results = rag.semantic_search(query, max_results=2)
        
        for i, result in enumerate(results, 1):
            doc = result['document']
            similarity = result['similarity']
            print(f"   {i}. {doc.get('title', 'Untitled')} (similarity: {similarity:.3f})")
            print(f"      Content preview: {doc.get('content', '')[:100]}...")
    
    print("\n4️⃣ QUESTION ANSWERING DEMO")
    print("-" * 40)
    
    questions = [
        "What makes KotaDB fast?",
        "How do I use KotaDB for AI applications?",
        "What is the difference between text and vector search?"
    ]
    
    for question in questions:
        print(f"\n❓ Question: {question}")
        result = rag.ask_question(question)
        
        print(f"💡 Answer: {result['answer']}")
        print(f"📚 Sources: {', '.join(result['sources'])}")
        print(f"📄 Context chunks used: {result['context_docs_count']}")
    
    print("\n🎉 RAG DEMO COMPLETE!")
    print("=" * 50)
    print("✅ Demonstrated features:")
    print("   - Document ingestion and chunking")
    print("   - Vector embedding generation")
    print("   - Semantic search and retrieval")
    print("   - Context-aware answer generation")
    print("   - Knowledge base management")
    print("\n📚 Next steps:")
    print("   - Add your own documents to the knowledge base")
    print("   - Experiment with different embedding models")
    print("   - Integrate with your AI application")
    print("   - Scale to larger document collections")

if __name__ == '__main__':
    demo_rag_pipeline()