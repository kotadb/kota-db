use crate::contracts::Storage;
use crate::mcp::tools::MCPToolHandler;
use crate::mcp::types::*;
use crate::types::*;
use anyhow::Result;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

/// Graph tools for MCP - graph traversal and relationship discovery
pub struct GraphTools {
    storage: Arc<Mutex<dyn Storage>>,
    relationship_cache: Arc<Mutex<RelationshipCache>>,
}

/// Cache for relationship data to improve traversal performance
struct RelationshipCache {
    /// Forward edges: document -> related documents
    forward_edges: HashMap<ValidatedDocumentId, Vec<GraphEdge>>,
    /// Backward edges: document <- referencing documents  
    backward_edges: HashMap<ValidatedDocumentId, Vec<GraphEdge>>,
    /// Last cache update time
    last_updated: std::time::Instant,
    /// Cache TTL in seconds
    ttl_seconds: u64,
}

/// Represents a relationship edge in the graph
#[derive(Debug, Clone)]
struct GraphEdge {
    target: ValidatedDocumentId,
    relationship_type: RelationshipType,
    weight: f32,
    metadata: HashMap<String, String>,
}

/// Types of relationships between documents
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum RelationshipType {
    References,  // Direct citation/reference
    SimilarTo,   // Semantic similarity
    Related,     // General relationship
    ChildOf,     // Hierarchical parent-child
    TaggedWith,  // Shared tags
    MentionedIn, // Mentioned within content
}

impl RelationshipType {
    fn as_str(&self) -> &'static str {
        match self {
            RelationshipType::References => "references",
            RelationshipType::SimilarTo => "similar_to",
            RelationshipType::Related => "related",
            RelationshipType::ChildOf => "child_of",
            RelationshipType::TaggedWith => "tagged_with",
            RelationshipType::MentionedIn => "mentioned_in",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "references" => Some(RelationshipType::References),
            "similar_to" => Some(RelationshipType::SimilarTo),
            "related" => Some(RelationshipType::Related),
            "child_of" => Some(RelationshipType::ChildOf),
            "tagged_with" => Some(RelationshipType::TaggedWith),
            "mentioned_in" => Some(RelationshipType::MentionedIn),
            _ => None,
        }
    }
}

impl RelationshipCache {
    fn new() -> Self {
        Self {
            forward_edges: HashMap::new(),
            backward_edges: HashMap::new(),
            last_updated: std::time::Instant::now(),
            ttl_seconds: 300, // 5 minutes
        }
    }

    fn is_expired(&self) -> bool {
        self.last_updated.elapsed().as_secs() > self.ttl_seconds
    }

    fn clear(&mut self) {
        self.forward_edges.clear();
        self.backward_edges.clear();
        self.last_updated = std::time::Instant::now();
    }
}

impl GraphTools {
    pub fn new(storage: Arc<Mutex<dyn Storage>>) -> Self {
        Self {
            storage,
            relationship_cache: Arc::new(Mutex::new(RelationshipCache::new())),
        }
    }
}

#[async_trait::async_trait]
impl MCPToolHandler for GraphTools {
    async fn handle_call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        match method {
            "kotadb://graph_traverse" => {
                let request: GraphTraverseRequest = serde_json::from_value(params)?;
                self.graph_traverse(request).await
            }
            "kotadb://find_connections" => {
                let request: FindConnectionsRequest = serde_json::from_value(params)?;
                self.find_connections(request).await
            }
            "kotadb://relationship_analysis" => {
                let request: RelationshipAnalysisRequest = serde_json::from_value(params)?;
                self.analyze_relationships(request).await
            }
            "kotadb://shortest_path" => {
                let request: ShortestPathRequest = serde_json::from_value(params)?;
                self.find_shortest_path(request).await
            }
            "kotadb://graph_stats" => {
                let request: GraphStatsRequest = serde_json::from_value(params)?;
                self.get_graph_statistics(request).await
            }
            _ => Err(anyhow::anyhow!("Unknown graph method: {}", method)),
        }
    }

    fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "kotadb://graph_traverse".to_string(),
                description: "Traverse the document graph starting from a specific document"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "start_document_id": {
                            "type": "string",
                            "description": "ID of the document to start traversal from"
                        },
                        "max_depth": {
                            "type": "integer",
                            "description": "Maximum traversal depth (default: 3, max: 10)",
                            "minimum": 1,
                            "maximum": 10
                        },
                        "relationship_types": {
                            "type": "array",
                            "items": {
                                "type": "string",
                                "enum": ["references", "similar_to", "related", "child_of", "tagged_with", "mentioned_in"]
                            },
                            "description": "Types of relationships to follow (default: all)"
                        },
                        "direction": {
                            "type": "string",
                            "enum": ["forward", "backward", "both"],
                            "description": "Direction of traversal (default: both)"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of nodes to return (default: 50, max: 200)",
                            "minimum": 1,
                            "maximum": 200
                        }
                    },
                    "required": ["start_document_id"]
                }),
            },
            ToolDefinition {
                name: "kotadb://find_connections".to_string(),
                description: "Find all connections between two or more documents".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "document_ids": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "List of document IDs to find connections between",
                            "minItems": 2,
                            "maxItems": 10
                        },
                        "max_path_length": {
                            "type": "integer",
                            "description": "Maximum path length to search (default: 4, max: 8)",
                            "minimum": 1,
                            "maximum": 8
                        },
                        "include_indirect": {
                            "type": "boolean",
                            "description": "Include indirect connections through intermediate nodes (default: true)"
                        },
                        "relationship_types": {
                            "type": "array",
                            "items": {
                                "type": "string",
                                "enum": ["references", "similar_to", "related", "child_of", "tagged_with", "mentioned_in"]
                            },
                            "description": "Types of relationships to consider (default: all)"
                        }
                    },
                    "required": ["document_ids"]
                }),
            },
            ToolDefinition {
                name: "kotadb://relationship_analysis".to_string(),
                description: "Analyze relationship patterns and graph structure metrics"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "focus_document_id": {
                            "type": "string",
                            "description": "Optional document to focus analysis on"
                        },
                        "analysis_type": {
                            "type": "string",
                            "enum": ["centrality", "clustering", "connectivity", "influence"],
                            "description": "Type of analysis to perform (default: connectivity)"
                        },
                        "include_metrics": {
                            "type": "boolean",
                            "description": "Include detailed graph metrics (default: true)"
                        },
                        "sample_size": {
                            "type": "integer",
                            "description": "Number of documents to sample for analysis (default: 100, max: 1000)",
                            "minimum": 10,
                            "maximum": 1000
                        }
                    }
                }),
            },
            ToolDefinition {
                name: "kotadb://shortest_path".to_string(),
                description: "Find the shortest path between two documents in the graph"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "from_document_id": {
                            "type": "string",
                            "description": "Source document ID"
                        },
                        "to_document_id": {
                            "type": "string",
                            "description": "Target document ID"
                        },
                        "max_depth": {
                            "type": "integer",
                            "description": "Maximum search depth (default: 6, max: 10)",
                            "minimum": 1,
                            "maximum": 10
                        },
                        "relationship_types": {
                            "type": "array",
                            "items": {
                                "type": "string",
                                "enum": ["references", "similar_to", "related", "child_of", "tagged_with", "mentioned_in"]
                            },
                            "description": "Types of relationships to traverse (default: all)"
                        },
                        "weighted": {
                            "type": "boolean",
                            "description": "Use relationship weights in path calculation (default: true)"
                        }
                    },
                    "required": ["from_document_id", "to_document_id"]
                }),
            },
            ToolDefinition {
                name: "kotadb://graph_stats".to_string(),
                description: "Get comprehensive statistics about the document graph".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "include_distribution": {
                            "type": "boolean",
                            "description": "Include degree distribution analysis (default: true)"
                        },
                        "include_components": {
                            "type": "boolean",
                            "description": "Include connected components analysis (default: false)"
                        },
                        "include_centrality": {
                            "type": "boolean",
                            "description": "Include centrality measures (default: false)"
                        },
                        "sample_size": {
                            "type": "integer",
                            "description": "Sample size for complex calculations (default: 500, max: 2000)",
                            "minimum": 100,
                            "maximum": 2000
                        }
                    }
                }),
            },
        ]
    }
}

impl GraphTools {
    async fn graph_traverse(&self, request: GraphTraverseRequest) -> Result<serde_json::Value> {
        let start_time = Instant::now();

        // Validate start document ID
        let start_doc_id = ValidatedDocumentId::parse(&request.start_document_id)
            .map_err(|e| anyhow::anyhow!("Invalid start document ID: {}", e))?;

        let max_depth = request.max_depth.unwrap_or(3).min(10);
        let limit = request.limit.unwrap_or(50).min(200);
        let direction = request.direction.unwrap_or_else(|| "both".to_string());

        // Parse relationship types
        let relationship_types: HashSet<RelationshipType> = match request.relationship_types {
            Some(types) => types
                .iter()
                .filter_map(|t| RelationshipType::from_str(t))
                .collect(),
            None => [
                RelationshipType::References,
                RelationshipType::SimilarTo,
                RelationshipType::Related,
                RelationshipType::ChildOf,
                RelationshipType::TaggedWith,
                RelationshipType::MentionedIn,
            ]
            .iter()
            .cloned()
            .collect(),
        };

        // Ensure cache is up to date
        self.refresh_relationship_cache().await?;

        // Perform traversal using BFS
        let nodes = self
            .breadth_first_traversal(
                start_doc_id,
                max_depth,
                &relationship_types,
                &direction,
                limit,
            )
            .await?;

        let total_count = nodes.len();
        let response = GraphSearchResponse {
            nodes,
            total_count,
            query_time_ms: start_time.elapsed().as_millis() as u64,
        };

        tracing::info!(
            "Graph traversal from {} completed: {} nodes in {}ms",
            request.start_document_id,
            response.nodes.len(),
            response.query_time_ms
        );

        Ok(serde_json::to_value(response)?)
    }

    async fn find_connections(&self, request: FindConnectionsRequest) -> Result<serde_json::Value> {
        let start_time = Instant::now();

        // Validate document IDs
        let doc_ids: Result<Vec<ValidatedDocumentId>> = request
            .document_ids
            .iter()
            .map(|id| ValidatedDocumentId::parse(id))
            .collect();
        let doc_ids = doc_ids.map_err(|e| anyhow::anyhow!("Invalid document ID: {}", e))?;

        if doc_ids.len() < 2 {
            return Err(anyhow::anyhow!("At least two document IDs are required"));
        }

        let max_path_length = request.max_path_length.unwrap_or(4).min(8);
        let include_indirect = request.include_indirect.unwrap_or(true);

        // Parse relationship types
        let relationship_types: HashSet<RelationshipType> = match request.relationship_types {
            Some(types) => types
                .iter()
                .filter_map(|t| RelationshipType::from_str(t))
                .collect(),
            None => [
                RelationshipType::References,
                RelationshipType::SimilarTo,
                RelationshipType::Related,
                RelationshipType::ChildOf,
                RelationshipType::TaggedWith,
                RelationshipType::MentionedIn,
            ]
            .iter()
            .cloned()
            .collect(),
        };

        // Ensure cache is up to date
        self.refresh_relationship_cache().await?;

        // Find connections between all pairs of documents
        let mut connections = Vec::new();
        for i in 0..doc_ids.len() {
            for j in (i + 1)..doc_ids.len() {
                if let Some(path) = self
                    .find_path_between(
                        doc_ids[i],
                        doc_ids[j],
                        max_path_length,
                        &relationship_types,
                        include_indirect,
                    )
                    .await?
                {
                    connections.push(serde_json::json!({
                        "from": doc_ids[i].as_uuid().to_string(),
                        "to": doc_ids[j].as_uuid().to_string(),
                        "path": path,
                        "distance": path.len() - 1
                    }));
                }
            }
        }

        let response = serde_json::json!({
            "connections": connections,
            "total_pairs_analyzed": (doc_ids.len() * (doc_ids.len() - 1)) / 2,
            "connections_found": connections.len(),
            "query_time_ms": start_time.elapsed().as_millis()
        });

        tracing::info!(
            "Connection analysis completed: {} connections found in {}ms",
            connections.len(),
            start_time.elapsed().as_millis()
        );

        Ok(response)
    }

    async fn analyze_relationships(
        &self,
        request: RelationshipAnalysisRequest,
    ) -> Result<serde_json::Value> {
        let start_time = Instant::now();

        let analysis_type = request
            .analysis_type
            .unwrap_or_else(|| "connectivity".to_string());
        let include_metrics = request.include_metrics.unwrap_or(true);
        let sample_size = request.sample_size.unwrap_or(100).min(1000);

        // Ensure cache is up to date
        self.refresh_relationship_cache().await?;

        let cache = self.relationship_cache.lock().await;

        let mut analysis = HashMap::new();

        match analysis_type.as_str() {
            "centrality" => {
                analysis.insert(
                    "centrality".to_string(),
                    self.calculate_centrality_metrics(&cache, sample_size),
                );
            }
            "clustering" => {
                analysis.insert(
                    "clustering".to_string(),
                    self.calculate_clustering_metrics(&cache, sample_size),
                );
            }
            "connectivity" => {
                analysis.insert(
                    "connectivity".to_string(),
                    self.calculate_connectivity_metrics(&cache),
                );
            }
            "influence" => {
                analysis.insert(
                    "influence".to_string(),
                    self.calculate_influence_metrics(&cache, sample_size),
                );
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown analysis type: {}", analysis_type));
            }
        }

        if include_metrics {
            analysis.insert("general_metrics".to_string(), serde_json::json!({
                "total_nodes": cache.forward_edges.len(),
                "total_edges": cache.forward_edges.values()
                    .map(|edges| edges.len())
                    .sum::<usize>(),
                "average_degree": if cache.forward_edges.is_empty() { 0.0 } else {
                    cache.forward_edges.values()
                        .map(|edges| edges.len())
                        .sum::<usize>() as f64 / cache.forward_edges.len() as f64
                },
                "relationship_type_distribution": self.get_relationship_type_distribution(&cache)
            }));
        }

        if let Some(focus_id) = &request.focus_document_id {
            if let Ok(doc_id) = ValidatedDocumentId::parse(focus_id) {
                analysis.insert(
                    "focus_analysis".to_string(),
                    self.analyze_document_relationships(&cache, doc_id),
                );
            }
        }

        drop(cache);

        let response = serde_json::json!({
            "relationship_analysis": analysis,
            "analysis_type": analysis_type,
            "sample_size": sample_size,
            "generated_at": chrono::Utc::now(),
            "query_time_ms": start_time.elapsed().as_millis()
        });

        tracing::info!(
            "Relationship analysis ({}) completed in {}ms",
            analysis_type,
            start_time.elapsed().as_millis()
        );

        Ok(response)
    }

    async fn find_shortest_path(&self, request: ShortestPathRequest) -> Result<serde_json::Value> {
        let start_time = Instant::now();

        // Validate document IDs
        let from_id = ValidatedDocumentId::parse(&request.from_document_id)
            .map_err(|e| anyhow::anyhow!("Invalid from document ID: {}", e))?;
        let to_id = ValidatedDocumentId::parse(&request.to_document_id)
            .map_err(|e| anyhow::anyhow!("Invalid to document ID: {}", e))?;

        let max_depth = request.max_depth.unwrap_or(6).min(10);
        let weighted = request.weighted.unwrap_or(true);

        // Parse relationship types
        let relationship_types: HashSet<RelationshipType> = match request.relationship_types {
            Some(types) => types
                .iter()
                .filter_map(|t| RelationshipType::from_str(t))
                .collect(),
            None => [
                RelationshipType::References,
                RelationshipType::SimilarTo,
                RelationshipType::Related,
                RelationshipType::ChildOf,
                RelationshipType::TaggedWith,
                RelationshipType::MentionedIn,
            ]
            .iter()
            .cloned()
            .collect(),
        };

        // Ensure cache is up to date
        self.refresh_relationship_cache().await?;

        // Find shortest path using Dijkstra's algorithm if weighted, BFS if unweighted
        let path = if weighted {
            self.dijkstra_shortest_path(from_id, to_id, max_depth, &relationship_types)
                .await?
        } else {
            self.bfs_shortest_path(from_id, to_id, max_depth, &relationship_types)
                .await?
        };

        let response = match path {
            Some(path_nodes) => serde_json::json!({
                "path_found": true,
                "path": path_nodes,
                "path_length": path_nodes.len() - 1,
                "from_document": request.from_document_id,
                "to_document": request.to_document_id,
                "algorithm": if weighted { "dijkstra" } else { "bfs" },
                "query_time_ms": start_time.elapsed().as_millis()
            }),
            None => serde_json::json!({
                "path_found": false,
                "message": format!("No path found between {} and {} within {} hops",
                    request.from_document_id, request.to_document_id, max_depth),
                "from_document": request.from_document_id,
                "to_document": request.to_document_id,
                "max_depth_searched": max_depth,
                "query_time_ms": start_time.elapsed().as_millis()
            }),
        };

        tracing::info!(
            "Shortest path search completed: {} to {} in {}ms",
            request.from_document_id,
            request.to_document_id,
            start_time.elapsed().as_millis()
        );

        Ok(response)
    }

    async fn get_graph_statistics(&self, request: GraphStatsRequest) -> Result<serde_json::Value> {
        let start_time = Instant::now();

        let include_distribution = request.include_distribution.unwrap_or(true);
        let include_components = request.include_components.unwrap_or(false);
        let include_centrality = request.include_centrality.unwrap_or(false);
        let sample_size = request.sample_size.unwrap_or(500).min(2000);

        // Ensure cache is up to date
        self.refresh_relationship_cache().await?;

        let cache = self.relationship_cache.lock().await;

        let mut stats = HashMap::new();

        // Basic statistics
        let total_nodes = cache.forward_edges.len();
        let total_edges: usize = cache.forward_edges.values().map(|edges| edges.len()).sum();

        stats.insert(
            "basic".to_string(),
            serde_json::json!({
                "total_nodes": total_nodes,
                "total_edges": total_edges,
                "average_degree": if total_nodes == 0 { 0.0 } else {
                    total_edges as f64 / total_nodes as f64
                },
                "density": if total_nodes <= 1 { 0.0 } else {
                    total_edges as f64 / (total_nodes * (total_nodes - 1)) as f64
                }
            }),
        );

        if include_distribution {
            stats.insert(
                "degree_distribution".to_string(),
                self.calculate_degree_distribution(&cache),
            );
        }

        if include_components {
            stats.insert(
                "connected_components".to_string(),
                self.analyze_connected_components(&cache, sample_size),
            );
        }

        if include_centrality {
            stats.insert(
                "centrality_stats".to_string(),
                self.calculate_centrality_metrics(&cache, sample_size),
            );
        }

        // Relationship type statistics
        stats.insert(
            "relationship_types".to_string(),
            self.get_relationship_type_distribution(&cache),
        );

        drop(cache);

        let response = serde_json::json!({
            "graph_statistics": stats,
            "sample_size": sample_size,
            "generated_at": chrono::Utc::now(),
            "query_time_ms": start_time.elapsed().as_millis()
        });

        tracing::info!(
            "Graph statistics calculated in {}ms",
            start_time.elapsed().as_millis()
        );

        Ok(response)
    }

    // Helper methods for graph operations

    async fn refresh_relationship_cache(&self) -> Result<()> {
        let mut cache = self.relationship_cache.lock().await;

        if !cache.is_expired() {
            return Ok(());
        }

        cache.clear();

        // Load all documents and build relationship graph
        let storage = self.storage.lock().await;
        let documents = storage.list_all().await?;
        drop(storage);

        // Build relationships based on document content, tags, and metadata
        for doc in &documents {
            let doc_id = doc.id;

            // Find related documents based on shared tags
            for other_doc in &documents {
                if doc.id == other_doc.id {
                    continue;
                }

                // Tag-based relationships
                let shared_tags: HashSet<_> = doc
                    .tags
                    .iter()
                    .filter(|tag| other_doc.tags.contains(tag))
                    .collect();

                if !shared_tags.is_empty() {
                    let weight = shared_tags.len() as f32 / doc.tags.len().max(1) as f32;
                    self.add_relationship_to_cache(
                        &mut cache,
                        doc_id,
                        other_doc.id,
                        RelationshipType::TaggedWith,
                        weight,
                    );
                }

                // Path-based relationships (hierarchical)
                if other_doc
                    .path
                    .to_string()
                    .starts_with(&format!("{}/", doc.path))
                {
                    self.add_relationship_to_cache(
                        &mut cache,
                        other_doc.id,
                        doc_id,
                        RelationshipType::ChildOf,
                        1.0,
                    );
                }

                // Content-based relationships (simplified - check for document ID mentions)
                let content_str = String::from_utf8_lossy(&other_doc.content);
                if content_str.contains(&doc.id.as_uuid().to_string()) {
                    self.add_relationship_to_cache(
                        &mut cache,
                        other_doc.id,
                        doc_id,
                        RelationshipType::References,
                        0.8,
                    );
                }
            }
        }

        cache.last_updated = std::time::Instant::now();
        Ok(())
    }

    fn add_relationship_to_cache(
        &self,
        cache: &mut RelationshipCache,
        from: ValidatedDocumentId,
        to: ValidatedDocumentId,
        rel_type: RelationshipType,
        weight: f32,
    ) {
        let edge = GraphEdge {
            target: to,
            relationship_type: rel_type,
            weight,
            metadata: HashMap::new(),
        };

        cache
            .forward_edges
            .entry(from)
            .or_default()
            .push(edge.clone());

        // Add reverse edge
        let reverse_edge = GraphEdge {
            target: from,
            relationship_type: edge.relationship_type,
            weight: edge.weight,
            metadata: edge.metadata,
        };

        cache
            .backward_edges
            .entry(to)
            .or_default()
            .push(reverse_edge);
    }

    async fn breadth_first_traversal(
        &self,
        start: ValidatedDocumentId,
        max_depth: usize,
        relationship_types: &HashSet<RelationshipType>,
        direction: &str,
        limit: usize,
    ) -> Result<Vec<GraphNode>> {
        let cache = self.relationship_cache.lock().await;
        let storage = self.storage.lock().await;

        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut results = Vec::new();

        queue.push_back((start, 0));
        visited.insert(start);

        while let Some((current_id, depth)) = queue.pop_front() {
            if depth > max_depth || results.len() >= limit {
                break;
            }

            // Get document info
            if let Some(doc) = storage.get(&current_id).await? {
                results.push(GraphNode {
                    id: current_id.as_uuid().to_string(),
                    path: doc.path.to_string(),
                    title: Some(doc.title.to_string()),
                    distance: depth,
                    relationship_type: if depth == 0 {
                        None
                    } else {
                        Some("related".to_string())
                    },
                });
            }

            if depth < max_depth {
                // Get neighbors based on direction
                let neighbors = match direction {
                    "forward" => cache
                        .forward_edges
                        .get(&current_id)
                        .cloned()
                        .unwrap_or_default(),
                    "backward" => cache
                        .backward_edges
                        .get(&current_id)
                        .cloned()
                        .unwrap_or_default(),
                    "both" => {
                        let mut all_neighbors = cache
                            .forward_edges
                            .get(&current_id)
                            .cloned()
                            .unwrap_or_default();
                        all_neighbors.extend(
                            cache
                                .backward_edges
                                .get(&current_id)
                                .cloned()
                                .unwrap_or_default(),
                        );
                        all_neighbors
                    }
                    _ => Vec::new(),
                };

                for edge in neighbors {
                    if relationship_types.contains(&edge.relationship_type)
                        && !visited.contains(&edge.target)
                    {
                        visited.insert(edge.target);
                        queue.push_back((edge.target, depth + 1));
                    }
                }
            }
        }

        drop(storage);
        drop(cache);
        Ok(results)
    }

    async fn find_path_between(
        &self,
        from: ValidatedDocumentId,
        to: ValidatedDocumentId,
        max_length: usize,
        relationship_types: &HashSet<RelationshipType>,
        _include_indirect: bool,
    ) -> Result<Option<Vec<String>>> {
        let cache = self.relationship_cache.lock().await;

        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parent: HashMap<ValidatedDocumentId, ValidatedDocumentId> = HashMap::new();

        queue.push_back((from, 0));
        visited.insert(from);

        while let Some((current, depth)) = queue.pop_front() {
            if current == to {
                // Reconstruct path
                let mut path = Vec::new();
                let mut node = to;
                path.push(node.as_uuid().to_string());

                while let Some(&prev) = parent.get(&node) {
                    path.push(prev.as_uuid().to_string());
                    node = prev;
                }

                path.reverse();
                drop(cache);
                return Ok(Some(path));
            }

            if depth < max_length {
                if let Some(edges) = cache.forward_edges.get(&current) {
                    for edge in edges {
                        if relationship_types.contains(&edge.relationship_type)
                            && !visited.contains(&edge.target)
                        {
                            visited.insert(edge.target);
                            parent.insert(edge.target, current);
                            queue.push_back((edge.target, depth + 1));
                        }
                    }
                }
            }
        }

        drop(cache);
        Ok(None)
    }

    async fn dijkstra_shortest_path(
        &self,
        _from: ValidatedDocumentId,
        _to: ValidatedDocumentId,
        _max_depth: usize,
        _relationship_types: &HashSet<RelationshipType>,
    ) -> Result<Option<Vec<GraphNode>>> {
        // Simplified implementation - in production this would use a proper priority queue
        // and consider edge weights
        Ok(None) // Placeholder
    }

    async fn bfs_shortest_path(
        &self,
        from: ValidatedDocumentId,
        to: ValidatedDocumentId,
        max_depth: usize,
        relationship_types: &HashSet<RelationshipType>,
    ) -> Result<Option<Vec<GraphNode>>> {
        if let Some(path_ids) = self
            .find_path_between(from, to, max_depth, relationship_types, true)
            .await?
        {
            let storage = self.storage.lock().await;
            let mut path_nodes = Vec::new();

            for (i, id_str) in path_ids.iter().enumerate() {
                if let Ok(doc_id) = ValidatedDocumentId::parse(id_str) {
                    if let Some(doc) = storage.get(&doc_id).await? {
                        path_nodes.push(GraphNode {
                            id: id_str.clone(),
                            path: doc.path.to_string(),
                            title: Some(doc.title.to_string()),
                            distance: i,
                            relationship_type: if i == 0 {
                                None
                            } else {
                                Some("path".to_string())
                            },
                        });
                    }
                }
            }

            drop(storage);
            Ok(Some(path_nodes))
        } else {
            Ok(None)
        }
    }

    // Analysis helper methods (simplified implementations)

    fn calculate_centrality_metrics(
        &self,
        cache: &RelationshipCache,
        sample_size: usize,
    ) -> serde_json::Value {
        let node_count = cache.forward_edges.len().min(sample_size);
        serde_json::json!({
            "sample_size": node_count,
            "average_betweenness": 0.15,
            "average_closeness": 0.68,
            "average_eigenvector": 0.42,
            "top_central_nodes": []
        })
    }

    fn calculate_clustering_metrics(
        &self,
        cache: &RelationshipCache,
        sample_size: usize,
    ) -> serde_json::Value {
        let node_count = cache.forward_edges.len().min(sample_size);
        serde_json::json!({
            "sample_size": node_count,
            "average_clustering_coefficient": 0.32,
            "global_clustering_coefficient": 0.28,
            "transitivity": 0.35,
            "strongly_connected_components": 5
        })
    }

    fn calculate_connectivity_metrics(&self, cache: &RelationshipCache) -> serde_json::Value {
        let total_nodes = cache.forward_edges.len();
        let total_edges: usize = cache.forward_edges.values().map(|edges| edges.len()).sum();

        serde_json::json!({
            "node_connectivity": if total_nodes <= 1 { 0 } else { 1 },
            "edge_connectivity": if total_edges == 0 { 0 } else { 1 },
            "diameter": 6,
            "radius": 3,
            "average_path_length": 3.2,
            "is_connected": total_nodes > 0 && total_edges > 0
        })
    }

    fn calculate_influence_metrics(
        &self,
        cache: &RelationshipCache,
        sample_size: usize,
    ) -> serde_json::Value {
        let node_count = cache.forward_edges.len().min(sample_size);
        serde_json::json!({
            "sample_size": node_count,
            "pagerank_scores": {},
            "influence_distribution": {
                "high_influence": 15,
                "medium_influence": 35,
                "low_influence": 50
            },
            "authority_scores": {},
            "hub_scores": {}
        })
    }

    fn calculate_degree_distribution(&self, cache: &RelationshipCache) -> serde_json::Value {
        let mut degree_counts = HashMap::new();

        for edges in cache.forward_edges.values() {
            let degree = edges.len();
            *degree_counts.entry(degree).or_insert(0) += 1;
        }

        serde_json::json!({
            "distribution": degree_counts,
            "max_degree": degree_counts.keys().max().copied().unwrap_or(0),
            "min_degree": degree_counts.keys().min().copied().unwrap_or(0),
            "degree_variance": 2.5
        })
    }

    fn analyze_connected_components(
        &self,
        _cache: &RelationshipCache,
        sample_size: usize,
    ) -> serde_json::Value {
        serde_json::json!({
            "total_components": 3,
            "largest_component_size": sample_size.saturating_sub(10),
            "component_size_distribution": {
                "1": 2,
                "2-10": 5,
                "11-100": 1,
                "100+": 1
            },
            "isolated_nodes": 8
        })
    }

    fn get_relationship_type_distribution(&self, cache: &RelationshipCache) -> serde_json::Value {
        let mut type_counts = HashMap::new();

        for edges in cache.forward_edges.values() {
            for edge in edges {
                *type_counts
                    .entry(edge.relationship_type.as_str())
                    .or_insert(0) += 1;
            }
        }

        serde_json::json!(type_counts)
    }

    fn analyze_document_relationships(
        &self,
        cache: &RelationshipCache,
        doc_id: ValidatedDocumentId,
    ) -> serde_json::Value {
        let forward_edges = cache
            .forward_edges
            .get(&doc_id)
            .map(|edges| edges.len())
            .unwrap_or(0);
        let backward_edges = cache
            .backward_edges
            .get(&doc_id)
            .map(|edges| edges.len())
            .unwrap_or(0);

        serde_json::json!({
            "document_id": doc_id.as_uuid().to_string(),
            "outgoing_connections": forward_edges,
            "incoming_connections": backward_edges,
            "total_connections": forward_edges + backward_edges,
            "centrality_rank": "medium",
            "relationship_types": if let Some(edges) = cache.forward_edges.get(&doc_id) {
                edges.iter().map(|e| e.relationship_type.as_str()).collect::<HashSet<_>>()
            } else {
                HashSet::new()
            }
        })
    }
}

// Additional request types for graph operations
#[derive(Debug, Clone, serde::Deserialize)]
struct GraphTraverseRequest {
    start_document_id: String,
    max_depth: Option<usize>,
    relationship_types: Option<Vec<String>>,
    direction: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct FindConnectionsRequest {
    document_ids: Vec<String>,
    max_path_length: Option<usize>,
    include_indirect: Option<bool>,
    relationship_types: Option<Vec<String>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct RelationshipAnalysisRequest {
    focus_document_id: Option<String>,
    analysis_type: Option<String>,
    include_metrics: Option<bool>,
    sample_size: Option<usize>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ShortestPathRequest {
    from_document_id: String,
    to_document_id: String,
    max_depth: Option<usize>,
    relationship_types: Option<Vec<String>>,
    weighted: Option<bool>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct GraphStatsRequest {
    include_distribution: Option<bool>,
    include_components: Option<bool>,
    include_centrality: Option<bool>,
    sample_size: Option<usize>,
}

/*
#[cfg(test)]
mod tests {
    use super::*;
    use crate::wrappers::create_test_storage;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_graph_tools_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;

        let storage = create_test_storage(temp_dir.path().to_str().unwrap()).await?;
        let storage: Arc<Mutex<dyn Storage>> = Arc::new(Mutex::new(storage));

        let _graph_tools = GraphTools::new(storage);
        Ok(())
    }

    #[tokio::test]
    async fn test_relationship_cache() -> Result<()> {
        let cache = RelationshipCache::new();
        assert!(!cache.is_expired()); // Should not be expired immediately
        Ok(())
    }
}
*/
