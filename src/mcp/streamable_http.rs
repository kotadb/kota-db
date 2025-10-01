//! Streamable HTTP transport for the KotaDB MCP server.
//!
//! Provides an implementation of the 2025-06-18 MCP Streamable HTTP transport
//! atop the existing JSON-RPC tool registry. The endpoint supports the
//! negotiated POST semantics as well as a basic SSE channel for
//! server-initiated messages. Infrastructure for resumability and session
//! tracking is included so that future enhancements can push
//! notifications/events without reworking the HTTP surface.

use std::collections::{HashMap, VecDeque};
use std::convert::Infallible;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::State;
use axum::http::header::{self, HeaderMap, HeaderName, HeaderValue};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive};
use axum::response::{IntoResponse, Response, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use bytes::Bytes;
use chrono::Utc;
use futures::StreamExt;
use jsonrpc_core::types::request::{Call, MethodCall};
use jsonrpc_core::types::response::{Failure, Output, Success};
use jsonrpc_core::{Error as RpcError, ErrorCode, Params, Value, Version};
use serde::Serialize;
use tokio::sync::{broadcast, Mutex, RwLock};
use tokio_stream::wrappers::{errors::BroadcastStreamRecvError, BroadcastStream};
use uuid::Uuid;

use crate::mcp::config::MCPConfig;
use crate::mcp::tools::MCPToolRegistry;
use crate::mcp::types::ToolDefinition;

/// Router builder for the Streamable HTTP transport.
pub fn create_streamable_http_router(state: StreamableHttpState) -> Router {
    Router::new()
        .route(
            "/mcp",
            post(handle_streamable_post).get(handle_streamable_get),
        )
        // Keep legacy /mcp/tools bridge mounted for backwards compatibility.
        .route("/mcp/tools", get(list_tools_legacy).post(list_tools_legacy))
        .route("/mcp/tools/:tool_name", post(call_tool_legacy))
        .with_state(state)
}

/// Shared state for the Streamable HTTP transport.
#[derive(Clone)]
pub struct StreamableHttpState {
    config: Arc<MCPConfig>,
    tool_registry: Arc<MCPToolRegistry>,
    start_time: Arc<Instant>,
    session_manager: Arc<SessionManager>,
    allowed_origins: Arc<Vec<String>>,
}

impl StreamableHttpState {
    /// Construct streamable HTTP state from MCP configuration and registry.
    pub fn new(
        config: Arc<MCPConfig>,
        tool_registry: Arc<MCPToolRegistry>,
        start_time: Instant,
    ) -> Self {
        let allowed_origins = derive_allowed_origins(&config);
        Self {
            config,
            tool_registry,
            start_time: Arc::new(start_time),
            session_manager: Arc::new(SessionManager::new()),
            allowed_origins: Arc::new(allowed_origins),
        }
    }

    /// Validate the MCP protocol version header and return the negotiated value.
    fn extract_protocol_version(&self, headers: &HeaderMap) -> Result<String, McpHttpError> {
        if let Some(raw) = headers.get(MCP_PROTOCOL_VERSION_HEADER) {
            let value = raw.to_str().map_err(|_| {
                McpHttpError::bad_request(
                    "invalid_protocol_version",
                    "MCP-Protocol-Version header must be valid UTF-8",
                )
            })?;
            if value.trim().is_empty() {
                return Err(McpHttpError::bad_request(
                    "invalid_protocol_version",
                    "MCP-Protocol-Version header must not be empty",
                ));
            }

            if value != self.config.mcp.protocol_version {
                tracing::warn!(
                    "Unsupported MCP protocol version requested: {} (supported: {})",
                    value,
                    self.config.mcp.protocol_version
                );
                return Err(McpHttpError::bad_request(
                    "unsupported_protocol_version",
                    format!(
                        "Unsupported MCP protocol version '{}'. This server supports {}",
                        value, self.config.mcp.protocol_version
                    ),
                ));
            }
            Ok(value.to_string())
        } else {
            // For backwards compatibility fall back to configured version but issue a warning.
            tracing::warn!(
                "Missing MCP-Protocol-Version header; assuming {}",
                self.config.mcp.protocol_version
            );
            Ok(self.config.mcp.protocol_version.clone())
        }
    }

    /// Ensure the Origin header is within the allowed list (if provided).
    fn validate_origin(&self, headers: &HeaderMap) -> Result<(), McpHttpError> {
        if let Some(origin) = headers.get(header::ORIGIN) {
            let origin = origin.to_str().map_err(|_| {
                McpHttpError::bad_request("invalid_origin", "Origin header must be valid UTF-8")
            })?;
            if !self.allowed_origins.is_empty()
                && !self.allowed_origins.iter().any(|allowed| allowed == origin)
            {
                tracing::warn!("Rejected request with disallowed origin: {}", origin);
                return Err(McpHttpError::forbidden(
                    "origin_not_allowed",
                    format!(
                        "Origin '{}' is not permitted. Allowed origins: {}",
                        origin,
                        self.allowed_origins.join(", ")
                    ),
                ));
            }
        }
        Ok(())
    }

    /// Ensure Accept header includes the required MIME types for POST behaviour.
    fn validate_accept_for_post(&self, headers: &HeaderMap) -> Result<(), McpHttpError> {
        if let Some(accept) = headers.get(header::ACCEPT) {
            let accept = accept.to_str().map_err(|_| {
                McpHttpError::bad_request("invalid_accept", "Accept header must be valid UTF-8")
            })?;
            if !accept_contains(accept, "application/json")
                || !accept_contains(accept, "text/event-stream")
            {
                return Err(McpHttpError::not_acceptable(
                    "invalid_accept",
                    "Accept header must include both application/json and text/event-stream",
                ));
            }
        } else {
            return Err(McpHttpError::not_acceptable(
                "missing_accept",
                "Accept header is required and must include both application/json and text/event-stream",
            ));
        }
        Ok(())
    }

    /// Ensure Accept header for GET requests supports SSE.
    fn validate_accept_for_get(&self, headers: &HeaderMap) -> Result<(), McpHttpError> {
        if let Some(accept) = headers.get(header::ACCEPT) {
            let accept = accept.to_str().map_err(|_| {
                McpHttpError::bad_request("invalid_accept", "Accept header must be valid UTF-8")
            })?;
            if !accept_contains(accept, "text/event-stream") {
                return Err(McpHttpError::not_acceptable(
                    "invalid_accept",
                    "Accept header must include text/event-stream for SSE connections",
                ));
            }
        } else {
            return Err(McpHttpError::not_acceptable(
                "missing_accept",
                "Accept header is required for SSE connections",
            ));
        }
        Ok(())
    }

    /// Lookup a session id from headers, returning an error if not present.
    fn require_session_header<'a>(&self, headers: &'a HeaderMap) -> Result<&'a str, McpHttpError> {
        headers
            .get(MCP_SESSION_ID_HEADER)
            .ok_or_else(|| {
                McpHttpError::bad_request("missing_session", "Mcp-Session-Id header is required")
            })?
            .to_str()
            .map_err(|_| {
                McpHttpError::bad_request(
                    "invalid_session",
                    "Mcp-Session-Id header must be valid UTF-8",
                )
            })
    }

    fn session_manager(&self) -> Arc<SessionManager> {
        self.session_manager.clone()
    }

    fn tool_registry(&self) -> Arc<MCPToolRegistry> {
        self.tool_registry.clone()
    }

    fn server_capabilities(&self) -> Value {
        serde_json::json!({
            "capabilities": {
                "tools": {
                    "listChanged": false,
                    "supportsProgress": false
                },
                "resources": {
                    "listChanged": false,
                    "subscribe": false
                },
                "logging": {},
                "prompts": {
                    "listChanged": false
                }
            },
            "serverInfo": {
                "name": self.config.mcp.server_name,
                "version": self.config.mcp.server_version
            },
            "protocolVersion": self.config.mcp.protocol_version
        })
    }
}

/// Handle POST /mcp requests following the spec requirements.
async fn handle_streamable_post(
    State(state): State<StreamableHttpState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, McpHttpError> {
    state.validate_origin(&headers)?;
    state.validate_accept_for_post(&headers)?;
    let protocol_version = state.extract_protocol_version(&headers)?;

    // Parse JSON payload. We do this manually to customize the error response.
    let payload: Value = serde_json::from_slice(&body).map_err(|err| {
        tracing::warn!("Failed to parse JSON-RPC payload: {}", err);
        McpHttpError::bad_request("invalid_json", "Request body must be valid JSON")
    })?;

    // Attempt to treat payload as JSON-RPC request first.
    if let Ok(request) = serde_json::from_value::<jsonrpc_core::Request>(payload.clone()) {
        match request {
            jsonrpc_core::Request::Single(call) => {
                return handle_single_call(state, headers, call, protocol_version).await;
            }
            jsonrpc_core::Request::Batch(_) => {
                return Err(McpHttpError::bad_request(
                    "batch_not_supported",
                    "Batch JSON-RPC requests are not supported by this MCP endpoint",
                ));
            }
        }
    }

    // If this was not a request, try to interpret it as a response sent by the client.
    if serde_json::from_value::<Output>(payload).is_ok() {
        // Server currently does not issue client-directed requests, but acknowledge per spec.
        return Ok(Response::builder()
            .status(StatusCode::ACCEPTED)
            .body(axum::body::Body::empty())
            .expect("failed to build empty accepted response"));
    }

    Err(McpHttpError::bad_request(
        "invalid_message",
        "Body must contain a JSON-RPC request, notification, or response",
    ))
}

/// Handle GET /mcp SSE streams for server-initiated messages.
async fn handle_streamable_get(
    State(state): State<StreamableHttpState>,
    headers: HeaderMap,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>, McpHttpError> {
    state.validate_origin(&headers)?;
    state.validate_accept_for_get(&headers)?;
    let protocol_version = state.extract_protocol_version(&headers)?;
    let session_id = state.require_session_header(&headers)?;

    let session = state
        .session_manager()
        .get_session(session_id)
        .await
        .ok_or_else(|| {
            McpHttpError::not_found("unknown_session", "Session not found or has expired")
        })?;

    if session.protocol_version != protocol_version {
        return Err(McpHttpError::bad_request(
            "protocol_mismatch",
            format!(
                "Session negotiated protocol {}, but client used {}",
                session.protocol_version, protocol_version
            ),
        ));
    }

    let last_event_id = headers
        .get(HeaderName::from_static("last-event-id"))
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());

    let backlog = session.backlog_since(last_event_id).await;
    let rx = session.subscribe();

    let backlog_stream =
        futures::stream::iter(backlog.into_iter().map(event_to_sse)).map(Ok::<_, Infallible>);

    let live_stream = BroadcastStream::new(rx)
        .filter_map(|result| async move {
            match result {
                Ok(event) => Some(event_to_sse(event)),
                Err(BroadcastStreamRecvError::Lagged(skipped)) => {
                    let warning_event = Event::default()
                        .event("warning")
                        .data(format!("{{\"message\":\"dropped {} events\"}}", skipped));
                    Some(warning_event)
                }
            }
        })
        .map(Ok::<_, Infallible>);

    let stream = backlog_stream.chain(live_stream);

    let sse = Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(20))
            .text(": keep-alive"),
    );

    Ok(sse)
}

/// Handle a single JSON-RPC call.
async fn handle_single_call(
    state: StreamableHttpState,
    headers: HeaderMap,
    call: Call,
    protocol_version: String,
) -> Result<Response, McpHttpError> {
    match call {
        Call::MethodCall(method_call) => {
            process_method_call(state, headers, method_call, protocol_version).await
        }
        Call::Notification(_) => Ok(Response::builder()
            .status(StatusCode::ACCEPTED)
            .body(axum::body::Body::empty())
            .expect("failed to build empty accepted response")),
        Call::Invalid { .. } => Err(McpHttpError::bad_request(
            "invalid_request",
            "The JSON-RPC request is invalid",
        )),
    }
}

/// Process JSON-RPC method calls.
async fn process_method_call(
    state: StreamableHttpState,
    headers: HeaderMap,
    call: MethodCall,
    protocol_version: String,
) -> Result<Response, McpHttpError> {
    let session_manager = state.session_manager();
    let registry = state.tool_registry();
    let mut session_for_response = None;

    let result = match call.method.as_str() {
        "initialize" => {
            let session = session_manager
                .create_session(protocol_version.clone())
                .await;
            session_for_response = Some(session.id.clone());
            Ok(state.server_capabilities())
        }
        "tools/list" => {
            let _session = session_manager
                .require_session(state.require_session_header(&headers)?)
                .await?;
            let tools = registry.get_all_tool_definitions();
            Ok(serde_json::json!({ "tools": tools }))
        }
        "tools/call" => {
            let session_id = state.require_session_header(&headers)?;
            let _session = session_manager.require_session(session_id).await?;
            let params = call.params.clone();
            handle_tool_call(&registry, params).await
        }
        "resources/list" => {
            let _session = session_manager
                .require_session(state.require_session_header(&headers)?)
                .await?;
            Ok(serde_json::json!({ "resources": [] }))
        }
        "resources/read" => Err(McpHttpError::not_found(
            "resource_not_found",
            "resources/read is not implemented",
        )),
        "capabilities" => {
            let _session = session_manager
                .require_session(state.require_session_header(&headers)?)
                .await?;
            Ok(state.server_capabilities())
        }
        "ping" => {
            let session_id = state.require_session_header(&headers)?;
            let session = session_manager.require_session(session_id).await?;
            Ok(serde_json::json!({
                "status": "ok",
                "timestamp": Utc::now().to_rfc3339(),
                "uptime_seconds": state.start_time.elapsed().as_secs(),
                "session": session.id,
            }))
        }
        _ => {
            let err = RpcError {
                code: ErrorCode::MethodNotFound,
                message: format!("Unknown method {}", call.method),
                data: None,
            };
            return Ok(jsonrpc_response(
                Output::Failure(Failure {
                    jsonrpc: Some(Version::V2),
                    error: err,
                    id: call.id,
                }),
                session_for_response,
            ));
        }
    }?;

    let output = Output::Success(Success {
        jsonrpc: Some(Version::V2),
        result,
        id: call.id,
    });

    Ok(jsonrpc_response(output, session_for_response))
}

/// Helper to invoke tool registry for tools/call.
async fn handle_tool_call(
    registry: &Arc<MCPToolRegistry>,
    params: Params,
) -> Result<Value, McpHttpError> {
    let params: Value = params.parse().map_err(|_| {
        McpHttpError::bad_request("invalid_params", "tools/call requires named parameters")
    })?;

    let name = params.get("name").and_then(Value::as_str).ok_or_else(|| {
        McpHttpError::bad_request("missing_tool_name", "tools/call requires a 'name' field")
    })?;

    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

    let method = name.to_string();
    let response = registry
        .handle_tool_call(&method, arguments)
        .await
        .map_err(|err| {
            tracing::error!("MCP tool call failed for {}: {}", method, err);
            McpHttpError::internal_error("tool_error", format!("Tool call failed: {}", err))
        })?;

    Ok(serde_json::json!({
        "content": [
            {
                "type": "text",
                "text": serde_json::to_string_pretty(&response)
                    .unwrap_or_else(|_| response.to_string())
            }
        ]
    }))
}

/// Translate a JSON-RPC output into an HTTP response with appropriate headers.
fn jsonrpc_response(output: Output, session_id: Option<String>) -> Response {
    let mut builder = Response::builder().status(StatusCode::OK).header(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );

    if let Some(session_id) = session_id {
        builder = builder.header(
            MCP_SESSION_ID_HEADER,
            HeaderValue::from_str(&session_id).unwrap(),
        );
    }

    let body = serde_json::to_vec(&output).unwrap_or_else(|_| b"{}".to_vec());
    builder
        .body(Body::from(body))
        .expect("failed to build JSON-RPC response")
}

/// Legacy GET/POST tool list handler retained for backwards compatibility.
async fn list_tools_legacy(
    State(state): State<StreamableHttpState>,
) -> Result<Json<McpToolsListResponse>, McpHttpError> {
    let tools = state
        .tool_registry()
        .get_all_tool_definitions()
        .into_iter()
        .map(|t| {
            let category = categorize_tool(&t.name);
            McpToolDefinition {
                name: t.name,
                description: t.description,
                category,
            }
        })
        .collect::<Vec<_>>();

    Ok(Json(McpToolsListResponse {
        total_count: tools.len(),
        tools,
    }))
}

/// Legacy tool invocation handler to maintain compatibility with the previous bridge.
async fn call_tool_legacy(
    State(state): State<StreamableHttpState>,
    axum::extract::Path(tool_name): axum::extract::Path<String>,
    Json(request): Json<McpToolRequest>,
) -> Result<Json<McpToolResponse>, McpHttpError> {
    let method = map_tool_name_to_mcp_method(&tool_name).ok_or_else(|| {
        McpHttpError::not_found("tool_not_found", format!("Unknown tool: {}", tool_name))
    })?;

    let response = state
        .tool_registry()
        .handle_tool_call(&method, request.params.clone())
        .await
        .map_err(|err| {
            McpHttpError::internal_error("tool_error", format!("Tool call failed: {}", err))
        })?;

    Ok(Json(McpToolResponse {
        success: true,
        data: Some(response),
        error: None,
    }))
}

/// Session manager storing per-client state for SSE delivery.
struct SessionManager {
    sessions: RwLock<HashMap<String, Arc<Session>>>,
}

impl SessionManager {
    fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    async fn create_session(&self, protocol_version: String) -> Arc<Session> {
        let id = Uuid::new_v4().to_string();
        let (tx, _rx) = broadcast::channel(64);
        let session = Arc::new(Session::new(id.clone(), protocol_version, tx));
        self.sessions.write().await.insert(id, session.clone());
        session
    }

    async fn get_session(&self, id: &str) -> Option<Arc<Session>> {
        self.sessions.read().await.get(id).cloned()
    }

    async fn require_session(&self, id: &str) -> Result<Arc<Session>, McpHttpError> {
        self.get_session(id).await.ok_or_else(|| {
            McpHttpError::not_found("unknown_session", "Session not found or expired")
        })
    }
}

/// Per-session SSE state.
struct Session {
    id: String,
    protocol_version: String,
    #[allow(dead_code)]
    created_at: Instant,
    tx: broadcast::Sender<ServerEvent>,
    backlog: Mutex<VecDeque<ServerEvent>>,
    next_event_id: AtomicU64,
}

impl Session {
    fn new(id: String, protocol_version: String, tx: broadcast::Sender<ServerEvent>) -> Self {
        Self {
            id,
            protocol_version,
            created_at: Instant::now(),
            tx,
            backlog: Mutex::new(VecDeque::new()),
            next_event_id: AtomicU64::new(1),
        }
    }

    fn subscribe(&self) -> broadcast::Receiver<ServerEvent> {
        self.tx.subscribe()
    }

    async fn backlog_since(&self, last_event: Option<u64>) -> Vec<ServerEvent> {
        let backlog = self.backlog.lock().await;
        backlog
            .iter()
            .filter(|event| last_event.map(|id| event.id > id).unwrap_or(true))
            .cloned()
            .collect()
    }

    #[allow(dead_code)]
    async fn publish(&self, payload: Value) {
        let id = self.next_event_id.fetch_add(1, Ordering::Relaxed);
        let event = ServerEvent { id, payload };
        {
            let mut backlog = self.backlog.lock().await;
            backlog.push_back(event.clone());
            const MAX_BACKLOG: usize = 256;
            if backlog.len() > MAX_BACKLOG {
                backlog.pop_front();
            }
        }
        let _ = self.tx.send(event);
    }
}

#[derive(Clone)]
struct ServerEvent {
    id: u64,
    payload: Value,
}

fn event_to_sse(event: ServerEvent) -> Event {
    let payload = serde_json::to_string(&event.payload).unwrap_or_else(|_| "{}".to_string());
    Event::default().id(event.id.to_string()).data(payload)
}

/// Error representation for HTTP responses.
#[derive(Debug)]
struct McpHttpError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl McpHttpError {
    fn bad_request(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code,
            message: message.into(),
        }
    }

    fn forbidden(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code,
            message: message.into(),
        }
    }

    fn not_found(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code,
            message: message.into(),
        }
    }

    fn not_acceptable(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_ACCEPTABLE,
            code,
            message: message.into(),
        }
    }

    fn internal_error(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code,
            message: message.into(),
        }
    }
}

impl IntoResponse for McpHttpError {
    fn into_response(self) -> Response {
        let body = Json(McpErrorBody {
            error: self.code.into(),
            message: self.message,
        });
        (self.status, body).into_response()
    }
}

#[derive(Serialize)]
struct McpErrorBody {
    error: String,
    message: String,
}

/// Request/response structures reused from the legacy HTTP bridge to preserve behaviour.
#[derive(Debug, serde::Deserialize)]
struct McpToolRequest {
    #[serde(flatten)]
    params: Value,
}

#[derive(Debug, serde::Serialize)]
struct McpToolResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpErrorPayload>,
}

#[derive(Debug, serde::Serialize)]
struct McpErrorPayload {
    code: String,
    message: String,
}

#[derive(Debug, serde::Serialize)]
struct McpToolsListResponse {
    tools: Vec<McpToolDefinition>,
    total_count: usize,
}

#[derive(Debug, serde::Serialize)]
struct McpToolDefinition {
    name: String,
    description: String,
    category: String,
}

fn categorize_tool(name: &str) -> String {
    let lname = name.to_lowercase();
    if lname.contains("symbol") || lname.contains("search") {
        "search".into()
    } else if lname.contains("caller") || lname.contains("relationship") {
        "relationships".into()
    } else if lname.contains("impact") || lname.contains("analysis") {
        "analysis".into()
    } else {
        "general".into()
    }
}

fn map_tool_name_to_mcp_method(name: &str) -> Option<String> {
    match name {
        "search_code" => Some("kotadb://text_search/code".into()),
        "search_symbols" => Some("kotadb://symbol_search/query".into()),
        "find_callers" => Some("kotadb://find_callers".into()),
        "analyze_impact" => Some("kotadb://impact_analysis".into()),
        "stats" => Some("kotadb://status/system".into()),
        other => {
            tracing::warn!("Unknown legacy tool mapping requested: {}", other);
            None
        }
    }
}

fn accept_contains(accept: &str, needle: &str) -> bool {
    accept
        .split(',')
        .any(|part| part.trim().starts_with(needle) || part.trim() == "*/*")
}

const MCP_PROTOCOL_VERSION_HEADER: &str = "mcp-protocol-version";
const MCP_SESSION_ID_HEADER: &str = "mcp-session-id";

fn derive_allowed_origins(config: &MCPConfig) -> Vec<String> {
    let mut origins = vec![
        format!("http://localhost:{}", config.server.port),
        "http://localhost".to_string(),
        format!("http://127.0.0.1:{}", config.server.port),
        "http://127.0.0.1".to_string(),
    ];

    if let Some(custom) = config.security.allowed_origins.as_ref().and_then(|list| {
        if list.is_empty() {
            None
        } else {
            Some(list.clone())
        }
    }) {
        origins.extend(custom);
    }

    origins.sort();
    origins.dedup();
    origins
}

/// Helper trait for reuse in documentation/tests.
pub fn legacy_tool_definitions(tool_registry: &MCPToolRegistry) -> Vec<ToolDefinition> {
    tool_registry.get_all_tool_definitions()
}
