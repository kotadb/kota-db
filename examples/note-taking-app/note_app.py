#!/usr/bin/env python3
"""
KotaDB Note-Taking Application Example

A complete note-taking application demonstrating advanced KotaDB features:
- Hierarchical note organization with folders
- Real-time search and filtering
- Note tags and metadata
- Export functionality
- Rich text support
- No mocks - real KotaDB integration

This example shows how to build a sophisticated document management system
using KotaDB's advanced indexing and search capabilities.

Usage:
    pip install -r requirements.txt
    python note_app.py
    # Visit http://localhost:5001
"""

import os
import json
import re
from datetime import datetime, timedelta
from typing import List, Dict, Any, Optional
from dataclasses import dataclass

from flask import Flask, render_template, request, jsonify, redirect, url_for, flash, send_file
from kotadb import KotaDB, DocumentBuilder, QueryBuilder, ValidatedPath
from kotadb.exceptions import KotaDBError, ValidationError

# Configure Flask app
app = Flask(__name__)
app.secret_key = os.environ.get('FLASK_SECRET_KEY', 'note-app-secret-change-in-production')

# Configure KotaDB connection
KOTADB_URL = os.environ.get('KOTADB_URL', 'http://localhost:8080')
db = KotaDB(KOTADB_URL)

@dataclass
class NoteStats:
    """Statistics for the note-taking application."""
    total_notes: int
    notes_today: int
    total_folders: int
    total_tags: int
    most_used_tags: List[str]
    recent_activity: List[Dict[str, Any]]

class NoteManager:
    """Advanced note management with KotaDB integration."""
    
    def __init__(self, kotadb_client: KotaDB):
        self.db = kotadb_client
        self.base_path = "/notes"
    
    def create_note(self, folder: str, title: str, content: str, tags: List[str] = None) -> str:
        """Create a new note with hierarchical path structure."""
        tags = tags or []
        
        # Create hierarchical path
        folder_clean = re.sub(r'[^\w\-_/]', '', folder.strip('/'))
        title_clean = re.sub(r'[^\w\-_ ]', '', title)
        path = f"{self.base_path}/{folder_clean}/{title_clean}.md"
        
        # Add automatic tags
        auto_tags = self._generate_auto_tags(content, folder)
        all_tags = list(set(tags + auto_tags))
        
        # Build note with metadata
        builder = (DocumentBuilder()
                  .path(ValidatedPath(path))
                  .title(title)
                  .content(self._format_note_content(title, content, folder)))
        
        for tag in all_tags:
            builder.add_tag(tag)
        
        # Add folder tag
        builder.add_tag(f"folder:{folder}")
        builder.add_tag("note")  # All notes get this tag
        
        return self.db.insert_with_builder(builder)
    
    def get_folder_structure(self) -> Dict[str, Any]:
        """Get hierarchical folder structure with note counts."""
        try:
            # Get all notes
            results = self.db.query("*", limit=1000)
            notes = results.get('documents', [])
            
            folders = {}
            for note in notes:
                if not note.get('path', '').startswith(self.base_path):
                    continue
                
                # Extract folder from path
                path_parts = note['path'].replace(self.base_path + '/', '').split('/')
                if len(path_parts) > 1:
                    folder = path_parts[0]
                    if folder not in folders:
                        folders[folder] = {'count': 0, 'notes': []}
                    folders[folder]['count'] += 1
                    folders[folder]['notes'].append({
                        'id': note['id'],
                        'title': note.get('title', 'Untitled'),
                        'tags': note.get('tags', []),
                        'created': note.get('created_at', 'Unknown')
                    })
            
            return folders
        except Exception as e:
            print(f"Error getting folder structure: {e}")
            return {}
    
    def search_notes(self, query: str = None, folder: str = None, tags: List[str] = None, 
                    limit: int = 50) -> List[Dict[str, Any]]:
        """Advanced note searching with multiple filters."""
        try:
            builder = QueryBuilder()
            
            # Text search
            if query:
                builder.text(query)
            
            # Folder filter
            if folder:
                builder.tag_filter(f"folder:{folder}")
            
            # Tag filters (combine multiple tags)
            if tags:
                for tag in tags:
                    builder.tag_filter(tag)
            
            # Always filter to notes only
            builder.tag_filter("note")
            builder.limit(limit)
            
            results = self.db.query_with_builder(builder)
            return results.get('documents', [])
        except Exception as e:
            print(f"Error searching notes: {e}")
            return []
    
    def get_note_stats(self) -> NoteStats:
        """Get comprehensive statistics about notes."""
        try:
            # Get all notes
            all_notes = self.search_notes(limit=1000)
            
            # Calculate stats
            total_notes = len(all_notes)
            
            # Notes created today
            today = datetime.now().date()
            notes_today = 0  # Would need created_at field from KotaDB
            
            # Count folders
            folders = set()
            all_tags = []
            
            for note in all_notes:
                # Extract folder from tags
                for tag in note.get('tags', []):
                    if tag.startswith('folder:'):
                        folders.add(tag[7:])  # Remove 'folder:' prefix
                    elif tag != 'note':  # Exclude the automatic 'note' tag
                        all_tags.append(tag)
            
            # Most used tags
            tag_counts = {}
            for tag in all_tags:
                tag_counts[tag] = tag_counts.get(tag, 0) + 1
            
            most_used_tags = sorted(tag_counts.keys(), 
                                  key=lambda x: tag_counts[x], 
                                  reverse=True)[:5]
            
            return NoteStats(
                total_notes=total_notes,
                notes_today=notes_today,
                total_folders=len(folders),
                total_tags=len(set(all_tags)),
                most_used_tags=most_used_tags,
                recent_activity=all_notes[:5]  # Most recent
            )
        except Exception as e:
            print(f"Error getting stats: {e}")
            return NoteStats(0, 0, 0, 0, [], [])
    
    def export_notes(self, format: str = 'markdown') -> str:
        """Export all notes to a single file."""
        try:
            notes = self.search_notes(limit=1000)
            
            if format == 'markdown':
                content = "# My Notes Export\n\n"
                content += f"Exported on: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n"
                content += f"Total notes: {len(notes)}\n\n"
                content += "---\n\n"
                
                for note in notes:
                    content += f"## {note.get('title', 'Untitled')}\n\n"
                    content += f"**Tags:** {', '.join(note.get('tags', []))}\n\n"
                    content += f"{note.get('content', '')}\n\n"
                    content += "---\n\n"
                
                return content
            
            # JSON export
            elif format == 'json':
                return json.dumps(notes, indent=2, default=str)
            
            else:
                raise ValueError(f"Unsupported export format: {format}")
                
        except Exception as e:
            print(f"Error exporting notes: {e}")
            return f"Export failed: {e}"
    
    def _generate_auto_tags(self, content: str, folder: str) -> List[str]:
        """Generate automatic tags based on content analysis."""
        auto_tags = []
        
        # Programming language detection
        code_patterns = {
            'python': [r'def\s+\w+', r'import\s+\w+', r'from\s+\w+', r'class\s+\w+'],
            'javascript': [r'function\s+\w+', r'const\s+\w+', r'let\s+\w+', r'=>'],
            'rust': [r'fn\s+\w+', r'let\s+mut', r'impl\s+\w+', r'struct\s+\w+'],
            'sql': [r'SELECT\s+', r'FROM\s+\w+', r'WHERE\s+', r'INSERT\s+INTO']
        }
        
        content_lower = content.lower()
        for lang, patterns in code_patterns.items():
            if any(re.search(pattern, content, re.IGNORECASE) for pattern in patterns):
                auto_tags.append(lang)
        
        # Content type detection
        if len(content) > 1000:
            auto_tags.append('long-form')
        elif any(marker in content_lower for marker in ['todo', '- [ ]', 'task']):
            auto_tags.append('todo')
        elif any(marker in content_lower for marker in ['meeting', 'attendees', 'agenda']):
            auto_tags.append('meeting')
        
        # Folder-based tags
        if 'project' in folder.lower():
            auto_tags.append('project')
        elif 'personal' in folder.lower():
            auto_tags.append('personal')
        
        return auto_tags
    
    def _format_note_content(self, title: str, content: str, folder: str) -> str:
        """Format note content with metadata header."""
        header = f"# {title}\n\n"
        header += f"**Folder:** {folder}\n"
        header += f"**Created:** {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n\n"
        header += "---\n\n"
        return header + content

# Initialize note manager
note_manager = NoteManager(db)

# --- Web Routes ---

@app.route('/')
def dashboard():
    """Main dashboard with note statistics and recent activity."""
    try:
        stats = note_manager.get_note_stats()
        folders = note_manager.get_folder_structure()
        recent_notes = note_manager.search_notes(limit=5)
        
        return render_template('dashboard.html', 
                             stats=stats, 
                             folders=folders,
                             recent_notes=recent_notes,
                             kotadb_url=KOTADB_URL)
    except Exception as e:
        flash(f'Error loading dashboard: {e}', 'error')
        return render_template('dashboard.html', 
                             stats=NoteStats(0, 0, 0, 0, [], []), 
                             folders={},
                             recent_notes=[],
                             kotadb_url=KOTADB_URL)

@app.route('/folders')
def folder_view():
    """View notes organized by folders."""
    folders = note_manager.get_folder_structure()
    return render_template('folders.html', folders=folders)

@app.route('/folder/<folder_name>')
def view_folder(folder_name):
    """View all notes in a specific folder."""
    notes = note_manager.search_notes(folder=folder_name)
    return render_template('folder_notes.html', folder=folder_name, notes=notes)

@app.route('/create')
def create_note_form():
    """Show note creation form."""
    folders = list(note_manager.get_folder_structure().keys())
    return render_template('create_note.html', folders=folders)

@app.route('/create', methods=['POST'])
def create_note():
    """Handle note creation."""
    try:
        folder = request.form.get('folder', '').strip() or 'General'
        title = request.form.get('title', '').strip()
        content = request.form.get('content', '').strip()
        tags_input = request.form.get('tags', '').strip()
        
        if not title:
            flash('Title is required', 'error')
            return redirect(url_for('create_note_form'))
        
        # Parse tags
        tags = [tag.strip() for tag in tags_input.split(',') if tag.strip()]
        
        # Create note
        note_id = note_manager.create_note(folder, title, content, tags)
        
        flash(f'Note "{title}" created successfully!', 'success')
        return redirect(url_for('view_note', note_id=note_id))
        
    except ValidationError as e:
        flash(f'Validation error: {e}', 'error')
        return redirect(url_for('create_note_form'))
    except Exception as e:
        flash(f'Error creating note: {e}', 'error')
        return redirect(url_for('create_note_form'))

@app.route('/note/<note_id>')
def view_note(note_id):
    """View a specific note."""
    try:
        note = db.get(note_id)
        return render_template('view_note.html', note=note)
    except Exception as e:
        flash(f'Error loading note: {e}', 'error')
        return redirect(url_for('dashboard'))

@app.route('/search')
def search_notes():
    """Search interface for notes."""
    query = request.args.get('q', '').strip()
    folder = request.args.get('folder', '').strip()
    tag = request.args.get('tag', '').strip()
    
    notes = []
    if query or folder or tag:
        tags_list = [tag] if tag else None
        notes = note_manager.search_notes(query=query, folder=folder, tags=tags_list)
    
    folders = list(note_manager.get_folder_structure().keys())
    
    return render_template('search_notes.html', 
                         notes=notes, 
                         query=query, 
                         selected_folder=folder,
                         selected_tag=tag,
                         folders=folders)

@app.route('/export')
def export_notes():
    """Export all notes."""
    format_type = request.args.get('format', 'markdown')
    
    try:
        content = note_manager.export_notes(format_type)
        
        # Create temporary file
        import tempfile
        with tempfile.NamedTemporaryFile(mode='w', 
                                       suffix=f'.{format_type}', 
                                       delete=False) as f:
            f.write(content)
            temp_path = f.name
        
        return send_file(temp_path, 
                        as_attachment=True, 
                        download_name=f'notes_export_{datetime.now().strftime("%Y%m%d")}.{format_type}')
    
    except Exception as e:
        flash(f'Export failed: {e}', 'error')
        return redirect(url_for('dashboard'))

# --- API Routes ---

@app.route('/api/notes', methods=['GET'])
def api_get_notes():
    """API endpoint to get notes with filtering."""
    query = request.args.get('q')
    folder = request.args.get('folder')
    tag = request.args.get('tag')
    limit = int(request.args.get('limit', 20))
    
    tags_list = [tag] if tag else None
    notes = note_manager.search_notes(query=query, folder=folder, tags=tags_list, limit=limit)
    
    return jsonify({
        'notes': notes,
        'count': len(notes),
        'filters': {'query': query, 'folder': folder, 'tag': tag}
    })

@app.route('/api/stats')
def api_get_stats():
    """API endpoint for note statistics."""
    stats = note_manager.get_note_stats()
    return jsonify({
        'total_notes': stats.total_notes,
        'notes_today': stats.notes_today,
        'total_folders': stats.total_folders,
        'total_tags': stats.total_tags,
        'most_used_tags': stats.most_used_tags
    })

@app.route('/api/folders')
def api_get_folders():
    """API endpoint for folder structure."""
    folders = note_manager.get_folder_structure()
    return jsonify(folders)

if __name__ == '__main__':
    print("📝 Starting KotaDB Note-Taking App")
    print(f"📡 Connecting to KotaDB at: {KOTADB_URL}")
    
    # Test KotaDB connection
    try:
        stats = db.stats()
        print(f"✅ Connected! Database has {stats.get('document_count', 0)} documents")
    except Exception as e:
        print(f"❌ Warning: Could not connect to KotaDB: {e}")
        print("   Make sure KotaDB server is running at", KOTADB_URL)
    
    print("🌐 Starting Note App server at http://localhost:5001")
    print("📚 Features: Hierarchical notes, Advanced search, Export, Statistics")
    
    app.run(debug=True, host='0.0.0.0', port=5001)