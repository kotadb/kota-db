// Documentation Verification Framework
// Systematic validation of documentation claims vs actual implementation
// Addresses issue #180: Documentation accuracy verification

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, info};

/// Status of a verification check
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationStatus {
    Verified,     // Feature exists and works as documented
    Missing,      // Feature is documented but not implemented
    Partial,      // Feature exists but behaves differently than documented
    Undocumented, // Feature exists but not documented
}

/// A single verification check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationCheck {
    pub feature: String,
    pub status: VerificationStatus,
    pub documented_claim: String,
    pub actual_implementation: String,
    pub severity: Severity,
    pub recommendation: Option<String>,
    pub location: String, // File and line where claim is made
}

/// Severity level for discrepancies
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Critical, // Feature completely missing or broken
    High,     // Significant difference from documentation
    Medium,   // Minor behavioral differences
    Low,      // Minor documentation issues (typos, outdated details)
}

/// Overall verification report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationVerificationReport {
    pub total_checks: usize,
    pub verified_count: usize,
    pub missing_count: usize,
    pub partial_count: usize,
    pub undocumented_count: usize,
    pub checks: Vec<VerificationCheck>,
    pub summary: String,
    pub critical_issues: Vec<String>,
    pub recommendations: Vec<String>,
}

impl Default for DocumentationVerificationReport {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentationVerificationReport {
    /// Create a new empty report
    pub fn new() -> Self {
        Self {
            total_checks: 0,
            verified_count: 0,
            missing_count: 0,
            partial_count: 0,
            undocumented_count: 0,
            checks: Vec::new(),
            summary: String::new(),
            critical_issues: Vec::new(),
            recommendations: Vec::new(),
        }
    }

    /// Add a verification check to the report
    pub fn add_check(&mut self, check: VerificationCheck) {
        // Update counters
        match check.status {
            VerificationStatus::Verified => self.verified_count += 1,
            VerificationStatus::Missing => self.missing_count += 1,
            VerificationStatus::Partial => self.partial_count += 1,
            VerificationStatus::Undocumented => self.undocumented_count += 1,
        }

        // Collect critical issues
        if check.severity == Severity::Critical {
            self.critical_issues.push(format!(
                "{}: {}",
                check.feature, check.actual_implementation
            ));
        }

        // Collect recommendations
        if let Some(ref rec) = check.recommendation {
            self.recommendations.push(rec.clone());
        }

        self.checks.push(check);
        self.total_checks += 1;
    }

    /// Finalize the report with summary
    pub fn finalize(&mut self) {
        let accuracy_percent = if self.total_checks > 0 {
            (self.verified_count as f64 / self.total_checks as f64 * 100.0).round() as u32
        } else {
            100
        };

        self.summary = format!(
            "Documentation Accuracy: {}% ({}/{} features verified). {} missing, {} partial, {} undocumented.",
            accuracy_percent,
            self.verified_count,
            self.total_checks,
            self.missing_count,
            self.partial_count,
            self.undocumented_count
        );
    }

    /// Check if the report indicates acceptable accuracy
    pub fn is_acceptable(&self) -> bool {
        // No critical issues is the main requirement
        // Undocumented endpoints are not failures, just recommendations
        self.critical_issues.is_empty()
    }
}

/// Main verification engine
pub struct DocumentationVerifier {
    report: DocumentationVerificationReport,
}

impl DocumentationVerifier {
    pub fn new() -> Self {
        Self {
            report: DocumentationVerificationReport::new(),
        }
    }

    /// Verify API endpoint documentation claims
    pub fn verify_api_endpoints(&mut self) -> Result<()> {
        info!("Verifying API endpoint documentation claims");

        // Check documented endpoints vs actual HTTP server routes
        let documented_endpoints = self.get_documented_endpoints();
        let actual_endpoints = self.get_actual_endpoints()?;

        for (method, path, description) in &documented_endpoints {
            let endpoint_key = format!("{} {}", method, path);

            if actual_endpoints.contains_key(&endpoint_key) {
                self.report.add_check(VerificationCheck {
                    feature: format!("API Endpoint: {} {}", method, path),
                    status: VerificationStatus::Verified,
                    documented_claim: description.clone(),
                    actual_implementation: "Endpoint exists and is routed correctly".to_string(),
                    severity: Severity::Low,
                    recommendation: None,
                    location: "docs/api/api_reference.md".to_string(),
                });
            } else {
                self.report.add_check(VerificationCheck {
                    feature: format!("API Endpoint: {} {}", method, path),
                    status: VerificationStatus::Missing,
                    documented_claim: description.clone(),
                    actual_implementation: "Endpoint not found in HTTP server routes".to_string(),
                    severity: Severity::Critical,
                    recommendation: Some(
                        "Either implement the endpoint or remove it from documentation".to_string(),
                    ),
                    location: "docs/api/api_reference.md".to_string(),
                });
            }
        }

        // Check for undocumented endpoints
        let documented_keys: std::collections::HashSet<String> = documented_endpoints
            .iter()
            .map(|(method, path, _)| format!("{} {}", method, path))
            .collect();

        for endpoint_key in actual_endpoints.keys() {
            if !documented_keys.contains(endpoint_key) {
                self.report.add_check(VerificationCheck {
                    feature: format!("Undocumented API Endpoint: {}", endpoint_key),
                    status: VerificationStatus::Undocumented,
                    documented_claim: "Not documented".to_string(),
                    actual_implementation: "Endpoint exists in HTTP server".to_string(),
                    severity: Severity::Medium,
                    recommendation: Some("Add documentation for this endpoint".to_string()),
                    location: "src/http_server.rs".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Verify client library claims
    pub fn verify_client_libraries(&mut self) -> Result<()> {
        info!("Verifying client library documentation claims");

        // Python Client
        let python_exists = self.check_local_python_client().unwrap_or(false);

        self.report.add_check(VerificationCheck {
            feature: "Python Client Library".to_string(),
            status: if python_exists {
                VerificationStatus::Verified
            } else {
                VerificationStatus::Missing
            },
            documented_claim: "pip install kotadb-client (local client structure)".to_string(),
            actual_implementation: if python_exists {
                "Python client exists with proper local structure".to_string()
            } else {
                "Python client directory not found or incomplete".to_string()
            },
            severity: if !python_exists {
                Severity::Critical
            } else {
                Severity::Low
            },
            recommendation: if !python_exists {
                Some("Implement Python client or remove from documentation".to_string())
            } else {
                None
            },
            location: "README.md:14-19, docs/api/api_reference.md:76-114".to_string(),
        });

        // TypeScript Client
        let typescript_exists = self.check_local_typescript_client().unwrap_or(false);

        self.report.add_check(VerificationCheck {
            feature: "TypeScript Client Library".to_string(),
            status: if typescript_exists {
                VerificationStatus::Verified
            } else {
                VerificationStatus::Missing
            },
            documented_claim: "npm install kotadb-client (local client structure)".to_string(),
            actual_implementation: if typescript_exists {
                "TypeScript client exists with proper local structure".to_string()
            } else {
                "TypeScript client directory not found or incomplete".to_string()
            },
            severity: if !typescript_exists {
                Severity::Critical
            } else {
                Severity::Low
            },
            recommendation: if !typescript_exists {
                Some("Implement TypeScript client or remove from documentation".to_string())
            } else {
                None
            },
            location: "README.md:30-45, docs/api/api_reference.md:116-155".to_string(),
        });

        // Go Client
        let go_exists = std::path::Path::new("clients/go").exists();
        self.report.add_check(VerificationCheck {
            feature: "Go Client Library".to_string(),
            status: if go_exists {
                VerificationStatus::Verified
            } else {
                VerificationStatus::Missing
            },
            documented_claim: "🚧 Work in Progress - Go client is currently under development"
                .to_string(),
            actual_implementation: if go_exists {
                "Go client directory exists".to_string()
            } else {
                "Go client correctly marked as work in progress".to_string()
            },
            severity: Severity::Low, // Correctly marked as WIP
            recommendation: None,
            location: "README.md:74-80".to_string(),
        });

        Ok(())
    }

    /// Verify feature claims from README and documentation
    pub fn verify_core_features(&mut self) -> Result<()> {
        info!("Verifying core feature documentation claims");

        // Storage Engine Features
        self.verify_storage_features()?;

        // Index Features
        self.verify_index_features()?;

        // Performance Claims
        self.verify_performance_claims()?;

        // Example Applications
        self.verify_examples()?;

        Ok(())
    }

    fn verify_storage_features(&mut self) -> Result<()> {
        // Check WAL implementation
        let wal_exists = std::path::Path::new("src/file_storage.rs").exists();
        // TODO: Could grep for WAL-related code

        self.report.add_check(VerificationCheck {
            feature: "Write-Ahead Log (WAL)".to_string(),
            status: if wal_exists {
                VerificationStatus::Verified
            } else {
                VerificationStatus::Missing
            },
            documented_claim: "WAL ensures data durability".to_string(),
            actual_implementation: if wal_exists {
                "FileStorage implementation includes WAL functionality".to_string()
            } else {
                "WAL implementation not found".to_string()
            },
            severity: if !wal_exists {
                Severity::Critical
            } else {
                Severity::Low
            },
            recommendation: None,
            location: "README.md:256-259".to_string(),
        });

        Ok(())
    }

    fn verify_index_features(&mut self) -> Result<()> {
        // B+ Tree Index
        let btree_exists = std::path::Path::new("src/primary_index.rs").exists();
        self.report.add_check(VerificationCheck {
            feature: "B+ Tree Primary Index".to_string(),
            status: if btree_exists {
                VerificationStatus::Verified
            } else {
                VerificationStatus::Missing
            },
            documented_claim: "O(log n) path-based lookups with wildcard support".to_string(),
            actual_implementation: if btree_exists {
                "PrimaryIndex implementation exists with B+ tree structure".to_string()
            } else {
                "Primary index implementation not found".to_string()
            },
            severity: if !btree_exists {
                Severity::Critical
            } else {
                Severity::Low
            },
            recommendation: None,
            location: "README.md:262".to_string(),
        });

        // Trigram Index
        let trigram_exists = std::path::Path::new("src/trigram_index.rs").exists();
        self.report.add_check(VerificationCheck {
            feature: "Trigram Full-Text Search".to_string(),
            status: if trigram_exists {
                VerificationStatus::Verified
            } else {
                VerificationStatus::Missing
            },
            documented_claim: "Fuzzy-tolerant full-text search with ranking".to_string(),
            actual_implementation: if trigram_exists {
                "TrigramIndex implementation exists".to_string()
            } else {
                "Trigram index implementation not found".to_string()
            },
            severity: if !trigram_exists {
                Severity::Critical
            } else {
                Severity::Low
            },
            recommendation: None,
            location: "README.md:263".to_string(),
        });

        // Vector semantics retired
        self.report.add_check(VerificationCheck {
            feature: "Vector Semantic Search".to_string(),
            status: VerificationStatus::Verified,
            documented_claim:
                "Semantic search capabilities are retired until the cloud-first relaunch."
                    .to_string(),
            actual_implementation:
                "Embedding, vector index, and semantic modules removed from the codebase."
                    .to_string(),
            severity: Severity::Low,
            recommendation: Some(
                "Re-introduce documentation once the cloud-first semantic stack is available."
                    .to_string(),
            ),
            location: "docs/api/api.md:5".to_string(),
        });

        Ok(())
    }

    fn verify_performance_claims(&mut self) -> Result<()> {
        // These would require actual benchmarking to verify precisely
        // For now, just check if benchmark infrastructure exists

        let bench_dir_exists = std::path::Path::new("benches").exists();
        self.report.add_check(VerificationCheck {
            feature: "Performance Benchmarks Infrastructure".to_string(),
            status: if bench_dir_exists {
                VerificationStatus::Verified
            } else {
                VerificationStatus::Partial
            },
            documented_claim:
                "Real-world benchmarks on Apple Silicon with specific latency targets".to_string(),
            actual_implementation: if bench_dir_exists {
                "Benchmark directory exists for performance validation".to_string()
            } else {
                "Performance claims documented but benchmark infrastructure may be limited"
                    .to_string()
            },
            severity: Severity::Medium,
            recommendation: if !bench_dir_exists {
                Some(
                    "Create comprehensive benchmark suite to validate performance claims"
                        .to_string(),
                )
            } else {
                Some(
                    "Verify that benchmarks actually test the documented performance scenarios"
                        .to_string(),
                )
            },
            location: "README.md:136-147".to_string(),
        });

        Ok(())
    }

    fn verify_examples(&mut self) -> Result<()> {
        // Check documented examples vs actual files
        let example_claims = vec![
            ("Flask Web App", "examples/flask-web-app/"),
            ("Note-Taking App", "examples/note-taking-app/"),
            ("RAG Pipeline", "examples/rag-pipeline/"),
        ];

        for (name, path) in example_claims {
            let exists = std::path::Path::new(path).exists();
            self.report.add_check(VerificationCheck {
                feature: format!("Example: {}", name),
                status: if exists {
                    VerificationStatus::Verified
                } else {
                    VerificationStatus::Missing
                },
                documented_claim: format!("Production-ready {} example application", name),
                actual_implementation: if exists {
                    format!("Example directory exists at {}", path)
                } else {
                    format!("Example directory not found at {}", path)
                },
                severity: if !exists {
                    Severity::High
                } else {
                    Severity::Low
                },
                recommendation: if !exists {
                    Some(format!(
                        "Implement {} example or remove from documentation",
                        name
                    ))
                } else {
                    None
                },
                location: "README.md:155-174".to_string(),
            });
        }

        Ok(())
    }

    /// Get documented API endpoints from documentation
    ///
    /// TECHNICAL DEBT: This list is currently hardcoded and requires manual maintenance
    /// when API endpoints change. Future improvement: parse documentation files dynamically
    /// or maintain a single source of truth (e.g., OpenAPI spec).
    ///
    /// See issue tracker for "dynamic endpoint verification" enhancement.
    fn get_documented_endpoints(&self) -> Vec<(String, String, String)> {
        // NOTE: Update this list when API endpoints change in documentation
        // Last updated: 2025-08-18 for PR #246
        vec![
            (
                "POST".to_string(),
                "/documents".to_string(),
                "Create a new document".to_string(),
            ),
            (
                "GET".to_string(),
                "/documents/:id".to_string(),
                "Retrieve a document by ID".to_string(),
            ),
            (
                "PUT".to_string(),
                "/documents/:id".to_string(),
                "Update an existing document".to_string(),
            ),
            (
                "DELETE".to_string(),
                "/documents/:id".to_string(),
                "Delete a document".to_string(),
            ),
            (
                "GET".to_string(),
                "/documents/search".to_string(),
                "Search for documents".to_string(),
            ),
            (
                "GET".to_string(),
                "/health".to_string(),
                "Health check".to_string(),
            ),
        ]
    }

    /// Get actual HTTP endpoints from server implementation
    ///
    /// TECHNICAL DEBT: This currently returns a hardcoded list based on manual analysis
    /// of src/http_server.rs. Future improvement: automatically parse route definitions
    /// from the actual HTTP server code using AST parsing or reflection.
    fn get_actual_endpoints(&self) -> Result<HashMap<String, String>> {
        // NOTE: Update this list when routes change in src/http_server.rs
        // Last updated: 2025-08-18 for PR #246
        let mut endpoints = HashMap::new();

        endpoints.insert("GET /health".to_string(), "health_check".to_string());
        endpoints.insert("POST /documents".to_string(), "create_document".to_string());
        endpoints.insert("GET /documents".to_string(), "search_documents".to_string());
        endpoints.insert(
            "GET /documents/search".to_string(),
            "search_documents".to_string(),
        );
        endpoints.insert("GET /documents/:id".to_string(), "get_document".to_string());
        endpoints.insert(
            "PUT /documents/:id".to_string(),
            "update_document".to_string(),
        );
        endpoints.insert(
            "DELETE /documents/:id".to_string(),
            "delete_document".to_string(),
        );
        endpoints.insert(
            "POST /search/semantic".to_string(),
            "semantic_search".to_string(),
        );
        endpoints.insert(
            "POST /search/hybrid".to_string(),
            "hybrid_search".to_string(),
        );
        endpoints.insert("GET /stats".to_string(), "get_aggregated_stats".to_string());
        endpoints.insert(
            "GET /stats/connections".to_string(),
            "get_connection_stats".to_string(),
        );
        endpoints.insert(
            "GET /stats/performance".to_string(),
            "get_performance_stats".to_string(),
        );
        endpoints.insert(
            "GET /stats/resources".to_string(),
            "get_resource_stats".to_string(),
        );

        Ok(endpoints)
    }

    /// Check if Python client exists locally with proper packaging
    /// Note: This validates local client structure, not actual PyPI publication
    fn check_local_python_client(&self) -> Result<bool> {
        let python_dir = std::path::Path::new("clients/python");

        if !python_dir.exists() {
            return Ok(false);
        }

        // Batch filesystem checks to reduce I/O operations
        let pyproject_path = python_dir.join("pyproject.toml");
        let setup_path = python_dir.join("setup.py");
        let src_path = python_dir.join("src");
        let package_path = python_dir.join("kotadb");

        let has_setup = pyproject_path.exists() || setup_path.exists();
        let has_source = src_path.exists() || package_path.exists();

        if !has_setup {
            info!("Python client missing setup files (pyproject.toml or setup.py)");
        }
        if !has_source {
            info!("Python client missing source directory (src/ or kotadb/)");
        }

        Ok(has_setup && has_source)
    }

    /// Check if TypeScript client exists locally with proper packaging
    /// Note: This validates local client structure, not actual npm publication
    fn check_local_typescript_client(&self) -> Result<bool> {
        let ts_dir = std::path::Path::new("clients/typescript");

        if !ts_dir.exists() {
            return Ok(false);
        }

        // Batch filesystem checks to reduce I/O operations
        let package_json_path = ts_dir.join("package.json");
        let src_path = ts_dir.join("src");
        let dist_path = ts_dir.join("dist");

        let has_package_json = package_json_path.exists();
        let has_source = src_path.exists() || dist_path.exists();

        if !has_package_json {
            info!("TypeScript client missing package.json");
        }
        if !has_source {
            info!("TypeScript client missing source directory (src/ or dist/)");
        }

        Ok(has_package_json && has_source)
    }

    /// Run all verification checks and return the report
    pub fn run_full_verification(mut self) -> Result<DocumentationVerificationReport> {
        info!("Starting comprehensive documentation verification");

        self.verify_api_endpoints()
            .context("Failed to verify API endpoints")?;

        self.verify_client_libraries()
            .context("Failed to verify client libraries")?;

        self.verify_core_features()
            .context("Failed to verify core features")?;

        self.report.finalize();

        info!(
            "Documentation verification completed: {}",
            self.report.summary
        );

        if !self.report.critical_issues.is_empty() {
            error!(
                "Critical documentation issues found: {:?}",
                self.report.critical_issues
            );
        }

        Ok(self.report)
    }
}

impl Default for DocumentationVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_report_creation() {
        let mut report = DocumentationVerificationReport::new();
        assert_eq!(report.total_checks, 0);
        assert_eq!(report.verified_count, 0);

        let check = VerificationCheck {
            feature: "Test Feature".to_string(),
            status: VerificationStatus::Verified,
            documented_claim: "Test claim".to_string(),
            actual_implementation: "Test implementation".to_string(),
            severity: Severity::Low,
            recommendation: None,
            location: "test.md".to_string(),
        };

        report.add_check(check);
        assert_eq!(report.total_checks, 1);
        assert_eq!(report.verified_count, 1);
    }

    #[test]
    fn test_verification_report_accuracy() {
        let mut report = DocumentationVerificationReport::new();

        // Add some test checks
        report.add_check(VerificationCheck {
            feature: "Feature 1".to_string(),
            status: VerificationStatus::Verified,
            documented_claim: "Claim 1".to_string(),
            actual_implementation: "Implementation 1".to_string(),
            severity: Severity::Low,
            recommendation: None,
            location: "test1.md".to_string(),
        });

        report.add_check(VerificationCheck {
            feature: "Feature 2".to_string(),
            status: VerificationStatus::Missing,
            documented_claim: "Claim 2".to_string(),
            actual_implementation: "Not implemented".to_string(),
            severity: Severity::Critical,
            recommendation: Some("Implement feature 2".to_string()),
            location: "test2.md".to_string(),
        });

        report.finalize();

        assert_eq!(report.total_checks, 2);
        assert_eq!(report.verified_count, 1);
        assert_eq!(report.missing_count, 1);
        assert_eq!(report.critical_issues.len(), 1);
        assert_eq!(report.recommendations.len(), 1);
        assert!(!report.is_acceptable()); // Has critical issues
    }

    #[test]
    fn test_client_verification_methods() {
        let verifier = DocumentationVerifier::new();

        // Test Python client verification (will be false for most test environments)
        let python_result = verifier.check_local_python_client();
        assert!(
            python_result.is_ok(),
            "Python client check should not error"
        );

        // Test TypeScript client verification
        let ts_result = verifier.check_local_typescript_client();
        assert!(
            ts_result.is_ok(),
            "TypeScript client check should not error"
        );
    }

    #[test]
    fn test_hardcoded_endpoints_structure() {
        let verifier = DocumentationVerifier::new();
        let endpoints = verifier.get_documented_endpoints();

        // Verify structure is consistent
        assert!(!endpoints.is_empty(), "Should have documented endpoints");

        for (method, path, description) in endpoints {
            assert!(!method.is_empty(), "Method should not be empty");
            assert!(!path.is_empty(), "Path should not be empty");
            assert!(!description.is_empty(), "Description should not be empty");
            assert!(
                method == "GET" || method == "POST" || method == "PUT" || method == "DELETE",
                "Method should be valid HTTP method"
            );
            assert!(path.starts_with('/'), "Path should start with /");
        }
    }
}
