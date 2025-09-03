// Services Layer - Business logic extraction for interface parity
//
// This module provides a unified services layer that extracts business logic
// from CLI commands, making it reusable across CLI, MCP, and future interfaces.
// This ensures feature parity and eliminates code duplication.

pub mod search_service;

pub use search_service::{
    DatabaseAccess, SearchOptions, SearchResult, SearchService, SearchType, SymbolMatch,
    SymbolResult, SymbolSearchOptions,
};
