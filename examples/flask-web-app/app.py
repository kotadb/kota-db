#!/usr/bin/env python3
"""
KotaDB Flask Web Application Example

A complete web application demonstrating KotaDB integration with:
- Document CRUD operations
- Full-text search
- RESTful API
- Web UI for document management
- Real KotaDB server connection (no mocks)

Usage:
    pip install flask kotadb-client
    python app.py
    # Visit http://localhost:5000
"""

import os
import json
from datetime import datetime
from flask import Flask, render_template, request, jsonify, redirect, url_for, flash
from kotadb import KotaDB, DocumentBuilder, QueryBuilder, ValidatedPath
from kotadb.exceptions import KotaDBError, ValidationError

# Configure Flask app
app = Flask(__name__)
app.secret_key = os.environ.get('FLASK_SECRET_KEY', 'dev-secret-change-in-production')

# Configure KotaDB connection
KOTADB_URL = os.environ.get('KOTADB_URL', 'http://localhost:8080')
db = KotaDB(KOTADB_URL)

# --- API Routes ---

@app.route('/api/health')
def api_health():
    """Health check endpoint that tests KotaDB connection."""
    try:
        stats = db.stats()
        return jsonify({
            'status': 'healthy',
            'kotadb_connected': True,
            'document_count': stats.get('document_count', 0)
        })
    except Exception as e:
        return jsonify({
            'status': 'unhealthy',
            'kotadb_connected': False,
            'error': str(e)
        }), 503

@app.route('/api/documents', methods=['GET'])
def api_list_documents():
    """List all documents with optional search."""
    try:
        query = request.args.get('q', '').strip()
        limit = int(request.args.get('limit', 20))
        
        if query:
            # Search documents
            results = db.query(query, limit=limit)
            documents = results.get('documents', [])
        else:
            # Get recent documents via wildcard search
            results = db.query('*', limit=limit)
            documents = results.get('documents', [])
        
        return jsonify({
            'documents': documents,
            'count': len(documents),
            'query': query
        })
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@app.route('/api/documents', methods=['POST'])
def api_create_document():
    """Create a new document using builder pattern."""
    try:
        data = request.get_json()
        
        if not data:
            return jsonify({'error': 'No data provided'}), 400
        
        # Use DocumentBuilder for type safety
        builder = DocumentBuilder()
        
        # Required fields
        path = data.get('path')
        if not path:
            return jsonify({'error': 'Path is required'}), 400
        builder.path(ValidatedPath(path))
        
        title = data.get('title')
        if not title:
            return jsonify({'error': 'Title is required'}), 400
        builder.title(title)
        
        content = data.get('content', '')
        builder.content(content)
        
        # Optional tags
        tags = data.get('tags', [])
        for tag in tags:
            builder.add_tag(tag)
        
        # Insert document
        doc_id = db.insert_with_builder(builder)
        
        return jsonify({
            'id': doc_id,
            'message': 'Document created successfully'
        }), 201
        
    except ValidationError as e:
        return jsonify({'error': f'Validation error: {e}'}), 400
    except KotaDBError as e:
        return jsonify({'error': f'Database error: {e}'}), 500
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@app.route('/api/documents/<doc_id>', methods=['GET'])
def api_get_document(doc_id):
    """Get a specific document by ID."""
    try:
        document = db.get(doc_id)
        return jsonify(document)
    except KotaDBError as e:
        return jsonify({'error': str(e)}), 404
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@app.route('/api/documents/<doc_id>', methods=['PUT'])
def api_update_document(doc_id):
    """Update a specific document."""
    try:
        data = request.get_json()
        if not data:
            return jsonify({'error': 'No data provided'}), 400
        
        # Get existing document first
        existing_doc = db.get(doc_id)
        
        # Update with new data
        updated_doc = db.update(doc_id, data)
        
        return jsonify({
            'message': 'Document updated successfully',
            'document': updated_doc
        })
    except KotaDBError as e:
        return jsonify({'error': str(e)}), 404
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@app.route('/api/documents/<doc_id>', methods=['DELETE'])
def api_delete_document(doc_id):
    """Delete a specific document."""
    try:
        db.delete(doc_id)
        return jsonify({'message': 'Document deleted successfully'})
    except KotaDBError as e:
        return jsonify({'error': str(e)}), 404
    except Exception as e:
        return jsonify({'error': str(e)}), 500

@app.route('/api/search')
def api_search():
    """Advanced search endpoint with structured queries."""
    try:
        query_text = request.args.get('q', '').strip()
        tag_filter = request.args.get('tag')
        limit = int(request.args.get('limit', 20))
        
        if not query_text and not tag_filter:
            return jsonify({'error': 'Query or tag filter required'}), 400
        
        # Use QueryBuilder for structured search
        builder = QueryBuilder()
        
        if query_text:
            builder.text(query_text)
        
        if tag_filter:
            builder.tag_filter(tag_filter)
        
        builder.limit(limit)
        
        results = db.query_with_builder(builder)
        
        return jsonify(results)
    except Exception as e:
        return jsonify({'error': str(e)}), 500

# --- Web UI Routes ---

@app.route('/')
def index():
    """Main page with document list and search."""
    try:
        # Get recent documents
        results = db.query('*', limit=10)
        documents = results.get('documents', [])
        
        # Get database stats
        stats = db.stats()
        
        return render_template('index.html', 
                             documents=documents, 
                             stats=stats,
                             kotadb_url=KOTADB_URL)
    except Exception as e:
        flash(f'Error loading documents: {e}', 'error')
        return render_template('index.html', 
                             documents=[], 
                             stats={},
                             kotadb_url=KOTADB_URL)

@app.route('/create')
def create_form():
    """Show document creation form."""
    return render_template('create.html')

@app.route('/create', methods=['POST'])
def create_document():
    """Handle document creation from web form."""
    try:
        # Get form data
        path = request.form.get('path', '').strip()
        title = request.form.get('title', '').strip()
        content = request.form.get('content', '').strip()
        tags_input = request.form.get('tags', '').strip()
        
        # Validate required fields
        if not path:
            flash('Path is required', 'error')
            return redirect(url_for('create_form'))
        
        if not title:
            flash('Title is required', 'error')
            return redirect(url_for('create_form'))
        
        # Parse tags
        tags = [tag.strip() for tag in tags_input.split(',') if tag.strip()]
        
        # Create document with builder pattern
        builder = (DocumentBuilder()
                  .path(ValidatedPath(path))
                  .title(title)
                  .content(content))
        
        for tag in tags:
            builder.add_tag(tag)
        
        doc_id = db.insert_with_builder(builder)
        
        flash(f'Document "{title}" created successfully!', 'success')
        return redirect(url_for('view_document', doc_id=doc_id))
        
    except ValidationError as e:
        flash(f'Validation error: {e}', 'error')
        return redirect(url_for('create_form'))
    except Exception as e:
        flash(f'Error creating document: {e}', 'error')
        return redirect(url_for('create_form'))

@app.route('/document/<doc_id>')
def view_document(doc_id):
    """View a specific document."""
    try:
        document = db.get(doc_id)
        return render_template('document.html', document=document)
    except Exception as e:
        flash(f'Error loading document: {e}', 'error')
        return redirect(url_for('index'))

@app.route('/search')
def search():
    """Search documents."""
    query = request.args.get('q', '').strip()
    tag_filter = request.args.get('tag', '').strip()
    
    documents = []
    if query or tag_filter:
        try:
            builder = QueryBuilder()
            
            if query:
                builder.text(query)
            if tag_filter:
                builder.tag_filter(tag_filter)
            
            builder.limit(50)
            results = db.query_with_builder(builder)
            documents = results.get('documents', [])
        except Exception as e:
            flash(f'Search error: {e}', 'error')
    
    return render_template('search.html', 
                         documents=documents, 
                         query=query, 
                         tag_filter=tag_filter)

# --- Template Creation (since we're not using separate template files) ---

@app.before_first_request
def create_templates():
    """Create HTML templates in memory for demo purposes."""
    import os
    
    # Create templates directory
    template_dir = os.path.join(app.root_path, 'templates')
    os.makedirs(template_dir, exist_ok=True)
    
    # Base template
    base_template = '''<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{% block title %}KotaDB Web App{% endblock %}</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, sans-serif; margin: 0; padding: 20px; }
        .container { max-width: 1200px; margin: 0 auto; }
        .header { border-bottom: 2px solid #667eea; padding-bottom: 20px; margin-bottom: 30px; }
        .header h1 { color: #667eea; margin: 0; }
        .nav { margin-top: 10px; }
        .nav a { color: #667eea; text-decoration: none; margin-right: 20px; }
        .nav a:hover { text-decoration: underline; }
        .card { background: white; border: 1px solid #ddd; border-radius: 8px; padding: 20px; margin-bottom: 20px; }
        .btn { background: #667eea; color: white; border: none; padding: 10px 20px; border-radius: 5px; text-decoration: none; display: inline-block; cursor: pointer; }
        .btn:hover { background: #5a6fd8; }
        .form-group { margin-bottom: 20px; }
        .form-group label { display: block; margin-bottom: 5px; font-weight: bold; }
        .form-group input, .form-group textarea { width: 100%; padding: 10px; border: 1px solid #ddd; border-radius: 4px; box-sizing: border-box; }
        .form-group textarea { height: 200px; }
        .alert { padding: 15px; border-radius: 5px; margin-bottom: 20px; }
        .alert.success { background: #d4edda; color: #155724; border: 1px solid #c3e6cb; }
        .alert.error { background: #f8d7da; color: #721c24; border: 1px solid #f5c6cb; }
        .stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 20px; }
        .stat-card { text-align: center; background: #f8f9fa; padding: 20px; border-radius: 8px; }
        .stat-number { font-size: 2em; font-weight: bold; color: #667eea; }
        .tags { margin-top: 10px; }
        .tag { background: #e3f2fd; color: #1976d2; padding: 2px 8px; border-radius: 12px; font-size: 12px; margin-right: 5px; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🚀 KotaDB Web App Demo</h1>
            <div class="nav">
                <a href="{{ url_for('index') }}">Home</a>
                <a href="{{ url_for('create_form') }}">Create Document</a>
                <a href="{{ url_for('search') }}">Search</a>
            </div>
        </div>
        
        {% with messages = get_flashed_messages(with_categories=true) %}
            {% if messages %}
                {% for category, message in messages %}
                    <div class="alert {{ 'success' if category == 'success' else 'error' }}">
                        {{ message }}
                    </div>
                {% endfor %}
            {% endif %}
        {% endwith %}
        
        {% block content %}{% endblock %}
    </div>
</body>
</html>'''
    
    with open(os.path.join(template_dir, 'base.html'), 'w') as f:
        f.write(base_template)
    
    # Index template
    index_template = '''{% extends "base.html" %}
{% block title %}KotaDB Web App - Home{% endblock %}
{% block content %}
    <div class="stats">
        <div class="stat-card">
            <div class="stat-number">{{ stats.get('document_count', 0) }}</div>
            <div>Documents</div>
        </div>
        <div class="stat-card">
            <div class="stat-number">Connected</div>
            <div>KotaDB Server</div>
        </div>
    </div>
    
    <div class="card">
        <h2>Recent Documents</h2>
        {% if documents %}
            {% for doc in documents %}
                <div class="card">
                    <h3><a href="{{ url_for('view_document', doc_id=doc.id) }}">{{ doc.title }}</a></h3>
                    <p>{{ doc.content[:200] }}{% if doc.content|length > 200 %}...{% endif %}</p>
                    <div class="tags">
                        {% for tag in doc.tags %}
                            <span class="tag">{{ tag }}</span>
                        {% endfor %}
                    </div>
                </div>
            {% endfor %}
        {% else %}
            <p>No documents found. <a href="{{ url_for('create_form') }}">Create your first document!</a></p>
        {% endif %}
    </div>
{% endblock %}'''
    
    with open(os.path.join(template_dir, 'index.html'), 'w') as f:
        f.write(index_template)

# --- Error Handlers ---

@app.errorhandler(404)
def page_not_found(e):
    return jsonify({'error': 'Endpoint not found'}), 404

@app.errorhandler(500)
def internal_error(e):
    return jsonify({'error': 'Internal server error'}), 500

if __name__ == '__main__':
    print("🚀 Starting KotaDB Flask Web App Demo")
    print(f"📡 Connecting to KotaDB at: {KOTADB_URL}")
    
    # Test KotaDB connection
    try:
        stats = db.stats()
        print(f"✅ Connected! Database has {stats.get('document_count', 0)} documents")
    except Exception as e:
        print(f"❌ Warning: Could not connect to KotaDB: {e}")
        print("   Make sure KotaDB server is running at", KOTADB_URL)
    
    print("🌐 Starting Flask server at http://localhost:5000")
    print("📚 Features: Document CRUD, Search, Web UI, REST API")
    
    app.run(debug=True, host='0.0.0.0', port=5000)