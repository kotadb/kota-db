#!/usr/bin/env cargo run --bin
//! # Meeting Notes System Example
//!
//! This example demonstrates using KotaDB for meeting notes management.
//! It shows:
//! - Meeting note organization and storage
//! - Temporal queries (find meetings by date/time)
//! - Action item tracking across meetings
//! - Participant and topic relationship mapping
//! - Follow-up and decision tracking
//!
//! ## Usage
//! ```bash
//! cargo run --example 03_meeting_notes_system
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
    println!("📅 Meeting Notes System - KotaDB Example");
    println!("========================================\n");

    // Create storage and indices for meeting data
    let mut storage = create_file_storage("./examples-data/meeting-notes", Some(1000)).await?;
    let mut primary_index =
        create_primary_index("./examples-data/meeting-primary", Some(1000)).await?;
    let mut search_index =
        create_trigram_index("./examples-data/meeting-search", Some(1000)).await?;

    println!("📝 Setting up meeting notes database...");

    // Create realistic meeting content
    let meeting_data = create_meeting_database();

    println!("📄 Adding {} meeting records...", meeting_data.len());

    // Insert all meeting documents
    for (i, (path, title, content, tags)) in meeting_data.iter().enumerate() {
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

        if i % 3 == 0 {
            println!("  📝 Added {} meetings...", i + 1);
        }
    }

    println!("✅ Meeting database populated!\n");

    // Demonstrate meeting workflows
    demonstrate_temporal_queries(&storage).await?;
    demonstrate_action_item_tracking(&storage, &search_index).await?;
    demonstrate_participant_analysis(&storage).await?;
    demonstrate_decision_tracking(&storage, &search_index).await?;
    demonstrate_meeting_analytics(&storage).await?;

    println!("\n🎉 Meeting Notes System example completed!");
    println!("   Meeting data: ./examples-data/meeting-notes/");

    Ok(())
}

/// Create realistic meeting notes content
fn create_meeting_database() -> Vec<(String, String, String, Vec<String>)> {
    vec![
        // Team meetings
        (
            "/meetings/2025/08/team-standup-2025-08-07.md".to_string(),
            "Team Standup - August 7, 2025".to_string(),
            r#"# Team Standup - August 7, 2025

**Date**: August 7, 2025  
**Time**: 9:00 AM - 9:30 AM  
**Type**: Daily Standup  
**Participants**: Alice (PM), Bob (Dev), Carol (Design), David (QA)  
**Location**: Conference Room A / Zoom Hybrid  

## Agenda
1. Yesterday's accomplishments
2. Today's plans
3. Blockers and impediments
4. Quick updates

## Updates

### Alice (Project Manager)
**Yesterday**: 
- Completed user story prioritization for Sprint 12
- Met with stakeholders about Q4 roadmap
- Reviewed QA testing results

**Today**: 
- Sprint planning preparation
- Budget review meeting with finance
- Customer feedback analysis

**Blockers**: None

### Bob (Senior Developer)
**Yesterday**: 
- Fixed critical bug in authentication service
- Code review for 3 pull requests
- Pair programming session with new team member

**Today**: 
- Implement user notification system
- Database migration for new features
- Technical documentation update

**Blockers**: Waiting for API specifications from external team

## Action Items
- [ ] **Alice**: Get approval for user research budget by Friday
- [ ] **Bob**: Follow up with external team on API specs (by EOD)
- [ ] **Team**: Sprint planning session tomorrow at 2 PM

## Quick Decisions
- Move user notification feature to Sprint 13 due to API dependency
- Approve overtime for critical bug fixes this week
- Schedule architecture review meeting for next week

## Notes
- Good progress on Sprint 12 objectives (85% complete)
- Customer satisfaction scores improved this quarter
- New team member onboarding going well
"#.to_string(),
            vec!["meeting".to_string(), "standup".to_string(), "team".to_string(), "daily".to_string()],
        ),

        (
            "/meetings/2025/08/product-planning-2025-08-05.md".to_string(),
            "Product Planning Meeting - August 5, 2025".to_string(),
            r#"# Product Planning Meeting - August 5, 2025

**Date**: August 5, 2025  
**Time**: 2:00 PM - 4:00 PM  
**Type**: Product Planning  
**Participants**: Alice (PM), Sarah (Product Owner), Bob (Tech Lead), Mike (Marketing), Lisa (Sales)  
**Location**: Executive Conference Room  

## Agenda
1. Q3 Performance Review
2. Q4 Product Roadmap
3. Feature Prioritization
4. Resource Allocation
5. Customer Feedback Analysis

## Q3 Performance Review

### Key Metrics
- **Monthly Active Users**: 245,000 (+15% from Q2)
- **Customer Satisfaction**: 4.2/5.0 (+0.3 from Q2)
- **Feature Adoption**: 78% for new dashboard
- **Revenue Growth**: 22% quarter-over-quarter

### Major Accomplishments
- ✅ Mobile app launch (iOS and Android)
- ✅ Advanced analytics dashboard
- ✅ Single sign-on integration
- ✅ API v3 with improved performance

## Q4 Product Roadmap

### Planned Features
- [ ] **Real-time Collaboration** (Oct 2025)
- [ ] **Advanced Security** (Nov 2025)
- [ ] **API Marketplace** (Dec 2025)

## Action Items
- [ ] **Sarah**: Create detailed feature specifications
- [ ] **Bob**: Technical architecture design for collaboration
- [ ] **Mike**: Customer communication about Q4 roadmap

## Decisions Made
1. **Q4 Budget**: $2.1M approved for development
2. **Hiring**: Add 2 senior developers by September
3. **Priorities**: Focus on collaboration and security first
"#.to_string(),
            vec!["meeting".to_string(), "product".to_string(), "planning".to_string(), "roadmap".to_string()],
        ),

        // Client meetings
        (
            "/meetings/2025/08/client-acme-corp-2025-08-06.md".to_string(),
            "Client Meeting - ACME Corp - August 6, 2025".to_string(),
            r#"# Client Meeting - ACME Corp - August 6, 2025

**Date**: August 6, 2025  
**Time**: 10:00 AM - 11:30 AM  
**Type**: Client Check-in  
**Client**: ACME Corporation  
**Participants**: 
- **Our Team**: Alice (Account Manager), Bob (Technical Lead), Carol (Customer Success)
- **Client Team**: John Smith (CTO), Mary Johnson (Project Manager), Dave Wilson (Lead Developer)

## Meeting Objectives
1. Review Q3 implementation progress
2. Address current technical challenges
3. Plan Q4 integration roadmap
4. Discuss contract renewal terms

## Implementation Progress Review

### Completed Milestones ✅
- **Phase 1**: User authentication integration (June 2025)
- **Phase 2**: Data migration from legacy system (July 2025)
- **Phase 3**: Basic workflow automation (August 2025)

### Current Status
- **Overall Progress**: 75% complete, on schedule
- **User Adoption**: 180/250 target users active
- **Performance**: Meeting all SLA requirements

## Technical Discussion

### Current Challenges
- API rate limiting during peak hours
- Large file processing timeout errors
- Report generation performance issues

### Solutions Approved
- Upgrade to premium API tier
- Implement chunked upload mechanism
- Database query optimization for reports

## Action Items
- [ ] **Bob**: Emergency fix for large file processing
- [ ] **Alice**: Process API premium tier upgrade
- [ ] **Carol**: Schedule additional user training sessions

## Contract Renewal Discussion
- Current contract expires December 31, 2025
- 2-year extension with volume discounts proposed
- 15% discount for early renewal commitment
"#.to_string(),
            vec!["meeting".to_string(), "client".to_string(), "acme".to_string(), "technical".to_string()],
        ),

        // Architecture review
        (
            "/meetings/2025/08/architecture-review-2025-08-15.md".to_string(),
            "Architecture Review - Real-time Collaboration - August 15, 2025".to_string(),
            r#"# Architecture Review - Real-time Collaboration - August 15, 2025

**Date**: August 15, 2025  
**Time**: 2:00 PM - 4:00 PM  
**Type**: Architecture Review  
**Project**: Real-time Collaboration Features  
**Participants**: Bob (Tech Lead), Sarah (Architect), Mike (Senior Dev), Carol (UX), Dave (DevOps)

## Review Scope
Comprehensive architecture review for real-time collaborative editing feature scheduled for Q4 2025 release.

## Proposed Architecture

### Core Components
1. **WebSocket Gateway**: Real-time connection management
2. **Operational Transform Engine**: Conflict resolution
3. **Document State Service**: Centralized document management
4. **Presence Service**: User cursor and selection tracking

### Technology Stack
- **Backend**: Node.js + Express + Socket.io
- **Real-time Engine**: Custom operational transformation
- **Database**: PostgreSQL + Redis for caching
- **Message Queue**: Apache Kafka for event streaming

## Technical Decisions

### ✅ Approved Decisions
1. **Overall Architecture**: Microservices with OT approach
2. **Technology Stack**: Node.js + Rust + PostgreSQL + Redis
3. **Timeline**: 4-month implementation schedule
4. **Team Assignment**: Bob as technical lead, 4-person dev team

### 📋 Pending Decisions
- Third-party library selection (ShareJS vs custom implementation)
- Geographic replication strategy and regions
- Beta testing program scope and timeline

## Implementation Plan

### Phase 1: Foundation (Month 1)
- Set up development environment
- Implement basic WebSocket connectivity
- Create document data model
- Basic conflict resolution prototype

### Phase 2: Core Features (Month 2)
- Operational transformation implementation
- Multi-user editing functionality
- Presence indicators and user cursors
- Basic version history

### Phase 3: Performance & Scale (Month 3)
- Performance optimization
- Load testing and tuning
- Monitoring and alerting setup
- Security hardening

### Phase 4: Production Ready (Month 4)
- Beta testing with select customers
- Bug fixes and stability improvements
- Documentation and training materials
- Production deployment

## Success Metrics
- **Latency**: <100ms operation acknowledgment
- **Uptime**: 99.9% availability
- **Adoption**: 50% of active users try feature within 30 days
- **Satisfaction**: >4.0/5.0 user rating

## Risk Assessment
- **Technical**: Real-time systems complexity
- **Timeline**: Aggressive 4-month schedule
- **Team Capacity**: Limited experience with real-time systems

## Action Items
- [ ] **Bob**: Finalize technical specification document
- [ ] **Sarah**: Security review of proposed architecture
- [ ] **Mike**: Resource allocation and team assignments
- [ ] **Carol**: UX wireframes for collaborative editing

## Next Steps
- Weekly technical check-ins starting August 22
- Architecture review meeting scheduled for August 27
- Customer beta program planning session August 30
"#.to_string(),
            vec!["meeting".to_string(), "architecture".to_string(), "review".to_string(), "real-time".to_string(), "collaboration".to_string()],
        ),
    ]
}

/// Demonstrate temporal queries for meeting analysis
async fn demonstrate_temporal_queries(storage: &impl Storage) -> Result<()> {
    println!("⏰ Temporal Meeting Analysis");
    println!("============================\n");

    let all_docs = storage.list_all().await?;
    let meetings: Vec<_> = all_docs
        .iter()
        .filter(|doc| doc.tags.iter().any(|tag| tag.as_str() == "meeting"))
        .collect();

    // Meetings by time period
    println!("1. 📅 Meetings by time period:");
    let now = Utc::now();
    let last_week = now - Duration::days(7);
    let last_month = now - Duration::days(30);

    let recent_meetings: Vec<_> = meetings
        .iter()
        .filter(|doc| doc.created_at > last_week)
        .collect();
    let monthly_meetings: Vec<_> = meetings
        .iter()
        .filter(|doc| doc.created_at > last_month)
        .collect();

    println!("   📊 Last 7 days: {} meetings", recent_meetings.len());
    println!("   📊 Last 30 days: {} meetings", monthly_meetings.len());
    println!("   📊 Total meetings: {} meetings", meetings.len());
    println!();

    // Meeting frequency analysis
    println!("2. 📈 Meeting frequency by type:");
    let mut meeting_types = HashMap::new();
    for doc in &meetings {
        for tag in &doc.tags {
            if tag.as_str() != "meeting" {
                *meeting_types.entry(tag.as_str().to_string()).or_insert(0) += 1;
            }
        }
    }

    for (meeting_type, count) in meeting_types.iter() {
        println!("   📋 {meeting_type}: {count} meetings");
    }
    println!();

    // Recent activity
    println!("3. 🔄 Recent meeting activity:");
    let mut recent_sorted = recent_meetings.clone();
    recent_sorted.sort_by_key(|doc| std::cmp::Reverse(doc.created_at));

    for doc in recent_sorted.iter().take(3) {
        println!(
            "   📅 {} - {} ({})",
            doc.created_at.format("%m-%d"),
            doc.title.as_str(),
            doc.tags
                .iter()
                .filter(|tag| tag.as_str() != "meeting")
                .map(|tag| tag.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    println!();

    Ok(())
}

/// Demonstrate action item tracking across meetings
async fn demonstrate_action_item_tracking(
    storage: &impl Storage,
    search_index: &impl Index,
) -> Result<()> {
    println!("✅ Action Item Tracking");
    println!("=======================\n");

    // Find all action items across meetings
    let query1 = Query::new(Some("action items".to_string()), None, None, 10)?;
    let action_results = search_index.search(&query1).await?;

    let query2 = Query::new(Some("TODO".to_string()), None, None, 10)?;
    let todo_results = search_index.search(&query2).await?;

    println!("1. 📋 Action items found in meetings:");
    println!("   🔍 'action items' mentions: {}", action_results.len());
    println!("   🔍 'TODO' mentions: {}", todo_results.len());
    println!();

    // Analyze action item completion
    let all_docs = storage.list_all().await?;
    let mut total_action_items = 0;
    let mut completed_items = 0;
    let mut pending_items = 0;

    for doc in &all_docs {
        if doc.tags.iter().any(|tag| tag.as_str() == "meeting") {
            let content = String::from_utf8_lossy(&doc.content);
            let lines: Vec<&str> = content.lines().collect();

            for line in lines {
                if line.contains("- [x]") {
                    completed_items += 1;
                    total_action_items += 1;
                } else if line.contains("- [ ]") {
                    pending_items += 1;
                    total_action_items += 1;
                }
            }
        }
    }

    println!("2. 📊 Action item completion status:");
    if total_action_items > 0 {
        let completion_rate = (completed_items as f64 / total_action_items as f64) * 100.0;
        println!("   ✅ Completed: {completed_items} items ({completion_rate:.1}%)");
        println!(
            "   ⏳ Pending: {pending_items} items ({:.1}%)",
            100.0 - completion_rate
        );
        println!("   📈 Total tracked: {total_action_items} items");
    }
    println!();

    // Find high-priority action items
    println!("3. ⚠️  High-priority action items:");
    let priority_keywords = vec!["urgent", "critical", "ASAP", "emergency"];

    for doc in &all_docs {
        if doc.tags.iter().any(|tag| tag.as_str() == "meeting") {
            let content = String::from_utf8_lossy(&doc.content);
            for keyword in &priority_keywords {
                if content.to_lowercase().contains(&keyword.to_lowercase()) {
                    println!(
                        "   🚨 Priority item in: {} ({})",
                        doc.title.as_str(),
                        doc.created_at.format("%m-%d")
                    );
                    break;
                }
            }
        }
    }
    println!();

    Ok(())
}

/// Demonstrate participant and relationship analysis
async fn demonstrate_participant_analysis(storage: &impl Storage) -> Result<()> {
    println!("👥 Participant Analysis");
    println!("=======================\n");

    let all_docs = storage.list_all().await?;
    let meetings: Vec<_> = all_docs
        .iter()
        .filter(|doc| doc.tags.iter().any(|tag| tag.as_str() == "meeting"))
        .collect();

    // Extract participants from meeting content
    let mut participant_frequency = HashMap::new();
    let mut meeting_types = HashMap::new();

    for doc in &meetings {
        let content = String::from_utf8_lossy(&doc.content);

        // Count participant mentions (simplified)
        let participants = vec!["Alice", "Bob", "Carol", "David", "Sarah", "Mike", "Lisa"];
        for participant in &participants {
            if content.contains(participant) {
                *participant_frequency
                    .entry(participant.to_string())
                    .or_insert(0) += 1;
            }
        }

        // Categorize meeting types
        for tag in &doc.tags {
            if tag.as_str() != "meeting" {
                *meeting_types.entry(tag.as_str().to_string()).or_insert(0) += 1;
            }
        }
    }

    println!("1. 👤 Most active meeting participants:");
    let mut sorted_participants: Vec<_> = participant_frequency.iter().collect();
    sorted_participants.sort_by_key(|(_, &count)| std::cmp::Reverse(count));

    for (participant, count) in sorted_participants.iter().take(5) {
        println!("   👤 {participant}: {count} meetings");
    }
    println!();

    println!("2. 📋 Meeting distribution by type:");
    let mut sorted_types: Vec<_> = meeting_types.iter().collect();
    sorted_types.sort_by_key(|(_, &count)| std::cmp::Reverse(count));

    for (meeting_type, count) in sorted_types.iter() {
        println!("   📊 {meeting_type}: {count} meetings");
    }
    println!();

    // Meeting collaboration patterns
    println!("3. 🤝 Collaboration patterns:");
    let standup_meetings = meetings
        .iter()
        .filter(|doc| doc.tags.iter().any(|tag| tag.as_str() == "standup"))
        .count();
    let planning_meetings = meetings
        .iter()
        .filter(|doc| doc.tags.iter().any(|tag| tag.as_str() == "planning"))
        .count();
    let client_meetings = meetings
        .iter()
        .filter(|doc| doc.tags.iter().any(|tag| tag.as_str() == "client"))
        .count();

    println!("   📅 Regular standups: {standup_meetings} meetings");
    println!("   📋 Planning sessions: {planning_meetings} meetings");
    println!("   👥 Client meetings: {client_meetings} meetings");
    println!();

    Ok(())
}

/// Demonstrate decision tracking across meetings
async fn demonstrate_decision_tracking(
    storage: &impl Storage,
    search_index: &impl Index,
) -> Result<()> {
    println!("🎯 Decision Tracking");
    println!("====================\n");

    // Find decisions across meetings
    let query1 = Query::new(Some("decision".to_string()), None, None, 10)?;
    let decision_results = search_index.search(&query1).await?;

    let query2 = Query::new(Some("approved".to_string()), None, None, 10)?;
    let approved_results = search_index.search(&query2).await?;

    let query3 = Query::new(Some("pending".to_string()), None, None, 10)?;
    let pending_results = search_index.search(&query3).await?;

    println!("1. 📊 Decision-making activity:");
    println!(
        "   🎯 Decision mentions: {} documents",
        decision_results.len()
    );
    println!("   ✅ Approved items: {} documents", approved_results.len());
    println!("   ⏳ Pending items: {} documents", pending_results.len());
    println!();

    // Analyze decision types
    let all_docs = storage.list_all().await?;
    let mut decision_categories = HashMap::new();

    for doc in &all_docs {
        if doc.tags.iter().any(|tag| tag.as_str() == "meeting") {
            let content = String::from_utf8_lossy(&doc.content);

            // Categorize decisions (simplified)
            if content.contains("technical") || content.contains("architecture") {
                *decision_categories
                    .entry("Technical".to_string())
                    .or_insert(0) += 1;
            }
            if content.contains("budget") || content.contains("resource") {
                *decision_categories
                    .entry("Resource".to_string())
                    .or_insert(0) += 1;
            }
            if content.contains("product") || content.contains("feature") {
                *decision_categories
                    .entry("Product".to_string())
                    .or_insert(0) += 1;
            }
            if content.contains("process") || content.contains("workflow") {
                *decision_categories
                    .entry("Process".to_string())
                    .or_insert(0) += 1;
            }
        }
    }

    println!("2. 📋 Decision categories:");
    for (category, count) in decision_categories.iter() {
        println!("   🎯 {category}: {count} meetings");
    }
    println!();

    // Recent decisions
    println!("3. 🔄 Recent decisions (by meeting date):");
    let meetings: Vec<_> = all_docs
        .iter()
        .filter(|doc| doc.tags.iter().any(|tag| tag.as_str() == "meeting"))
        .collect();

    let mut recent_meetings = meetings.clone();
    recent_meetings.sort_by_key(|doc| std::cmp::Reverse(doc.created_at));

    for doc in recent_meetings.iter().take(3) {
        let content = String::from_utf8_lossy(&doc.content);
        if content.contains("decision") || content.contains("approved") {
            println!(
                "   📅 {} - {} (contains decisions)",
                doc.created_at.format("%m-%d"),
                doc.title.as_str()
            );
        }
    }
    println!();

    Ok(())
}

/// Demonstrate meeting analytics and insights
async fn demonstrate_meeting_analytics(storage: &impl Storage) -> Result<()> {
    println!("📈 Meeting Analytics");
    println!("===================\n");

    let all_docs = storage.list_all().await?;
    let meetings: Vec<_> = all_docs
        .iter()
        .filter(|doc| doc.tags.iter().any(|tag| tag.as_str() == "meeting"))
        .collect();

    // Meeting volume analysis
    println!("1. 📊 Meeting volume statistics:");
    let total_meetings = meetings.len();
    let avg_meetings_per_week = total_meetings as f64 / 4.0; // Assuming 4 weeks of data

    println!("   📅 Total meetings recorded: {total_meetings}");
    println!("   📈 Average meetings per week: {avg_meetings_per_week:.1}");

    // Content analysis
    let total_content_size: usize = meetings.iter().map(|doc| doc.content.len()).sum();
    let avg_content_size = total_content_size as f64 / meetings.len() as f64;

    println!(
        "   📝 Total meeting content: {:.1} KB",
        total_content_size as f64 / 1024.0
    );
    println!("   📄 Average meeting size: {avg_content_size:.0} characters");
    println!();

    // Meeting effectiveness indicators
    println!("2. 🎯 Meeting effectiveness indicators:");

    let mut meetings_with_outcomes = 0;
    let mut meetings_with_actions = 0;
    let mut meetings_with_decisions = 0;

    for doc in &meetings {
        let content = String::from_utf8_lossy(&doc.content);

        if content.contains("action") || content.contains("TODO") {
            meetings_with_actions += 1;
        }
        if content.contains("decision") || content.contains("approved") {
            meetings_with_decisions += 1;
        }
        if content.contains("outcome") || content.contains("result") {
            meetings_with_outcomes += 1;
        }
    }

    let action_rate = (meetings_with_actions as f64 / meetings.len() as f64) * 100.0;
    let decision_rate = (meetings_with_decisions as f64 / meetings.len() as f64) * 100.0;
    let outcome_rate = (meetings_with_outcomes as f64 / meetings.len() as f64) * 100.0;

    println!("   ✅ Meetings with action items: {action_rate:.1}%");
    println!("   🎯 Meetings with decisions: {decision_rate:.1}%");
    println!("   📈 Meetings with clear outcomes: {outcome_rate:.1}%");
    println!();

    // Team engagement analysis
    println!("3. 👥 Team engagement patterns:");

    let standup_meetings = meetings
        .iter()
        .filter(|doc| doc.tags.iter().any(|tag| tag.as_str() == "standup"))
        .count();
    let planning_meetings = meetings
        .iter()
        .filter(|doc| doc.tags.iter().any(|tag| tag.as_str() == "planning"))
        .count();
    let client_meetings = meetings
        .iter()
        .filter(|doc| doc.tags.iter().any(|tag| tag.as_str() == "client"))
        .count();

    println!("   📅 Daily standups: {standup_meetings} (team synchronization)");
    println!("   📋 Planning sessions: {planning_meetings} (strategic alignment)");
    println!("   👥 Client meetings: {client_meetings} (external collaboration)");

    // Calculate meeting diversity
    let structured_meetings = standup_meetings + planning_meetings + client_meetings;
    let meeting_diversity = (structured_meetings as f64 / meetings.len() as f64) * 100.0;
    println!("   📊 Meeting structure index: {meeting_diversity:.1}%");
    println!();

    Ok(())
}
