use crate::mcp::tools::MCPToolHandler;
use crate::mcp::types::*;
use crate::types::{ValidatedDocumentId, ValidatedPath, ValidatedTag, ValidatedTitle};
use crate::{CoordinatedDeletionService, DocumentBuilder};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Document management tools that use coordinated deletion for proper index synchronization
pub struct CoordinatedDocumentTools {
    storage: Arc<Mutex<dyn crate::contracts::Storage>>,
    deletion_service: Arc<CoordinatedDeletionService>,
}

impl CoordinatedDocumentTools {
    pub fn new(
        storage: Arc<Mutex<dyn crate::contracts::Storage>>,
        deletion_service: Arc<CoordinatedDeletionService>,
    ) -> Self {
        Self {
            storage,
            deletion_service,
        }
    }
}

#[async_trait::async_trait]
impl MCPToolHandler for CoordinatedDocumentTools {
    async fn handle_call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        match method {
            "kotadb://document_create" => {
                let request: DocumentCreateRequest = serde_json::from_value(params)?;
                self.create_document(request).await
            }
            "kotadb://document_get" => {
                let request: DocumentGetRequest = serde_json::from_value(params)?;
                self.get_document(request).await
            }
            "kotadb://document_update" => {
                let request: DocumentUpdateRequest = serde_json::from_value(params)?;
                self.update_document(request).await
            }
            "kotadb://document_delete" => {
                let request: DocumentDeleteRequest = serde_json::from_value(params)?;
                self.delete_document(request).await
            }
            "kotadb://document_list" => {
                let request: DocumentListRequest = serde_json::from_value(params)?;
                self.list_documents(request).await
            }
            _ => Err(anyhow::anyhow!("Unknown document method: {}", method)),
        }
    }

    fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "kotadb://document_create".to_string(),
                description: "Create a new document in KotaDB with optional metadata and tags"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Unique path identifier for the document (e.g., '/docs/example.md')"
                        },
                        "title": {
                            "type": "string",
                            "description": "Optional human-readable title for the document"
                        },
                        "content": {
                            "type": "string",
                            "description": "The main content/body of the document"
                        },
                        "tags": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Optional array of tags for categorization"
                        },
                        "metadata": {
                            "type": "object",
                            "description": "Optional key-value metadata for the document"
                        }
                    },
                    "required": ["path", "content"]
                }),
            },
            ToolDefinition {
                name: "kotadb://document_get".to_string(),
                description: "Retrieve a document by its ID with full content and metadata"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The unique document ID to retrieve"
                        }
                    },
                    "required": ["id"]
                }),
            },
            ToolDefinition {
                name: "kotadb://document_update".to_string(),
                description: "Update an existing document's content, metadata, or tags".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The unique document ID to update"
                        },
                        "title": {
                            "type": "string",
                            "description": "New title for the document"
                        },
                        "content": {
                            "type": "string",
                            "description": "New content for the document"
                        },
                        "tags": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "New tags array (replaces existing tags)"
                        },
                        "metadata": {
                            "type": "object",
                            "description": "New metadata (merges with existing metadata)"
                        }
                    },
                    "required": ["id"]
                }),
            },
            ToolDefinition {
                name: "kotadb://document_delete".to_string(),
                description: "Delete a document by its ID (uses coordinated deletion for proper index synchronization)".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The unique document ID to delete"
                        }
                    },
                    "required": ["id"]
                }),
            },
            ToolDefinition {
                name: "kotadb://document_list".to_string(),
                description: "List documents with optional filtering and pagination".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of documents to return (default: 50)"
                        },
                        "offset": {
                            "type": "integer",
                            "description": "Number of documents to skip (default: 0)"
                        },
                        "tags": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Filter by tags (documents must have all specified tags)"
                        },
                        "path_prefix": {
                            "type": "string",
                            "description": "Filter by path prefix (e.g., '/docs/' for all docs)"
                        }
                    }
                }),
            },
        ]
    }
}

impl CoordinatedDocumentTools {
    async fn create_document(&self, request: DocumentCreateRequest) -> Result<serde_json::Value> {
        let start_time = std::time::Instant::now();

        // Validate the path
        let validated_path = ValidatedPath::new(&request.path)
            .map_err(|e| anyhow::anyhow!("Invalid path: {}", e))?;

        // Build the document using the component library
        let mut doc_builder = DocumentBuilder::new()
            .path(validated_path.as_str())?
            .content(request.content.as_bytes());

        if let Some(title) = &request.title {
            doc_builder = doc_builder.title(title)?;
        }

        if let Some(tags) = &request.tags {
            for tag in tags {
                doc_builder = doc_builder.tag(tag)?;
            }
        }

        // Note: Metadata is not supported in the current Document structure

        let document = doc_builder.build()?;
        let doc_id = document.id;

        // Store the document using the wrapped storage
        let storage = self.storage.clone();
        let mut storage_guard = storage.lock().await;
        storage_guard.insert(document).await?;

        let response = DocumentCreateResponse {
            id: doc_id.to_string(),
            path: request.path,
            created_at: chrono::Utc::now(),
        };

        tracing::info!(
            "Document created via MCP with coordinated deletion support: {} in {}ms",
            response.id,
            start_time.elapsed().as_millis()
        );

        Ok(serde_json::json!({
            "success": true,
            "document": response
        }))
    }

    async fn get_document(&self, request: DocumentGetRequest) -> Result<serde_json::Value> {
        let start_time = std::time::Instant::now();

        let doc_id = ValidatedDocumentId::parse(&request.id)
            .map_err(|e| anyhow::anyhow!("Invalid document ID: {}", e))?;

        let storage = self.storage.clone();
        let storage_guard = storage.lock().await;

        let document = storage_guard
            .get(&doc_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Document not found: {}", request.id))?;

        let response = DocumentGetResponse {
            id: document.id.to_string(),
            path: document.path.to_string(),
            title: Some(document.title.as_str().to_string()),
            content: String::from_utf8_lossy(&document.content).to_string(),
            tags: document
                .tags
                .into_iter()
                .map(|tag| tag.as_str().to_string())
                .collect(),
            metadata: HashMap::new(), // TODO: Add metadata support
            created_at: document.created_at,
            updated_at: document.updated_at,
        };

        tracing::debug!(
            "Document retrieved via MCP: {} in {}ms",
            response.id,
            start_time.elapsed().as_millis()
        );

        Ok(serde_json::json!({
            "success": true,
            "document": response
        }))
    }

    async fn update_document(&self, request: DocumentUpdateRequest) -> Result<serde_json::Value> {
        let start_time = std::time::Instant::now();

        let doc_id = ValidatedDocumentId::parse(&request.id)
            .map_err(|e| anyhow::anyhow!("Invalid document ID: {}", e))?;

        let storage = self.storage.clone();
        let mut storage_guard = storage.lock().await;

        // Get existing document
        let mut document = storage_guard
            .get(&doc_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Document not found: {}", request.id))?;

        // Update fields if provided
        if let Some(title) = request.title {
            document.title = ValidatedTitle::new(&title)?;
        }
        if let Some(content) = request.content {
            document.content = content.into_bytes();
        }
        if let Some(tags) = request.tags {
            document.tags = tags
                .into_iter()
                .map(|tag| ValidatedTag::new(&tag))
                .collect::<Result<Vec<_>, _>>()?;
        }
        // TODO: Add metadata support to Document struct

        document.updated_at = chrono::Utc::now();

        // Update in storage
        storage_guard.update(document.clone()).await?;

        let response = DocumentUpdateResponse {
            id: document.id.to_string(),
            path: document.path.to_string(),
            title: document.title.to_string(),
            content: String::from_utf8_lossy(&document.content).to_string(),
            tags: document.tags.iter().map(|tag| tag.to_string()).collect(),
            created_at: document.created_at,
            updated_at: document.updated_at,
        };

        tracing::info!(
            "Document updated via MCP: {} in {}ms",
            response.id,
            start_time.elapsed().as_millis()
        );

        Ok(serde_json::json!({
            "success": true,
            "document": response
        }))
    }

    async fn delete_document(&self, request: DocumentDeleteRequest) -> Result<serde_json::Value> {
        let start_time = std::time::Instant::now();

        let doc_id = ValidatedDocumentId::parse(&request.id)
            .map_err(|e| anyhow::anyhow!("Invalid document ID: {}", e))?;

        // Use coordinated deletion service for proper index synchronization
        let deleted = self.deletion_service.delete_document(&doc_id).await?;

        let response = DocumentDeleteResponse {
            id: request.id,
            deleted,
        };

        tracing::info!(
            "Document deleted via MCP using coordinated deletion: {} (success: {}) in {}ms",
            response.id,
            response.deleted,
            start_time.elapsed().as_millis()
        );

        Ok(serde_json::json!({
            "success": response.deleted,
            "id": response.id,
            "message": if response.deleted {
                "Document successfully deleted from storage and all indices"
            } else {
                "Document not found"
            }
        }))
    }

    async fn list_documents(&self, request: DocumentListRequest) -> Result<serde_json::Value> {
        let start_time = std::time::Instant::now();

        let storage = self.storage.clone();
        let storage_guard = storage.lock().await;

        // For now, implement a simple list - in production this would use pagination/filtering
        let all_documents = storage_guard.list_all().await?;

        let mut filtered_docs = all_documents;

        // Apply filters
        if let Some(tags) = &request.tags {
            filtered_docs.retain(|doc| {
                tags.iter()
                    .all(|tag| doc.tags.iter().any(|doc_tag| doc_tag.as_str() == tag))
            });
        }

        if let Some(prefix) = &request.path_prefix {
            filtered_docs.retain(|doc| doc.path.to_string().starts_with(prefix));
        }

        // Apply pagination
        let offset = request.offset.unwrap_or(0);
        let limit = request.limit.unwrap_or(50);

        let total_count = filtered_docs.len();
        let documents: Vec<_> = filtered_docs
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|doc| DocumentListItem {
                id: doc.id.to_string(),
                path: doc.path.to_string(),
                title: Some(doc.title.as_str().to_string()),
                tags: doc
                    .tags
                    .into_iter()
                    .map(|tag| tag.as_str().to_string())
                    .collect(),
                created_at: doc.created_at,
                updated_at: doc.updated_at,
            })
            .collect();

        let response = DocumentListResponse {
            documents,
            total_count,
            offset,
            limit,
        };

        tracing::debug!(
            "Document list via MCP: {} documents in {}ms",
            response.documents.len(),
            start_time.elapsed().as_millis()
        );

        Ok(serde_json::json!({
            "success": true,
            "documents": response.documents,
            "total": response.total_count,
            "offset": response.offset,
            "limit": response.limit
        }))
    }
}

// Additional types for document operations
#[derive(Debug, Clone, serde::Deserialize)]
struct DocumentUpdateRequest {
    id: String,
    title: Option<String>,
    content: Option<String>,
    tags: Option<Vec<String>>,
    #[allow(dead_code)] // Metadata field for future document metadata features
    metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct DocumentUpdateResponse {
    id: String,
    path: String,
    title: String,
    content: String,
    tags: Vec<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct DocumentDeleteRequest {
    id: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct DocumentDeleteResponse {
    id: String,
    deleted: bool,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct DocumentListRequest {
    limit: Option<usize>,
    offset: Option<usize>,
    tags: Option<Vec<String>>,
    path_prefix: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct DocumentListResponse {
    documents: Vec<DocumentListItem>,
    total_count: usize,
    offset: usize,
    limit: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
struct DocumentListItem {
    id: String,
    path: String,
    title: Option<String>,
    tags: Vec<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}
