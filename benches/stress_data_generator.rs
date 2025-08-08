#![allow(clippy::uninlined_format_args)]
// Stress Test Data Generator
// Generates realistic document datasets for comprehensive benchmarking

use anyhow::Result;
use kotadb::DocumentBuilder;
use rand::prelude::*;

/// Configuration for document generation
#[derive(Debug, Clone)]
pub struct DataGenConfig {
    /// Number of documents to generate
    pub count: usize,
    /// Base size for documents (will vary around this)
    pub base_size_bytes: usize,
    /// Size variation factor (0.0 to 1.0)
    pub size_variation: f64,
    /// Number of different topics/domains (used for future expansion)
    #[allow(dead_code)]
    pub topic_count: usize,
    /// Average number of tags per document
    pub avg_tags_per_doc: usize,
    /// Percentage of documents that reference others
    pub reference_percentage: f64,
    /// Random seed for reproducible generation
    pub seed: u64,
}

impl Default for DataGenConfig {
    fn default() -> Self {
        Self {
            count: 10_000,
            base_size_bytes: 5_000, // ~5KB average
            size_variation: 0.8,    // 80% variation
            topic_count: 20,
            avg_tags_per_doc: 3,
            reference_percentage: 0.3, // 30% have references
            seed: 42,
        }
    }
}

/// Realistic document generator for stress testing
pub struct StressDocumentGenerator {
    config: DataGenConfig,
    rng: StdRng,
    topics: Vec<DocumentTopic>,
    generated_paths: Vec<String>,
}

#[derive(Debug, Clone)]
struct DocumentTopic {
    name: String,
    tags: Vec<String>,
    content_templates: Vec<String>,
    path_prefixes: Vec<String>,
}

impl StressDocumentGenerator {
    pub fn new(config: DataGenConfig) -> Self {
        let rng = StdRng::seed_from_u64(config.seed);
        let topics = Self::create_realistic_topics();

        Self {
            config,
            rng,
            topics,
            generated_paths: Vec::new(),
        }
    }

    /// Generate a batch of realistic documents
    pub fn generate_documents(&mut self) -> Result<Vec<kotadb::contracts::Document>> {
        let mut documents = Vec::with_capacity(self.config.count);

        println!("🏭 Generating {} realistic documents...", self.config.count);

        for i in 0..self.config.count {
            if i % 1000 == 0 {
                println!("  📝 Generated {}/{} documents", i, self.config.count);
            }

            let doc = self.generate_single_document(i)?;
            documents.push(doc);
        }

        println!(
            "✅ Generated {} documents with realistic content",
            documents.len()
        );
        Ok(documents)
    }

    fn generate_single_document(&mut self, index: usize) -> Result<kotadb::contracts::Document> {
        // Select random topic
        let topic_index = self.rng.gen_range(0..self.topics.len());
        let topic = self.topics[topic_index].clone(); // Clone to avoid borrowing issues

        // Generate path
        let path_prefix_index = self.rng.gen_range(0..topic.path_prefixes.len());
        let path_prefix = &topic.path_prefixes[path_prefix_index];
        let filename = self.generate_filename(&topic.name, index);
        let path = format!("{path_prefix}/{filename}");
        self.generated_paths.push(path.clone());

        // Generate title
        let title = self.generate_title(&topic.name, index);

        // Generate content with realistic size
        let target_size = self.calculate_target_size();
        let content = self.generate_content(&topic, target_size);

        // Generate tags
        let tags = self.generate_tags(&topic);

        // Build document using the builder pattern
        let mut builder = DocumentBuilder::new()
            .path(&path)?
            .title(&title)?
            .content(content.into_bytes());

        // Add tags
        for tag in tags {
            builder = builder.tag(&tag)?;
        }

        builder.build()
    }

    fn calculate_target_size(&mut self) -> usize {
        let base = self.config.base_size_bytes as f64;
        let variation = base * self.config.size_variation;
        let random_factor = self.rng.gen::<f64>() * 2.0 - 1.0; // -1.0 to 1.0

        ((base + variation * random_factor).max(500.0) as usize).min(50_000) // 500B to 50KB
    }

    fn generate_filename(&mut self, topic: &str, index: usize) -> String {
        let patterns = [
            format!("{}-{:04}.md", topic.replace(" ", "-").to_lowercase(), index),
            format!(
                "{}-notes-{:04}.md",
                topic.replace(" ", "-").to_lowercase(),
                index
            ),
            format!(
                "{}-guide-{:04}.md",
                topic.replace(" ", "-").to_lowercase(),
                index
            ),
            format!(
                "{}-reference-{:04}.md",
                topic.replace(" ", "-").to_lowercase(),
                index
            ),
            format!(
                "{}-tutorial-{:04}.md",
                topic.replace(" ", "-").to_lowercase(),
                index
            ),
        ];

        patterns[self.rng.gen_range(0..patterns.len())].clone()
    }

    fn generate_title(&mut self, topic: &str, index: usize) -> String {
        let patterns = [
            format!("{topic} - Part {}", index % 10 + 1),
            format!("Understanding {topic}"),
            format!("{topic} Best Practices"),
            format!("Advanced {topic} Techniques"),
            format!("{topic} Tutorial"),
            format!("{topic} Reference Guide"),
            format!("Deep Dive into {topic}"),
            format!("{topic} Fundamentals"),
        ];

        patterns[self.rng.gen_range(0..patterns.len())].clone()
    }

    fn generate_content(&mut self, topic: &DocumentTopic, target_size: usize) -> String {
        let template =
            &topic.content_templates[self.rng.gen_range(0..topic.content_templates.len())];
        let mut content = template.clone();

        // Add random sections to reach target size
        while content.len() < target_size {
            let section = self.generate_content_section(topic);
            content.push_str(&format!("\n\n{section}"));

            // Prevent infinite loop
            if content.len() > target_size * 2 {
                break;
            }
        }

        // Add references to other documents if configured
        if self.rng.gen::<f64>() < self.config.reference_percentage
            && !self.generated_paths.is_empty()
        {
            let referenced_path =
                &self.generated_paths[self.rng.gen_range(0..self.generated_paths.len())];
            content.push_str(&format!(
                "\n\n## Related\n- See also: [{}]({referenced_path})",
                referenced_path.split('/').next_back().unwrap_or("document")
            ));
        }

        content
    }

    fn generate_content_section(&mut self, topic: &DocumentTopic) -> String {
        let sections = [
            format!("## Key Concepts in {}\n\nThis section explores the fundamental principles and core ideas.", topic.name),
            format!("### Implementation Details\n\nHere we dive into the practical aspects of working with {}.", topic.name),
            format!("## Best Practices\n\nBased on experience, these are the recommended approaches for {}.", topic.name),
            format!("### Common Pitfalls\n\nThese are frequent mistakes when working with {} and how to avoid them.", topic.name),
            format!("## Examples\n\n```\n// Example code demonstrating {} concepts\nfn example() {{\n    // Implementation here\n}}\n```", topic.name),
            format!("### Performance Considerations\n\nWhen optimizing {}, consider these performance factors.", topic.name),
            format!("## Advanced Topics\n\nFor experts in {}, these advanced concepts provide deeper insight.", topic.name),
            format!("### Troubleshooting\n\nCommon issues in {} and their solutions.", topic.name),
        ];

        sections[self.rng.gen_range(0..sections.len())].clone()
    }

    fn generate_tags(&mut self, topic: &DocumentTopic) -> Vec<String> {
        let mut tags = Vec::new();

        // Always include topic tags
        tags.extend(topic.tags.iter().cloned());

        // Add random additional tags
        let additional_tags = [
            "tutorial",
            "reference",
            "guide",
            "notes",
            "documentation",
            "advanced",
            "beginner",
            "intermediate",
            "production",
            "development",
            "performance",
            "security",
            "testing",
            "architecture",
            "design",
        ];

        let tag_count = self.rng.gen_range(1..=self.config.avg_tags_per_doc + 2);
        while tags.len() < tag_count {
            let tag = additional_tags[self.rng.gen_range(0..additional_tags.len())];
            if !tags.contains(&tag.to_string()) {
                tags.push(tag.to_string());
            }
        }

        tags
    }

    fn create_realistic_topics() -> Vec<DocumentTopic> {
        vec![
            DocumentTopic {
                name: "Rust Programming".to_string(),
                tags: vec!["rust".to_string(), "programming".to_string(), "systems".to_string()],
                content_templates: vec![
                    "# Rust Programming\n\nRust is a systems programming language focused on safety and performance.".to_string(),
                ],
                path_prefixes: vec!["/programming/rust".to_string(), "/languages/rust".to_string()],
            },
            DocumentTopic {
                name: "Database Design".to_string(),
                tags: vec!["database".to_string(), "design".to_string(), "architecture".to_string()],
                content_templates: vec![
                    "# Database Design\n\nEffective database design is crucial for application performance.".to_string(),
                ],
                path_prefixes: vec!["/database".to_string(), "/architecture/data".to_string()],
            },
            DocumentTopic {
                name: "Distributed Systems".to_string(),
                tags: vec!["distributed".to_string(), "systems".to_string(), "scalability".to_string()],
                content_templates: vec![
                    "# Distributed Systems\n\nBuilding systems that scale across multiple machines.".to_string(),
                ],
                path_prefixes: vec!["/systems/distributed".to_string(), "/architecture/distributed".to_string()],
            },
            DocumentTopic {
                name: "Machine Learning".to_string(),
                tags: vec!["ml".to_string(), "ai".to_string(), "data-science".to_string()],
                content_templates: vec![
                    "# Machine Learning\n\nAlgorithms and techniques for learning from data.".to_string(),
                ],
                path_prefixes: vec!["/ml".to_string(), "/ai/machine-learning".to_string()],
            },
            DocumentTopic {
                name: "Web Development".to_string(),
                tags: vec!["web".to_string(), "frontend".to_string(), "backend".to_string()],
                content_templates: vec![
                    "# Web Development\n\nBuilding modern web applications and services.".to_string(),
                ],
                path_prefixes: vec!["/web".to_string(), "/frontend".to_string(), "/backend".to_string()],
            },
            DocumentTopic {
                name: "DevOps".to_string(),
                tags: vec!["devops".to_string(), "deployment".to_string(), "automation".to_string()],
                content_templates: vec![
                    "# DevOps\n\nAutomating deployment and infrastructure management.".to_string(),
                ],
                path_prefixes: vec!["/devops".to_string(), "/infrastructure".to_string()],
            },
            DocumentTopic {
                name: "Security".to_string(),
                tags: vec!["security".to_string(), "cryptography".to_string(), "privacy".to_string()],
                content_templates: vec![
                    "# Security\n\nProtecting systems and data from threats.".to_string(),
                ],
                path_prefixes: vec!["/security".to_string(), "/cybersecurity".to_string()],
            },
            DocumentTopic {
                name: "Project Management".to_string(),
                tags: vec!["management".to_string(), "agile".to_string(), "planning".to_string()],
                content_templates: vec![
                    "# Project Management\n\nEffectively managing software development projects.".to_string(),
                ],
                path_prefixes: vec!["/management".to_string(), "/project".to_string()],
            },
        ]
    }

    /// Get statistics about generated documents
    pub fn get_stats(&self) -> GenerationStats {
        GenerationStats {
            total_documents: self.config.count,
            total_size_bytes: self.config.count * self.config.base_size_bytes,
            avg_size_bytes: self.config.base_size_bytes,
            topics_covered: self.topics.len(),
            seed_used: self.config.seed,
        }
    }
}

#[derive(Debug)]
pub struct GenerationStats {
    pub total_documents: usize,
    pub total_size_bytes: usize,
    pub avg_size_bytes: usize,
    pub topics_covered: usize,
    pub seed_used: u64,
}

impl std::fmt::Display for GenerationStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Generated {} docs (~{:.1}MB total, ~{}KB avg) across {} topics (seed: {})",
            self.total_documents,
            self.total_size_bytes as f64 / 1_048_576.0,
            self.avg_size_bytes / 1024,
            self.topics_covered,
            self.seed_used
        )
    }
}
