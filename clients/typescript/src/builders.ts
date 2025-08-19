/**
 * Builder patterns for KotaDB TypeScript client.
 *
 * Provides safe, fluent construction of documents and queries
 * with validation at each step.
 */

import {
  ValidatedPath,
  ValidatedDocumentId,
  ValidatedTitle,
  NonZeroSize,
  ValidatedTimestamp,
} from './validated-types';
import { ValidationError, validateTag, validateSearchQuery } from './validation';
import { CreateDocumentRequest, Document } from './types';

/**
 * Builder for creating Document objects with validation.
 *
 * Provides a fluent interface for safe document construction
 * mirroring the Rust builder patterns.
 *
 * @example
 * ```typescript
 * const doc = new DocumentBuilder()
 *   .path("/notes/meeting.md")
 *   .title("Team Meeting Notes")
 *   .content("Meeting content here...")
 *   .addTag("work")
 *   .addTag("meeting")
 *   .build();
 * ```
 */
export class DocumentBuilder {
  private _path?: ValidatedPath;
  private _title?: ValidatedTitle;
  private _content?: string | number[];
  private _tags: string[] = [];
  private _metadata: Record<string, any> = {};
  private _id?: ValidatedDocumentId;

  /**
   * Set the document path.
   *
   * @param path Path for the document (will be validated)
   * @returns Self for method chaining
   * @throws ValidationError If path is invalid
   */
  path(path: string | ValidatedPath): DocumentBuilder {
    if (typeof path === 'string') {
      this._path = new ValidatedPath(path);
    } else {
      this._path = path;
    }
    return this;
  }

  /**
   * Set the document title.
   *
   * @param title Title for the document (will be validated)
   * @returns Self for method chaining
   * @throws ValidationError If title is invalid
   */
  title(title: string | ValidatedTitle): DocumentBuilder {
    if (typeof title === 'string') {
      this._title = new ValidatedTitle(title);
    } else {
      this._title = title;
    }
    return this;
  }

  /**
   * Set the document content.
   *
   * @param content Content for the document
   * @returns Self for method chaining
   */
  content(content: string | number[]): DocumentBuilder {
    this._content = content;
    return this;
  }

  /**
   * Add a tag to the document.
   *
   * @param tag Tag to add (will be validated)
   * @returns Self for method chaining
   * @throws ValidationError If tag is invalid
   */
  addTag(tag: string): DocumentBuilder {
    validateTag(tag);
    if (!this._tags.includes(tag)) {
      this._tags.push(tag);
    }
    return this;
  }

  /**
   * Set all tags for the document.
   *
   * @param tags List of tags (each will be validated)
   * @returns Self for method chaining
   * @throws ValidationError If any tag is invalid
   */
  tags(tags: string[]): DocumentBuilder {
    for (const tag of tags) {
      validateTag(tag);
    }
    this._tags = [...tags]; // Create copy
    return this;
  }

  /**
   * Add a metadata field.
   *
   * @param key Metadata key
   * @param value Metadata value
   * @returns Self for method chaining
   */
  addMetadata(key: string, value: any): DocumentBuilder {
    this._metadata[key] = value;
    return this;
  }

  /**
   * Set all metadata for the document.
   *
   * @param metadata Metadata object
   * @returns Self for method chaining
   */
  metadata(metadata: Record<string, any>): DocumentBuilder {
    this._metadata = { ...metadata }; // Create copy
    return this;
  }

  /**
   * Set the document ID.
   *
   * @param docId Document ID (will be validated)
   * @returns Self for method chaining
   * @throws ValidationError If ID is invalid
   */
  id(docId: string | ValidatedDocumentId): DocumentBuilder {
    if (typeof docId === 'string') {
      this._id = new ValidatedDocumentId(docId);
    } else {
      this._id = docId;
    }
    return this;
  }

  /**
   * Generate a new random ID for the document.
   *
   * @returns Self for method chaining
   */
  autoId(): DocumentBuilder {
    this._id = ValidatedDocumentId.new();
    return this;
  }

  /**
   * Build the CreateDocumentRequest.
   *
   * @returns CreateDocumentRequest ready for insertion
   * @throws ValidationError If required fields are missing
   */
  build(): CreateDocumentRequest {
    if (!this._path) {
      throw new ValidationError('Document path is required');
    }

    if (!this._title) {
      throw new ValidationError('Document title is required');
    }

    if (this._content === undefined) {
      throw new ValidationError('Document content is required');
    }

    const request: CreateDocumentRequest = {
      path: this._path.asStr(),
      title: this._title.asStr(),
      content: this._content,
    };
    
    if (this._tags.length > 0) {
      request.tags = this._tags;
    }
    
    if (Object.keys(this._metadata).length > 0) {
      request.metadata = this._metadata;
    }
    
    return request;
  }

  /**
   * Build a complete Document with timestamps.
   *
   * @returns Document with current timestamps
   * @throws ValidationError If required fields are missing
   */
  buildWithTimestamps(): Document {
    if (!this._path) {
      throw new ValidationError('Document path is required');
    }

    if (!this._title) {
      throw new ValidationError('Document title is required');
    }

    if (this._content === undefined) {
      throw new ValidationError('Document content is required');
    }

    const docId = this._id ? this._id.asStr() : ValidatedDocumentId.new().asStr();

    // Calculate content size
    let size: number;
    let contentForDoc: string;

    if (typeof this._content === 'string') {
      size = new TextEncoder().encode(this._content).length;
      contentForDoc = this._content;
    } else {
      size = this._content.length;
      contentForDoc = new TextDecoder().decode(new Uint8Array(this._content));
    }

    const now = Math.floor(Date.now() / 1000);

    return {
      id: docId,
      path: this._path.asStr(),
      title: this._title.asStr(),
      content: contentForDoc,
      tags: this._tags,
      created_at: now,
      modified_at: now,
      size_bytes: size,
      metadata: this._metadata,
    };
  }
}

/**
 * Builder for creating search queries with validation.
 *
 * Provides a fluent interface for building complex queries
 * with proper validation and type safety.
 *
 * @example
 * ```typescript
 * const query = new QueryBuilder()
 *   .text("rust patterns")
 *   .limit(10)
 *   .offset(20)
 *   .build();
 * ```
 */
export class QueryBuilder {
  private _queryText?: string;
  private _limit?: number;
  private _offset: number = 0;
  private _semanticWeight?: number;
  private _filters: Record<string, any> = {};

  /**
   * Set the query text.
   *
   * @param query Search query text
   * @returns Self for method chaining
   * @throws ValidationError If query is invalid
   */
  text(query: string): QueryBuilder {
    validateSearchQuery(query);
    this._queryText = query;
    return this;
  }

  /**
   * Set the maximum number of results.
   *
   * @param limit Maximum number of results (must be positive)
   * @returns Self for method chaining
   * @throws ValidationError If limit is invalid
   */
  limit(limit: number): QueryBuilder {
    if (limit <= 0) {
      throw new ValidationError('Limit must be positive');
    }
    if (limit > 100000) {
      throw new ValidationError('Limit too large (max 100000)'); // Updated to match new limit from issue #248
    }
    this._limit = limit;
    return this;
  }

  /**
   * Set the number of results to skip.
   *
   * @param offset Number of results to skip (must be non-negative)
   * @returns Self for method chaining
   * @throws ValidationError If offset is invalid
   */
  offset(offset: number): QueryBuilder {
    if (offset < 0) {
      throw new ValidationError('Offset cannot be negative');
    }
    this._offset = offset;
    return this;
  }

  /**
   * Set the semantic search weight for hybrid search.
   *
   * @param weight Weight between 0.0 and 1.0
   * @returns Self for method chaining
   * @throws ValidationError If weight is invalid
   */
  semanticWeight(weight: number): QueryBuilder {
    if (weight < 0.0 || weight > 1.0) {
      throw new ValidationError('Semantic weight must be between 0.0 and 1.0');
    }
    this._semanticWeight = weight;
    return this;
  }

  /**
   * Add a filter to the query.
   *
   * @param key Filter key
   * @param value Filter value
   * @returns Self for method chaining
   */
  addFilter(key: string, value: any): QueryBuilder {
    this._filters[key] = value;
    return this;
  }

  /**
   * Add a tag filter.
   *
   * @param tag Tag to filter by
   * @returns Self for method chaining
   * @throws ValidationError If tag is invalid
   */
  tagFilter(tag: string): QueryBuilder {
    validateTag(tag);
    return this.addFilter('tag', tag);
  }

  /**
   * Add a path filter.
   *
   * @param pathPattern Path pattern to filter by
   * @returns Self for method chaining
   */
  pathFilter(pathPattern: string): QueryBuilder {
    return this.addFilter('path', pathPattern);
  }

  /**
   * Build the query parameters.
   *
   * @returns Object of query parameters
   * @throws ValidationError If required fields are missing
   */
  build(): Record<string, any> {
    if (!this._queryText) {
      throw new ValidationError('Query text is required');
    }

    const params: Record<string, any> = { q: this._queryText };

    if (this._limit !== undefined) {
      params.limit = this._limit;
    }

    if (this._offset > 0) {
      params.offset = this._offset;
    }

    if (this._semanticWeight !== undefined) {
      params.semantic_weight = this._semanticWeight;
    }

    // Add filters
    Object.assign(params, this._filters);

    return params;
  }

  /**
   * Build query data for semantic search endpoint.
   *
   * @returns Object suitable for semantic search POST body
   * @throws ValidationError If required fields are missing
   */
  buildForSemantic(): Record<string, any> {
    if (!this._queryText) {
      throw new ValidationError('Query text is required');
    }

    const data: Record<string, any> = { query: this._queryText };

    if (this._limit !== undefined) {
      data.limit = this._limit;
    }

    if (this._offset > 0) {
      data.offset = this._offset;
    }

    // Add filters
    Object.assign(data, this._filters);

    return data;
  }

  /**
   * Build query data for hybrid search endpoint.
   *
   * @returns Object suitable for hybrid search POST body
   */
  buildForHybrid(): Record<string, any> {
    const data = this.buildForSemantic();

    if (this._semanticWeight !== undefined) {
      data.semantic_weight = this._semanticWeight;
    }

    return data;
  }
}

/**
 * Builder for creating document updates with validation.
 *
 * Provides a fluent interface for safely updating documents
 * without overwriting fields unintentionally.
 *
 * @example
 * ```typescript
 * const updates = new UpdateBuilder()
 *   .title("Updated Title")
 *   .addTag("updated")
 *   .addMetadata("last_modified_by", "user123")
 *   .build();
 * ```
 */
export class UpdateBuilder {
  private _updates: Record<string, any> = {};
  private _tagsToAdd: string[] = [];
  private _tagsToRemove: string[] = [];
  private _metadataUpdates: Record<string, any> = {};

  /**
   * Update the document title.
   *
   * @param title New title (will be validated)
   * @returns Self for method chaining
   * @throws ValidationError If title is invalid
   */
  title(title: string | ValidatedTitle): UpdateBuilder {
    if (typeof title === 'string') {
      const validatedTitle = new ValidatedTitle(title);
      this._updates.title = validatedTitle.asStr();
    } else {
      this._updates.title = title.asStr();
    }
    return this;
  }

  /**
   * Update the document content.
   *
   * @param content New content
   * @returns Self for method chaining
   */
  content(content: string | number[]): UpdateBuilder {
    this._updates.content = content;
    return this;
  }

  /**
   * Add a tag (will be merged with existing tags).
   *
   * @param tag Tag to add (will be validated)
   * @returns Self for method chaining
   * @throws ValidationError If tag is invalid
   */
  addTag(tag: string): UpdateBuilder {
    validateTag(tag);
    if (!this._tagsToAdd.includes(tag)) {
      this._tagsToAdd.push(tag);
    }
    return this;
  }

  /**
   * Remove a tag.
   *
   * @param tag Tag to remove
   * @returns Self for method chaining
   */
  removeTag(tag: string): UpdateBuilder {
    if (!this._tagsToRemove.includes(tag)) {
      this._tagsToRemove.push(tag);
    }
    return this;
  }

  /**
   * Replace all tags.
   *
   * @param tags New tags list (each will be validated)
   * @returns Self for method chaining
   * @throws ValidationError If any tag is invalid
   */
  replaceTags(tags: string[]): UpdateBuilder {
    for (const tag of tags) {
      validateTag(tag);
    }
    this._updates.tags = [...tags];
    // Clear tag modifications since we're replacing
    this._tagsToAdd = [];
    this._tagsToRemove = [];
    return this;
  }

  /**
   * Add or update a metadata field.
   *
   * @param key Metadata key
   * @param value Metadata value
   * @returns Self for method chaining
   */
  addMetadata(key: string, value: any): UpdateBuilder {
    this._metadataUpdates[key] = value;
    return this;
  }

  /**
   * Remove a metadata field.
   *
   * @param key Metadata key to remove
   * @returns Self for method chaining
   */
  removeMetadata(key: string): UpdateBuilder {
    this._metadataUpdates[key] = null; // Use null to indicate removal
    return this;
  }

  /**
   * Build the update object.
   *
   * @returns Object of updates to apply
   */
  build(): Record<string, any> {
    const updates = { ...this._updates };

    // Handle tag updates
    if (this._tagsToAdd.length > 0 || this._tagsToRemove.length > 0) {
      if (!('tags' in updates)) {
        // Need current tags to modify them
        // This will require the caller to handle merging
        const tagOps: Record<string, string[]> = {};
        if (this._tagsToAdd.length > 0) {
          tagOps.add = this._tagsToAdd;
        }
        if (this._tagsToRemove.length > 0) {
          tagOps.remove = this._tagsToRemove;
        }
        updates._tag_operations = tagOps;
      }
    }

    // Handle metadata updates
    if (Object.keys(this._metadataUpdates).length > 0) {
      updates._metadata_operations = this._metadataUpdates;
    }

    return updates;
  }
}