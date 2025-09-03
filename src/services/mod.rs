// Services Layer - Business logic extraction for interface parity
//
// This module provides a unified services layer that extracts business logic
// from CLI commands, making it reusable across CLI, MCP, and future interfaces.
// This ensures feature parity and eliminates code duplication.

pub mod analysis_service;
pub mod management_service;
pub mod search_service;

pub use analysis_service::{
    AnalysisService, CallSite, CallersOptions, CallersResult, ImpactOptions, ImpactResult,
    ImpactSite, OverviewOptions, OverviewResult,
};
pub use management_service::{
    BasicStats, BenchmarkOptions, BenchmarkResult, BenchmarkTypeResult, DependencyGraphStats,
    IndexCodebaseOptions, IndexResult, ManagementService, RelationshipStats, ServerOptions,
    StatsOptions, StatsResult, SymbolStats, ValidateOptions, ValidateResult, ValidationCheck,
};
pub use search_service::{
    DatabaseAccess, SearchOptions, SearchResult, SearchService, SearchType, SymbolMatch,
    SymbolResult, SymbolSearchOptions,
};
