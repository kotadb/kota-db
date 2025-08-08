#!/usr/bin/env cargo run --bin
//! # Research Project Manager Example
//!
//! This example demonstrates using KotaDB for academic research management.
//! It shows:
//! - Paper and citation tracking
//! - Research note organization
//! - Literature review workflows
//! - Citation network analysis
//! - Progress tracking over time
//!
//! ## Usage
//! ```bash
//! cargo run --example 02_research_project_manager
//! ```

use anyhow::Result;
use chrono::{Duration, Utc};
use kotadb::{
    create_file_storage, create_primary_index, create_trigram_index, init_logging, DocumentBuilder,
    Index, Query, Storage, ValidatedPath,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = init_logging();
    println!("🔬 Research Project Manager - KotaDB Example");
    println!("=============================================\n");

    // Create storage and indices for research data
    let mut storage = create_file_storage("./examples-data/research-manager", Some(1000)).await?;
    let mut primary_index =
        create_primary_index("./examples-data/research-primary", Some(1000)).await?;
    let mut search_index =
        create_trigram_index("./examples-data/research-search", Some(1000)).await?;

    println!("📚 Setting up research project database...");

    // Create realistic research content
    let research_data = create_research_database();

    println!("📝 Adding {} research documents...", research_data.len());

    // Insert all research documents
    for (i, (path, title, content, tags)) in research_data.iter().enumerate() {
        let mut builder = DocumentBuilder::new()
            .path(path)?
            .title(title)?
            .content(content.as_bytes());

        for tag in tags {
            builder = builder.tag(tag)?;
        }

        let doc = builder.build()?;

        storage.insert(doc.clone()).await?;
        primary_index
            .insert(doc.id, ValidatedPath::new(path)?)
            .await?;
        search_index
            .insert(doc.id, ValidatedPath::new(path)?)
            .await?;

        if i % 5 == 0 {
            println!("  📄 Added {} documents...", i + 1);
        }
    }

    println!("✅ Research database populated!\n");

    // Demonstrate research workflows
    demonstrate_literature_review(&storage, &search_index).await?;
    demonstrate_citation_analysis(&storage).await?;
    demonstrate_progress_tracking(&storage).await?;
    demonstrate_research_queries(&storage, &search_index).await?;

    println!("\n🎉 Research Project Manager example completed!");
    println!("   Research data: ./examples-data/research-manager/");

    Ok(())
}

/// Create realistic research project content
fn create_research_database() -> Vec<(String, String, String, Vec<String>)> {
    vec![
        // Academic papers
        (
            "/papers/distributed-consensus-survey.md".to_string(),
            "Distributed Consensus Algorithms: A Comprehensive Survey".to_string(),
            r#"# Distributed Consensus Algorithms: A Comprehensive Survey

**Authors**: Smith, J., Johnson, A., Brown, M.  
**Journal**: ACM Computing Surveys  
**Year**: 2024  
**DOI**: 10.1145/3589334.3645123  
**Citations**: 47  

## Abstract
This survey provides a comprehensive overview of distributed consensus algorithms, from classical Paxos to modern Raft and beyond. We analyze theoretical foundations, practical implementations, and performance characteristics across different failure models.

## Key Contributions
1. Taxonomy of consensus algorithms by failure model
2. Performance comparison under various network conditions  
3. Implementation complexity analysis
4. Future research directions

## Notes
- Excellent overview of Raft vs Paxos trade-offs
- Good performance data for network partition scenarios
- Missing coverage of Byzantine fault tolerant algorithms
- Useful for understanding consensus landscape

## Related Work
- Lamport, L. "The Part-Time Parliament" (original Paxos paper)
- Ongaro, D. "In Search of an Understandable Consensus Algorithm" (Raft paper)
- Castro, M. "Practical Byzantine Fault Tolerance" (PBFT)

## Citations in My Work
- Used in Chapter 3 of dissertation for consensus background
- Referenced in paper on "Consensus in Edge Computing Environments"
- Helpful for understanding CAP theorem implications

## Keywords
consensus, distributed systems, Paxos, Raft, fault tolerance, replication
"#.to_string(),
            vec!["paper".to_string(), "consensus".to_string(), "distributed-systems".to_string(), "survey".to_string()],
        ),

        (
            "/papers/machine-learning-databases.md".to_string(),
            "Machine Learning for Database Query Optimization".to_string(),
            r#"# Machine Learning for Database Query Optimization

**Authors**: Chen, L., Williams, K., Davis, R.  
**Conference**: SIGMOD 2024  
**Year**: 2024  
**DOI**: 10.1145/3654321.3678901  
**Citations**: 23  

## Abstract
We present a machine learning approach to database query optimization that learns from past query execution patterns to predict optimal execution plans. Our method achieves 15-30% performance improvements over traditional cost-based optimizers.

## Key Findings
1. Neural networks can effectively predict query execution costs
2. Historical execution data provides valuable optimization signals
3. Adaptive learning improves performance over time
4. Works particularly well for complex analytical queries

## Technical Approach
- **Model**: Deep neural network with attention mechanism
- **Features**: Query structure, table statistics, index information
- **Training**: Online learning from query execution feedback
- **Integration**: Replaces cost model in PostgreSQL optimizer

## Experimental Results
- **Datasets**: TPC-H, TPC-DS, real production workloads
- **Improvement**: 15-30% reduction in query execution time
- **Overhead**: <5% optimization time increase
- **Adaptability**: Continues improving with more data

## Personal Notes
- Highly relevant to my thesis on adaptive database systems
- Could apply similar techniques to index selection
- Need to understand attention mechanism details
- Potential collaboration opportunity with database systems lab

## Questions for Authors
- How does the model handle novel query patterns?
- What happens when underlying data distribution changes?
- Can this approach work for transactional workloads?

## Related Work
- References foundational work on learned index structures
- Builds on classical query optimization literature
- Connects to broader ML for systems research area
"#.to_string(),
            vec!["paper".to_string(), "machine-learning".to_string(), "databases".to_string(), "optimization".to_string()],
        ),

        // Research notes
        (
            "/notes/consensus-algorithms-comparison.md".to_string(),
            "Consensus Algorithms Comparison Matrix".to_string(),
            r#"# Consensus Algorithms Comparison Matrix

## Overview
Detailed comparison of major consensus algorithms for my dissertation chapter on distributed coordination.

## Comparison Matrix

| Algorithm | Failure Model | Messages per Decision | Latency | Complexity | Production Use |
|-----------|---------------|----------------------|---------|------------|----------------|
| Paxos | Crash | 2 phases, 4+ msgs | 2 RTT | High | Google (Chubby) |
| Raft | Crash | 2+ msgs | 1 RTT | Medium | etcd, Consul |
| PBFT | Byzantine | O(n²) msgs | 3 phases | Very High | Hyperledger Fabric |
| HotStuff | Byzantine | O(n) msgs | 3 phases | High | Meta Diem |
| Tendermint | Byzantine | O(n²) msgs | Variable | High | Cosmos |

## Key Insights

### Paxos Family
- **Classic Paxos**: Theoretically elegant but complex to implement
- **Multi-Paxos**: More practical with leader optimization
- **Fast Paxos**: Reduces latency but increases message complexity
- **Vertical Paxos**: Handles configuration changes

### Raft Advantages
- Understandable algorithm design
- Strong leader model simplifies reasoning
- Excellent tooling and implementations
- Good performance for most use cases

### Byzantine Algorithms
- PBFT: First practical BFT algorithm
- HotStuff: Linear communication complexity
- Tendermint: Practical blockchain consensus
- FLP impossibility theorem implications

## Research Questions
1. Can we combine Raft's simplicity with Byzantine fault tolerance?
2. How do these algorithms perform in edge computing scenarios?
3. What are the implications for partial synchrony assumptions?

## Implementation Notes
- etcd (Raft): Production-ready, well-tested
- Consul (Raft): Good for service discovery use cases
- Chubby (Paxos): Google's lock service
- Hyperledger (PBFT): Permissioned blockchain networks

## Future Work
- Implement simplified BFT variant
- Performance analysis in edge environments
- Formal verification of safety properties
"#.to_string(),
            vec!["notes".to_string(), "consensus".to_string(), "comparison".to_string(), "research".to_string()],
        ),

        // Progress report
        (
            "/progress/dissertation-chapter-3.md".to_string(),
            "Dissertation Chapter 3: Progress Update".to_string(),
            r#"# Dissertation Chapter 3: Progress Update

**Chapter Title**: Distributed Consensus in Edge Computing Environments  
**Target Length**: 25-30 pages  
**Deadline**: September 30, 2025  
**Current Status**: 40% complete  

## Progress Summary

### ✅ Completed Sections
- [x] **3.1 Introduction** (3 pages)
  - Problem motivation for edge consensus
  - Research questions and contributions
  - Chapter organization

- [x] **3.2 Background** (4 pages)
  - Classical consensus algorithms (Paxos, Raft)
  - CAP theorem and consistency models
  - Edge computing characteristics

- [x] **3.3 Related Work** (5 pages)
  - Consensus in mobile ad-hoc networks
  - Blockchain consensus mechanisms
  - Geo-distributed consensus protocols

### 🚧 In Progress
- [ ] **3.4 Edge Consensus Model** (6 pages) - 60% complete
  - Network topology assumptions
  - Failure model for edge environments
  - Performance requirements

### 📋 Remaining Work
- [ ] **3.5 Algorithm Design** (7 pages)
  - Adaptive leader selection
  - Network-aware message routing
  - Byzantine fault tolerance extensions

- [ ] **3.6 Experimental Evaluation** (5 pages)
  - Simulation framework setup
  - Performance comparison with baselines
  - Analysis of results

- [ ] **3.7 Conclusion** (2 pages)
  - Summary of contributions
  - Limitations and future work

## Action Items
- [ ] **This Week**: Complete formal failure model by August 14
- [ ] **Next Month**: Run comprehensive simulations
- [ ] **Long Term**: Submit conference paper by October deadline
"#.to_string(),
            vec!["progress".to_string(), "dissertation".to_string(), "consensus".to_string(), "edge-computing".to_string()],
        ),

        // Meeting notes
        (
            "/meetings/advisor-meeting-2025-08-07.md".to_string(),
            "Advisor Meeting - August 7, 2025".to_string(),
            r#"# Advisor Meeting - August 7, 2025

**Date**: August 7, 2025  
**Participants**: Prof. Smith (advisor), Me  
**Duration**: 1 hour  
**Focus**: Dissertation Chapter 3 progress review  

## Discussion Points

### Chapter 3 Progress Review
- **Current Status**: 40% complete, on track for September deadline
- **Quality**: Writing quality is good, technical content solid
- **Scope**: May need to narrow focus for conference paper version

### Technical Feedback
- Strong motivation for edge-specific consensus protocols
- Good literature review covering relevant work
- Need more rigorous failure model definition
- Include energy consumption analysis

### Next Steps
- [ ] Formalize edge-specific failure model by August 14
- [ ] Add energy consumption to evaluation framework
- [ ] Research security implications of edge consensus
- [ ] Schedule follow-up meeting for August 21

## Key Insights
The edge computing angle is novel and important. Focus on what makes edge environments unique rather than trying to solve general consensus problems.
"#.to_string(),
            vec!["meeting".to_string(), "advisor".to_string(), "feedback".to_string(), "dissertation".to_string()],
        ),

        // Citation tracking
        (
            "/citations/influential-papers.md".to_string(),
            "Most Influential Papers in My Research".to_string(),
            r#"# Most Influential Papers in My Research

## Top 5 Most Cited Papers in My Work

### 1. Lamport, L. (1978) "Time, Clocks, and the Ordering of Events"
- **My Citations**: 15 times across 3 papers
- **Why Important**: Fundamental to understanding distributed systems
- **Key Concepts**: Logical clocks, happened-before relationship

### 2. Gilbert, S. & Lynch, N. (2002) "Brewer's Conjecture and the Feasibility of Consistent, Available, Partition-tolerant Web Services"
- **My Citations**: 12 times across 4 papers  
- **Why Important**: Formal proof of CAP theorem
- **Key Concepts**: Consistency, availability, partition tolerance trade-offs

### 3. Ongaro, D. & Ousterhout, J. (2014) "In Search of an Understandable Consensus Algorithm"
- **My Citations**: 18 times across 5 papers
- **Why Important**: Practical consensus algorithm design
- **Key Concepts**: Leader election, log replication, safety

### 4. DeCandia, G. et al. (2007) "Dynamo: Amazon's Highly Available Key-value Store"
- **My Citations**: 8 times across 2 papers
- **Why Important**: Real-world eventually consistent systems
- **Key Concepts**: Vector clocks, consistent hashing, gossip protocols

### 5. Castro, M. & Liskov, B. (1999) "Practical Byzantine Fault Tolerance"
- **My Citations**: 10 times across 3 papers
- **Why Important**: First practical BFT algorithm
- **Key Concepts**: Three-phase protocol, view changes, checkpoints

## Citation Network Analysis

### Core Research Areas
1. **Distributed Consensus** (45% of citations)
2. **Edge Computing** (25% of citations)  
3. **Database Systems** (20% of citations)
4. **Security & Byzantine Tolerance** (10% of citations)

### Most Cited Authors in My Work
1. **Leslie Lamport**: 28 citations (distributed systems theory)
2. **Nancy Lynch**: 15 citations (theoretical foundations)
3. **Diego Ongaro**: 18 citations (practical consensus)
4. **Barbara Liskov**: 12 citations (fault tolerance)

## Research Gaps and Opportunities
- Energy-aware consensus algorithms
- Mobile consensus in dynamic networks
- Byzantine behavior in edge environments
- Machine learning for adaptive protocols
"#.to_string(),
            vec!["citations".to_string(), "influential".to_string(), "research".to_string(), "analysis".to_string()],
        ),
    ]
}

/// Demonstrate literature review and paper discovery workflows
async fn demonstrate_literature_review(
    storage: &impl Storage,
    search_index: &impl Index,
) -> Result<()> {
    println!("📖 Literature Review Workflows");
    println!("==============================\n");

    // Find papers by topic
    println!("1. 🔍 Finding papers on 'consensus algorithms':");
    let query = Query::new(Some("consensus algorithms".to_string()), None, None, 10)?;
    let consensus_papers = search_index.search(&query).await?;
    for doc_id in consensus_papers.iter().take(3) {
        if let Some(doc) = storage.get(doc_id).await? {
            if doc.tags.iter().any(|tag| tag.as_str() == "paper") {
                println!("   📄 {} - {}", doc.title.as_str(), doc.path.as_str());
            }
        }
    }
    println!();

    // Find related work by tags
    println!("2. 🏷️  Papers tagged with 'distributed-systems':");
    let all_docs = storage.list_all().await?;
    let distributed_papers: Vec<_> = all_docs
        .iter()
        .filter(|doc| {
            doc.tags
                .iter()
                .any(|tag| tag.as_str() == "distributed-systems")
        })
        .collect();
    for doc in distributed_papers.iter().take(3) {
        println!("   📄 {} - {}", doc.title.as_str(), doc.path.as_str());
    }
    println!();

    // Search research notes
    println!("3. 📝 Research notes containing 'Byzantine':");
    let query = Query::new(Some("Byzantine".to_string()), None, None, 10)?;
    let byzantine_notes = search_index.search(&query).await?;
    for doc_id in byzantine_notes.iter().take(2) {
        if let Some(doc) = storage.get(doc_id).await? {
            if doc.tags.iter().any(|tag| tag.as_str() == "notes") {
                println!("   📝 {} - {}", doc.title.as_str(), doc.path.as_str());
            }
        }
    }
    println!();

    Ok(())
}

/// Demonstrate citation network analysis
async fn demonstrate_citation_analysis(storage: &impl Storage) -> Result<()> {
    println!("🔗 Citation Network Analysis");
    println!("============================\n");

    let all_docs = storage.list_all().await?;

    // Count papers by type
    let papers: Vec<_> = all_docs
        .iter()
        .filter(|doc| doc.tags.iter().any(|tag| tag.as_str() == "paper"))
        .collect();
    let notes: Vec<_> = all_docs
        .iter()
        .filter(|doc| doc.tags.iter().any(|tag| tag.as_str() == "notes"))
        .collect();
    let progress: Vec<_> = all_docs
        .iter()
        .filter(|doc| doc.tags.iter().any(|tag| tag.as_str() == "progress"))
        .collect();

    println!("📊 Research Database Statistics:");
    println!("   📄 Academic papers: {}", papers.len());
    println!("   📝 Research notes: {}", notes.len());
    println!("   📈 Progress reports: {}", progress.len());
    println!("   📚 Total documents: {}", all_docs.len());
    println!();

    // Analyze citation patterns
    println!("🔗 Citation Pattern Analysis:");
    let mut citation_counts = HashMap::new();
    for doc in &all_docs {
        let content = String::from_utf8_lossy(&doc.content);
        if content.contains("Lamport") {
            *citation_counts.entry("Lamport").or_insert(0) += 1;
        }
        if content.contains("Raft") {
            *citation_counts.entry("Raft").or_insert(0) += 1;
        }
        if content.contains("PBFT") || content.contains("Byzantine") {
            *citation_counts.entry("Byzantine").or_insert(0) += 1;
        }
        if content.contains("consensus") {
            *citation_counts.entry("Consensus").or_insert(0) += 1;
        }
    }

    for (topic, count) in citation_counts.iter() {
        println!("   🔗 {topic}: mentioned in {count} documents");
    }
    println!();

    Ok(())
}

/// Demonstrate research progress tracking
async fn demonstrate_progress_tracking(storage: &impl Storage) -> Result<()> {
    println!("📈 Research Progress Tracking");
    println!("=============================\n");

    let all_docs = storage.list_all().await?;

    // Find progress-related documents
    let progress_docs: Vec<_> = all_docs
        .iter()
        .filter(|doc| {
            doc.tags.iter().any(|tag| tag.as_str() == "progress")
                || doc.tags.iter().any(|tag| tag.as_str() == "meeting")
                || doc.path.as_str().contains("progress")
        })
        .collect();

    println!("📋 Recent Progress Updates:");
    for doc in progress_docs.iter().take(3) {
        println!(
            "   📅 {} - {} ({})",
            doc.created_at.format("%Y-%m-%d"),
            doc.title.as_str(),
            doc.path.as_str()
        );
    }
    println!();

    // Analyze completion status
    println!("✅ Project Status Analysis:");
    let mut total_tasks = 0;
    let mut completed_tasks = 0;

    for doc in &all_docs {
        let content = String::from_utf8_lossy(&doc.content);
        let lines: Vec<&str> = content.lines().collect();

        for line in lines {
            if line.contains("- [x]") {
                completed_tasks += 1;
                total_tasks += 1;
            } else if line.contains("- [ ]") {
                total_tasks += 1;
            }
        }
    }

    if total_tasks > 0 {
        let completion_rate = (completed_tasks as f64 / total_tasks as f64) * 100.0;
        println!(
            "   📊 Overall Progress: {completion_rate:.1}% ({completed_tasks}/{total_tasks} tasks completed)"
        );
    }

    println!("   🎯 Active Projects: Dissertation Chapter 3, Literature Review");
    println!("   ⏰ Upcoming Deadlines: September 30 (Chapter 3), October (Conference Paper)");
    println!();

    Ok(())
}

/// Demonstrate advanced research queries
async fn demonstrate_research_queries(
    storage: &impl Storage,
    search_index: &impl Index,
) -> Result<()> {
    println!("🔬 Advanced Research Queries");
    println!("============================\n");

    // Multi-keyword search
    println!("1. 🔍 Multi-concept search ('machine learning' + 'database'):");
    let query = Query::new(
        Some("machine learning database".to_string()),
        None,
        None,
        10,
    )?;
    let ml_db_results = search_index.search(&query).await?;
    for doc_id in ml_db_results.iter().take(2) {
        if let Some(doc) = storage.get(doc_id).await? {
            println!("   📄 {} - {}", doc.title.as_str(), doc.path.as_str());
        }
    }
    println!();

    // Time-based queries
    println!("2. ⏰ Recent research activity (last 30 days):");
    let now = Utc::now();
    let thirty_days_ago = now - Duration::days(30);

    let all_docs = storage.list_all().await?;
    let recent_docs: Vec<_> = all_docs
        .iter()
        .filter(|doc| doc.created_at > thirty_days_ago)
        .collect();

    for doc in recent_docs.iter().take(3) {
        println!(
            "   📅 {} - {} ({})",
            doc.created_at.format("%m-%d"),
            doc.title.as_str(),
            doc.path.as_str()
        );
    }
    println!();

    // Cross-reference analysis
    println!("3. 🔗 Cross-references between documents:");
    let mut reference_map: HashMap<String, Vec<String>> = HashMap::new();

    for doc in &all_docs {
        let content = String::from_utf8_lossy(&doc.content);
        let references = extract_document_references(&content);
        if !references.is_empty() {
            reference_map.insert(doc.title.as_str().to_string(), references);
        }
    }

    for (source, targets) in reference_map.iter().take(2) {
        println!("   📄 '{source}' references:");
        for target in targets.iter().take(3) {
            println!("      → {target}");
        }
    }
    println!();

    Ok(())
}

/// Extract document references from content (simplified)
fn extract_document_references(content: &str) -> Vec<String> {
    let mut references = Vec::new();

    // Look for common reference patterns
    if content.contains("Lamport") {
        references.push("Lamport's foundational work".to_string());
    }
    if content.contains("Raft") {
        references.push("Raft consensus algorithm".to_string());
    }
    if content.contains("PBFT") {
        references.push("Practical Byzantine Fault Tolerance".to_string());
    }
    if content.contains("Chapter 3") {
        references.push("Dissertation Chapter 3".to_string());
    }

    references
}
