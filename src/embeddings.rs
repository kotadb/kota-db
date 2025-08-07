// Embeddings Module - Pluggable embedding providers for semantic search
// Supports both local models (ONNX) and cloud APIs (OpenAI)

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;

/// Configuration for embedding providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub provider: EmbeddingProviderType,
    pub model_name: String,
    pub dimension: usize,
    pub max_batch_size: usize,
    pub provider_config: ProviderConfig,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: EmbeddingProviderType::Local,
            model_name: "all-MiniLM-L6-v2".to_string(),
            dimension: 384,
            max_batch_size: 32,
            provider_config: ProviderConfig::Local {
                model_path: "./models/all-MiniLM-L6-v2.onnx".into(),
                tokenizer_path: Some("./models/tokenizer.json".into()),
            },
        }
    }
}

/// Available embedding provider types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmbeddingProviderType {
    Local,
    OpenAI,
    Custom,
}

/// Provider-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderConfig {
    Local {
        model_path: PathBuf,
        tokenizer_path: Option<PathBuf>,
    },
    OpenAI {
        api_key: String,
        api_base: Option<String>, // For OpenAI-compatible APIs
        organization: Option<String>,
    },
    Custom {
        endpoint: String,
        api_key: Option<String>,
        headers: HashMap<String, String>,
    },
}

/// Result of embedding generation
#[derive(Debug, Clone)]
pub struct EmbeddingResult {
    pub embeddings: Vec<Vec<f32>>,
    pub model_used: String,
    pub tokens_used: Option<usize>,
}

/// Trait for embedding providers
#[async_trait::async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embeddings for a batch of texts
    async fn embed_texts(&self, texts: &[String]) -> Result<EmbeddingResult>;

    /// Generate embedding for a single text
    async fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        let result = self.embed_texts(&[text.to_string()]).await?;
        result
            .embeddings
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No embedding returned"))
    }

    /// Get the dimension of embeddings produced by this provider
    fn dimension(&self) -> usize;

    /// Get the model name
    fn model_name(&self) -> &str;

    /// Get the maximum batch size
    fn max_batch_size(&self) -> usize;
}

/// Local embedding provider using ONNX models
#[derive(Debug)]
pub struct LocalEmbeddingProvider {
    config: EmbeddingConfig,
    // We'll use ONNX Runtime for local inference
    // session: ort::Session, // Will be added when we integrate ort
    _model_loaded: bool, // Placeholder for now
}

impl LocalEmbeddingProvider {
    /// Create a new local embedding provider
    pub async fn new(config: EmbeddingConfig) -> Result<Self> {
        if config.provider != crate::embeddings::EmbeddingProviderType::Local {
            return Err(anyhow!("Config is not for local provider"));
        }

        // TODO: Load ONNX model here when we add ort dependency
        // For now, we'll simulate the functionality

        Ok(Self {
            config,
            _model_loaded: true,
        })
    }

    /// Tokenize text for the model
    fn tokenize(&self, text: &str) -> Result<Vec<i64>> {
        // TODO: Implement proper tokenization using tokenizers crate
        // For now, simulate basic tokenization
        let tokens: Vec<i64> = text
            .split_whitespace()
            .enumerate()
            .map(|(i, _)| i as i64)
            .collect();
        Ok(tokens)
    }

    /// Run inference with the ONNX model
    async fn run_inference(&self, tokens: &[Vec<i64>]) -> Result<Vec<Vec<f32>>> {
        // TODO: Implement actual ONNX inference
        // For now, return dummy embeddings of the correct dimension
        let batch_size = tokens.len();
        let mut embeddings = Vec::with_capacity(batch_size);

        for _ in 0..batch_size {
            let mut embedding = vec![0.0f32; self.config.dimension];
            // Generate a simple hash-based embedding for testing
            for (i, val) in embedding.iter_mut().enumerate() {
                *val = ((i * 137) % 1000) as f32 / 1000.0 - 0.5;
            }
            embeddings.push(embedding);
        }

        Ok(embeddings)
    }
}

#[async_trait::async_trait]
impl EmbeddingProvider for LocalEmbeddingProvider {
    async fn embed_texts(&self, texts: &[String]) -> Result<EmbeddingResult> {
        if texts.is_empty() {
            return Ok(EmbeddingResult {
                embeddings: Vec::new(),
                model_used: self.config.model_name.clone(),
                tokens_used: Some(0),
            });
        }

        // Tokenize all texts
        let mut all_tokens = Vec::new();
        for text in texts {
            let tokens = self.tokenize(text)?;
            all_tokens.push(tokens);
        }

        // Run inference
        let embeddings = self.run_inference(&all_tokens).await?;

        Ok(EmbeddingResult {
            embeddings,
            model_used: self.config.model_name.clone(),
            tokens_used: Some(all_tokens.iter().map(|t| t.len()).sum()),
        })
    }

    fn dimension(&self) -> usize {
        self.config.dimension
    }

    fn model_name(&self) -> &str {
        &self.config.model_name
    }

    fn max_batch_size(&self) -> usize {
        self.config.max_batch_size
    }
}

/// OpenAI embedding provider
#[derive(Debug)]
pub struct OpenAIEmbeddingProvider {
    config: EmbeddingConfig,
    client: reqwest::Client,
    api_key: String,
    api_base: String,
}

impl OpenAIEmbeddingProvider {
    /// Create a new OpenAI embedding provider
    pub fn new(config: EmbeddingConfig) -> Result<Self> {
        let provider_config = match &config.provider_config {
            ProviderConfig::OpenAI {
                api_key, api_base, ..
            } => (api_key.clone(), api_base.clone()),
            _ => return Err(anyhow!("Config is not for OpenAI provider")),
        };

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            config,
            client,
            api_key: provider_config.0,
            api_base: provider_config
                .1
                .unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
        })
    }
}

#[async_trait::async_trait]
impl EmbeddingProvider for OpenAIEmbeddingProvider {
    async fn embed_texts(&self, texts: &[String]) -> Result<EmbeddingResult> {
        if texts.is_empty() {
            return Ok(EmbeddingResult {
                embeddings: Vec::new(),
                model_used: self.config.model_name.clone(),
                tokens_used: Some(0),
            });
        }

        #[derive(Serialize)]
        struct EmbeddingRequest {
            input: Vec<String>,
            model: String,
            encoding_format: String,
        }

        #[derive(Deserialize)]
        struct EmbeddingResponse {
            data: Vec<EmbeddingData>,
            usage: Usage,
        }

        #[derive(Deserialize)]
        struct EmbeddingData {
            embedding: Vec<f32>,
            index: usize,
        }

        #[derive(Deserialize)]
        struct Usage {
            total_tokens: usize,
        }

        let request = EmbeddingRequest {
            input: texts.to_vec(),
            model: self.config.model_name.clone(),
            encoding_format: "float".to_string(),
        };

        let response = self
            .client
            .post(format!("{}/embeddings", self.api_base))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("OpenAI API error: {}", error_text));
        }

        let embedding_response: EmbeddingResponse = response.json().await?;

        // Sort embeddings by index to maintain order
        let mut sorted_data = embedding_response.data;
        sorted_data.sort_by_key(|d| d.index);

        let embeddings = sorted_data.into_iter().map(|d| d.embedding).collect();

        Ok(EmbeddingResult {
            embeddings,
            model_used: self.config.model_name.clone(),
            tokens_used: Some(embedding_response.usage.total_tokens),
        })
    }

    fn dimension(&self) -> usize {
        self.config.dimension
    }

    fn model_name(&self) -> &str {
        &self.config.model_name
    }

    fn max_batch_size(&self) -> usize {
        self.config.max_batch_size
    }
}

/// Embedding service that manages providers and caching
pub struct EmbeddingService {
    provider: Box<dyn EmbeddingProvider>,
    cache: RwLock<HashMap<String, Vec<f32>>>,
    #[allow(dead_code)] // Used for future configuration access
    config: EmbeddingConfig,
}

impl EmbeddingService {
    /// Create a new embedding service with the given configuration
    pub async fn new(config: EmbeddingConfig) -> Result<Self> {
        let provider: Box<dyn EmbeddingProvider> = match config.provider {
            crate::embeddings::EmbeddingProviderType::Local => {
                Box::new(LocalEmbeddingProvider::new(config.clone()).await?)
            }
            crate::embeddings::EmbeddingProviderType::OpenAI => {
                Box::new(OpenAIEmbeddingProvider::new(config.clone())?)
            }
            crate::embeddings::EmbeddingProviderType::Custom => {
                return Err(anyhow!("Custom providers not yet implemented"));
            }
        };

        Ok(Self {
            provider,
            cache: RwLock::new(HashMap::new()),
            config,
        })
    }

    /// Generate embedding for a single text with caching
    pub async fn embed_text(&self, text: &str) -> Result<Vec<f32>> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(embedding) = cache.get(text) {
                return Ok(embedding.clone());
            }
        }

        // Generate new embedding
        let embedding = self.provider.embed_text(text).await?;

        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.insert(text.to_string(), embedding.clone());
        }

        Ok(embedding)
    }

    /// Generate embeddings for multiple texts
    pub async fn embed_texts(&self, texts: &[String]) -> Result<EmbeddingResult> {
        self.provider.embed_texts(texts).await
    }

    /// Get the dimension of embeddings
    pub fn dimension(&self) -> usize {
        self.provider.dimension()
    }

    /// Get the model name
    pub fn model_name(&self) -> &str {
        self.provider.model_name()
    }

    /// Clear the embedding cache
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get cache statistics
    pub async fn cache_stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        (cache.len(), cache.capacity())
    }
}

/// Utility functions for common embedding models
pub mod models {
    use super::*;

    /// Configuration for OpenAI text-embedding-3-small (1536 dimensions)
    pub fn openai_text_embedding_3_small(api_key: String) -> EmbeddingConfig {
        EmbeddingConfig {
            provider: crate::embeddings::EmbeddingProviderType::OpenAI,
            model_name: "text-embedding-3-small".to_string(),
            dimension: 1536,
            max_batch_size: 2048,
            provider_config: ProviderConfig::OpenAI {
                api_key,
                api_base: None,
                organization: None,
            },
        }
    }

    /// Configuration for OpenAI text-embedding-3-large (3072 dimensions)
    pub fn openai_text_embedding_3_large(api_key: String) -> EmbeddingConfig {
        EmbeddingConfig {
            provider: crate::embeddings::EmbeddingProviderType::OpenAI,
            model_name: "text-embedding-3-large".to_string(),
            dimension: 3072,
            max_batch_size: 2048,
            provider_config: ProviderConfig::OpenAI {
                api_key,
                api_base: None,
                organization: None,
            },
        }
    }

    /// Configuration for local all-MiniLM-L6-v2 model (384 dimensions)
    pub fn local_minilm_l6_v2(model_path: PathBuf) -> EmbeddingConfig {
        EmbeddingConfig {
            provider: crate::embeddings::EmbeddingProviderType::Local,
            model_name: "all-MiniLM-L6-v2".to_string(),
            dimension: 384,
            max_batch_size: 32,
            provider_config: ProviderConfig::Local {
                model_path,
                tokenizer_path: None,
            },
        }
    }

    /// Configuration for local BGE-small-en-v1.5 model (384 dimensions)
    pub fn local_bge_small_en(model_path: PathBuf) -> EmbeddingConfig {
        EmbeddingConfig {
            provider: crate::embeddings::EmbeddingProviderType::Local,
            model_name: "BAAI/bge-small-en-v1.5".to_string(),
            dimension: 384,
            max_batch_size: 32,
            provider_config: ProviderConfig::Local {
                model_path,
                tokenizer_path: None,
            },
        }
    }

    /// Configuration for local E5-small-v2 model (384 dimensions)
    pub fn local_e5_small_v2(model_path: PathBuf) -> EmbeddingConfig {
        EmbeddingConfig {
            provider: crate::embeddings::EmbeddingProviderType::Local,
            model_name: "intfloat/e5-small-v2".to_string(),
            dimension: 384,
            max_batch_size: 32,
            provider_config: ProviderConfig::Local {
                model_path,
                tokenizer_path: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_local_embedding_provider() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let model_path = temp_dir.path().join("test_model.onnx");

        let config = models::local_minilm_l6_v2(model_path);
        let provider = LocalEmbeddingProvider::new(config).await?;

        let texts = vec!["Hello world".to_string(), "Test embedding".to_string()];
        let result = provider.embed_texts(&texts).await?;

        assert_eq!(result.embeddings.len(), 2);
        assert_eq!(result.embeddings[0].len(), 384);
        assert_eq!(result.model_used, "all-MiniLM-L6-v2");

        Ok(())
    }

    #[tokio::test]
    async fn test_embedding_service_caching() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let model_path = temp_dir.path().join("test_model.onnx");

        let config = models::local_minilm_l6_v2(model_path);
        let service = EmbeddingService::new(config).await?;

        let text = "Hello world";

        // First call - should cache
        let embedding1 = service.embed_text(text).await?;
        assert_eq!(embedding1.len(), 384);

        // Second call - should use cache
        let embedding2 = service.embed_text(text).await?;
        assert_eq!(embedding1, embedding2);

        let (cache_size, _) = service.cache_stats().await;
        assert_eq!(cache_size, 1);

        Ok(())
    }

    #[test]
    fn test_model_configurations() {
        let openai_config = models::openai_text_embedding_3_small("test-key".to_string());
        assert_eq!(openai_config.dimension, 1536);
        assert_eq!(openai_config.model_name, "text-embedding-3-small");

        let local_config = models::local_minilm_l6_v2("/path/to/model.onnx".into());
        assert_eq!(local_config.dimension, 384);
        assert_eq!(local_config.model_name, "all-MiniLM-L6-v2");
    }
}
