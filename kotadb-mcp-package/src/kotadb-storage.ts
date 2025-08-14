/**
 * Simple in-memory KotaDB storage implementation for MCP server
 * This provides basic document storage without requiring the full Rust binary
 */

import * as fs from 'fs/promises';
import * as path from 'path';
import { randomUUID } from 'crypto';

export interface Document {
  id: string;
  path: string;
  title: string;
  content: string;
  tags: string[];
  createdAt: string;
  updatedAt: string;
}

export interface SearchResult {
  id: string;
  path: string;
  title: string;
  content_preview: string;
  score: number;
}

export class KotaDBStorage {
  private documents: Map<string, Document> = new Map();
  private dataDir: string;
  private indexPath: string;

  constructor(dataDir?: string) {
    this.dataDir = dataDir || path.join(process.env.HOME || process.cwd(), '.kotadb', 'data');
    this.indexPath = path.join(this.dataDir, 'index.json');
  }

  async initialize(): Promise<void> {
    try {
      await fs.mkdir(this.dataDir, { recursive: true });
      await this.loadIndex();
    } catch (error) {
      console.error('Failed to initialize KotaDB storage:', error);
      throw error;
    }
  }

  private async loadIndex(): Promise<void> {
    try {
      const indexData = await fs.readFile(this.indexPath, 'utf-8');
      const documents = JSON.parse(indexData) as Document[];
      
      for (const doc of documents) {
        this.documents.set(doc.id, doc);
      }
      
      console.log(`Loaded ${documents.length} documents from index`);
    } catch (error) {
      // Index doesn't exist yet, start with empty storage
      console.log('Starting with empty document storage');
    }
  }

  private async saveIndex(): Promise<void> {
    const documents = Array.from(this.documents.values());
    await fs.writeFile(this.indexPath, JSON.stringify(documents, null, 2));
  }

  async createDocument(params: {
    path: string;
    title?: string;
    content: string;
    tags?: string[];
  }): Promise<Document> {
    const doc: Document = {
      id: randomUUID(),
      path: params.path,
      title: params.title || path.basename(params.path, path.extname(params.path)),
      content: params.content,
      tags: params.tags || [],
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };

    this.documents.set(doc.id, doc);
    await this.saveIndex();

    // Also save as markdown file
    const filePath = path.join(this.dataDir, `${doc.id}.md`);
    const markdown = `# ${doc.title}\n\n${doc.content}\n\n<!-- Tags: ${doc.tags.join(', ')} -->\n`;
    await fs.writeFile(filePath, markdown);

    return doc;
  }

  async getDocument(id: string): Promise<Document | null> {
    return this.documents.get(id) || null;
  }

  async updateDocument(id: string, content: string): Promise<Document | null> {
    const doc = this.documents.get(id);
    if (!doc) {
      return null;
    }

    doc.content = content;
    doc.updatedAt = new Date().toISOString();
    this.documents.set(id, doc);
    await this.saveIndex();

    // Update markdown file
    const filePath = path.join(this.dataDir, `${doc.id}.md`);
    const markdown = `# ${doc.title}\n\n${doc.content}\n\n<!-- Tags: ${doc.tags.join(', ')} -->\n`;
    await fs.writeFile(filePath, markdown);

    return doc;
  }

  async deleteDocument(id: string): Promise<boolean> {
    const doc = this.documents.get(id);
    if (!doc) {
      return false;
    }

    this.documents.delete(id);
    await this.saveIndex();

    // Delete markdown file
    try {
      const filePath = path.join(this.dataDir, `${doc.id}.md`);
      await fs.unlink(filePath);
    } catch (error) {
      console.warn(`Failed to delete markdown file for ${id}:`, error);
    }

    return true;
  }

  async listDocuments(limit?: number, offset?: number): Promise<{ documents: Document[]; total: number }> {
    const allDocs = Array.from(this.documents.values());
    const total = allDocs.length;
    
    // Sort by updated date (most recent first)
    allDocs.sort((a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime());
    
    const startIndex = offset || 0;
    const endIndex = limit ? startIndex + limit : allDocs.length;
    const documents = allDocs.slice(startIndex, endIndex);

    return { documents, total };
  }

  async searchDocuments(query: string, limit?: number): Promise<SearchResult[]> {
    const results: SearchResult[] = [];
    const searchTerms = query.toLowerCase().split(/\s+/);
    
    for (const doc of this.documents.values()) {
      const searchText = `${doc.title} ${doc.content} ${doc.tags.join(' ')}`.toLowerCase();
      
      // Simple scoring based on term matches
      let score = 0;
      let matches = 0;
      
      for (const term of searchTerms) {
        const termMatches = (searchText.match(new RegExp(term, 'g')) || []).length;
        if (termMatches > 0) {
          matches++;
          score += termMatches;
        }
      }
      
      // Only include documents that match at least one term
      if (matches > 0) {
        // Boost score for title matches
        const titleMatches = searchTerms.some(term => 
          doc.title.toLowerCase().includes(term)
        );
        if (titleMatches) {
          score *= 2;
        }

        // Create content preview
        const contentPreview = this.createContentPreview(doc.content, query, 200);
        
        results.push({
          id: doc.id,
          path: doc.path,
          title: doc.title,
          content_preview: contentPreview,
          score: score / searchTerms.length, // Normalize by number of terms
        });
      }
    }
    
    // Sort by score (highest first)
    results.sort((a, b) => b.score - a.score);
    
    // Apply limit
    const maxResults = limit || 10;
    return results.slice(0, maxResults);
  }

  private createContentPreview(content: string, query: string, maxLength: number): string {
    const queryTerms = query.toLowerCase().split(/\s+/);
    
    // Find the first occurrence of any query term
    let bestIndex = -1;
    let bestTerm = '';
    
    for (const term of queryTerms) {
      const index = content.toLowerCase().indexOf(term);
      if (index !== -1 && (bestIndex === -1 || index < bestIndex)) {
        bestIndex = index;
        bestTerm = term;
      }
    }
    
    if (bestIndex === -1) {
      // No query terms found, return beginning of content
      return content.slice(0, maxLength) + (content.length > maxLength ? '...' : '');
    }
    
    // Create preview centered around the found term
    const start = Math.max(0, bestIndex - maxLength / 2);
    const end = Math.min(content.length, start + maxLength);
    const preview = content.slice(start, end);
    
    const prefix = start > 0 ? '...' : '';
    const suffix = end < content.length ? '...' : '';
    
    return prefix + preview + suffix;
  }

  async getStats(): Promise<{
    total_documents: number;
    total_size_bytes: number;
    data_directory: string;
  }> {
    const documents = Array.from(this.documents.values());
    const totalSizeBytes = documents.reduce((sum, doc) => 
      sum + doc.content.length + doc.title.length, 0
    );

    return {
      total_documents: documents.length,
      total_size_bytes: totalSizeBytes,
      data_directory: this.dataDir,
    };
  }
}