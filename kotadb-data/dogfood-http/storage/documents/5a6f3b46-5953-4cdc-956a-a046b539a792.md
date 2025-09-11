---
tags:
- file
- kota-db
- ext_html
---
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>KotaDB Quick Start Demo</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            line-height: 1.6;
            color: #333;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
        }
        
        .container {
            max-width: 1200px;
            margin: 0 auto;
            padding: 20px;
        }
        
        .header {
            text-align: center;
            color: white;
            margin-bottom: 30px;
        }
        
        .header h1 {
            font-size: 3em;
            margin-bottom: 10px;
        }
        
        .header p {
            font-size: 1.2em;
            opacity: 0.9;
        }
        
        .demo-panel {
            background: white;
            border-radius: 15px;
            box-shadow: 0 20px 40px rgba(0,0,0,0.1);
            padding: 30px;
            margin-bottom: 20px;
        }
        
        .search-section {
            margin-bottom: 30px;
        }
        
        .search-box {
            display: flex;
            gap: 10px;
            margin-bottom: 20px;
        }
        
        .search-input {
            flex: 1;
            padding: 12px 15px;
            border: 2px solid #ddd;
            border-radius: 8px;
            font-size: 16px;
        }
        
        .search-button {
            background: #667eea;
            color: white;
            border: none;
            padding: 12px 25px;
            border-radius: 8px;
            cursor: pointer;
            font-weight: 600;
            transition: background 0.3s;
        }
        
        .search-button:hover {
            background: #5a6fd8;
        }
        
        .results {
            margin-top: 20px;
        }
        
        .result-item {
            background: #f8f9fa;
            border-left: 4px solid #667eea;
            padding: 15px;
            margin-bottom: 10px;
            border-radius: 5px;
        }
        
        .result-title {
            font-weight: 600;
            color: #333;
            margin-bottom: 5px;
        }
        
        .result-content {
            color: #666;
            font-size: 14px;
        }
        
        .result-tags {
            margin-top: 8px;
        }
        
        .tag {
            background: #e3f2fd;
            color: #1976d2;
            padding: 2px 8px;
            border-radius: 12px;
            font-size: 12px;
            margin-right: 5px;
        }
        
        .stats-grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin-bottom: 20px;
        }
        
        .stat-card {
            background: #f8f9fa;
            padding: 20px;
            border-radius: 10px;
            text-align: center;
        }
        
        .stat-number {
            font-size: 2em;
            font-weight: bold;
            color: #667eea;
        }
        
        .stat-label {
            color: #666;
            font-size: 14px;
        }
        
        .error {
            background: #ffebee;
            color: #c62828;
            padding: 10px 15px;
            border-radius: 5px;
            margin-bottom: 15px;
        }
        
        .loading {
            text-align: center;
            padding: 20px;
            color: #666;
        }
        
        .quick-actions {
            display: flex;
            gap: 10px;
            flex-wrap: wrap;
            margin-bottom: 20px;
        }
        
        .quick-action-btn {
            background: #e3f2fd;
            color: #1976d2;
            border: none;
            padding: 8px 15px;
            border-radius: 20px;
            cursor: pointer;
            font-size: 14px;
            transition: background 0.3s;
        }
        
        .quick-action-btn:hover {
            background: #bbdefb;
        }
        
        .footer {
            text-align: center;
            color: white;
            opacity: 0.8;
            margin-top: 40px;
        }
        
        .footer a {
            color: white;
            text-decoration: underline;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>🚀 KotaDB</h1>
            <p>Quick Start Demo - Interactive Web UI</p>
        </div>
        
        <div class="demo-panel">
            <h2>📊 Database Statistics</h2>
            <div class="stats-grid" id="statsGrid">
                <div class="loading">Loading stats...</div>
            </div>
        </div>
        
        <div class="demo-panel">
            <h2>🔍 Search Documents</h2>
            <div class="search-section">
                <div class="search-box">
                    <input type="text" class="search-input" id="searchInput" placeholder="Search for documents..." value="welcome">
                    <button class="search-button" onclick="search()">Search</button>
                </div>
                
                <div class="quick-actions">
                    <button class="quick-action-btn" onclick="quickSearch('welcome')">welcome</button>
                    <button class="quick-action-btn" onclick="quickSearch('features')">features</button>
                    <button class="quick-action-btn" onclick="quickSearch('quickstart')">quickstart</button>
                    <button class="quick-action-btn" onclick="quickSearch('database')">database</button>
                </div>
                
                <div class="results" id="searchResults"></div>
            </div>
        </div>
        
        <div class="demo-panel">
            <h2>🎯 Next Steps</h2>
            <p>This interactive demo shows KotaDB running with sample data. Here's what to try next:</p>
            <ul style="margin: 15px 0; padding-left: 20px;">
                <li><strong>Install a client library:</strong> <code>pip install kotadb-client</code> or <code>npm install kotadb-client</code></li>
                <li><strong>Run the examples:</strong> Check the <code>examples/</code> directory for comprehensive demos</li>
                <li><strong>Build your app:</strong> Use KotaDB for document storage, search, and AI applications</li>
                <li><strong>Read the docs:</strong> Visit the GitHub repository for full documentation</li>
            </ul>
        </div>
        
        <div class="footer">
            <p>
                KotaDB - A custom database for distributed human-AI cognition<br>
                <a href="https://github.com/jayminwest/kota-db" target="_blank">View on GitHub</a>
            </p>
        </div>
    </div>

    <script>
        const KOTADB_URL = 'http://localhost:8080';
        
        async function loadStats() {
            try {
                const response = await fetch(`${KOTADB_URL}/stats`);
                const stats = await response.json();
                
                const statsGrid = document.getElementById('statsGrid');
                statsGrid.innerHTML = '';
                
                const statCards = [
                    { label: 'Documents', value: stats.document_count || 0 },
                    { label: 'Index Entries', value: stats.index_entries || 0 },
                    { label: 'Storage Size', value: formatBytes(stats.storage_size_bytes || 0) },
                    { label: 'Cache Hit Rate', value: `${(stats.cache_hit_rate || 0 * 100).toFixed(1)}%` }
                ];
                
                statCards.forEach(stat => {
                    const card = document.createElement('div');
                    card.className = 'stat-card';
                    card.innerHTML = `
                        <div class="stat-number">${stat.value}</div>
                        <div class="stat-label">${stat.label}</div>
                    `;
                    statsGrid.appendChild(card);
                });
            } catch (error) {
                document.getElementById('statsGrid').innerHTML = 
                    '<div class="error">Failed to load stats. Make sure KotaDB is running on port 8080.</div>';
            }
        }
        
        async function search(query) {
            const searchQuery = query || document.getElementById('searchInput').value;
            const resultsDiv = document.getElementById('searchResults');
            
            if (!searchQuery.trim()) {
                resultsDiv.innerHTML = '<div class="error">Please enter a search query</div>';
                return;
            }
            
            resultsDiv.innerHTML = '<div class="loading">Searching...</div>';
            
            try {
                const response = await fetch(`${KOTADB_URL}/search?q=${encodeURIComponent(searchQuery)}&limit=5`);
                const data = await response.json();
                
                if (data.documents && data.documents.length > 0) {
                    resultsDiv.innerHTML = data.documents.map(doc => `
                        <div class="result-item">
                            <div class="result-title">${escapeHtml(doc.title || 'Untitled')}</div>
                            <div class="result-content">${escapeHtml((doc.content || '').substring(0, 200))}${doc.content && doc.content.length > 200 ? '...' : ''}</div>
                            <div class="result-tags">
                                ${(doc.tags || []).map(tag => `<span class="tag">${escapeHtml(tag)}</span>`).join('')}
                            </div>
                        </div>
                    `).join('');
                } else {
                    resultsDiv.innerHTML = '<div class="error">No documents found for your search query.</div>';
                }
            } catch (error) {
                resultsDiv.innerHTML = '<div class="error">Search failed. Make sure KotaDB is running on port 8080.</div>';
            }
        }
        
        function quickSearch(query) {
            document.getElementById('searchInput').value = query;
            search(query);
        }
        
        function formatBytes(bytes) {
            if (bytes === 0) return '0 B';
            const k = 1024;
            const sizes = ['B', 'KB', 'MB', 'GB'];
            const i = Math.floor(Math.log(bytes) / Math.log(k));
            return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
        }
        
        function escapeHtml(text) {
            const div = document.createElement('div');
            div.textContent = text;
            return div.innerHTML;
        }
        
        // Event listeners
        document.getElementById('searchInput').addEventListener('keypress', function(e) {
            if (e.key === 'Enter') {
                search();
            }
        });
        
        // Load initial data
        loadStats();
        search('welcome'); // Initial search
    </script>
</body>
</html>