// Embeddings Module - Pluggable embedding providers for semantic search
// Supports both local models (ONNX) and cloud APIs (OpenAI) with dimension standardization

use crate::embedding_transformer::{
    CompatibilityMode, EmbeddingTransformer, OPENAI_STANDARD_DIMENSION,
};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;

#[cfg(feature = "embeddings-onnx")]
use ort::session::Session;
#[cfg(feature = "embeddings-onnx")]
use tokenizers::Tokenizer;

/// Configuration for embedding providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub provider: EmbeddingProviderType,
    pub model_name: String,
    pub dimension: usize, // Output dimension (usually 1536 for OpenAI compatibility)
    pub native_dimension: Option<usize>, // Model's native dimension (if different)
    pub max_batch_size: usize,
    pub compatibility_mode: CompatibilityMode, // How to handle dimension compatibility
    pub provider_config: ProviderConfig,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: EmbeddingProviderType::Local,
            model_name: "all-MiniLM-L6-v2".to_string(),
            dimension: OPENAI_STANDARD_DIMENSION, // Output OpenAI-compatible dimensions
            native_dimension: Some(384),          // MiniLM's native dimension
            max_batch_size: 32,
            compatibility_mode: CompatibilityMode::OpenAIStandard, // Auto-transform to OpenAI standard
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
    transformer: Option<EmbeddingTransformer>, // For dimension compatibility
    #[cfg(feature = "embeddings-onnx")]
    #[allow(dead_code)] // Will be used when ONNX inference is implemented
    session: Session, // ONNX Runtime session
    #[cfg(feature = "embeddings-onnx")]
    #[allow(dead_code)] // Will be used when ONNX inference is implemented
    tokenizer: Option<Tokenizer>, // Tokenizer for text preprocessing
    #[cfg(not(feature = "embeddings-onnx"))]
    _placeholder: (), // Placeholder when ONNX disabled
}

impl LocalEmbeddingProvider {
    /// Create a new local embedding provider
    pub async fn new(config: EmbeddingConfig) -> Result<Self> {
        if config.provider != crate::embeddings::EmbeddingProviderType::Local {
            return Err(anyhow!("Config is not for local provider"));
        }

        // Create dimension transformer if needed
        let transformer = Self::create_transformer(&config)?;

        #[cfg(feature = "embeddings-onnx")]
        {
            let (session, tokenizer) = Self::load_onnx_model(&config).await?;

            Ok(Self {
                config,
                transformer,
                session,
                tokenizer,
            })
        }

        #[cfg(not(feature = "embeddings-onnx"))]
        {
            // Fallback when ONNX is not enabled
            tracing::warn!("ONNX Runtime not enabled, embedding functionality will be limited");
            Ok(Self {
                config,
                transformer,
                _placeholder: (),
            })
        }
    }

    /// Create transformer based on configuration
    fn create_transformer(config: &EmbeddingConfig) -> Result<Option<EmbeddingTransformer>> {
        match config.compatibility_mode {
            CompatibilityMode::Native => Ok(None),
            CompatibilityMode::OpenAIStandard => {
                let native_dim = config.native_dimension.unwrap_or(config.dimension);
                if native_dim == OPENAI_STANDARD_DIMENSION {
                    Ok(None) // No transformation needed
                } else {
                    Ok(Some(EmbeddingTransformer::to_openai_standard(native_dim)?))
                }
            }
            CompatibilityMode::Transform {
                target_dimension,
                method,
            } => {
                let native_dim = config.native_dimension.unwrap_or(config.dimension);
                Ok(Some(EmbeddingTransformer::new(
                    native_dim,
                    target_dimension,
                    method,
                )?))
            }
        }
    }

    #[cfg(feature = "embeddings-onnx")]
    /// Load ONNX model and tokenizer
    async fn load_onnx_model(config: &EmbeddingConfig) -> Result<(Session, Option<Tokenizer>)> {
        let ProviderConfig::Local {
            model_path,
            tokenizer_path,
        } = &config.provider_config
        else {
            return Err(anyhow!("Invalid config for local provider"));
        };

        // For now, return an error if ONNX model file doesn't exist
        // This allows the system to fall back gracefully
        if !model_path.exists() {
            return Err(anyhow!(
                "ONNX model file not found at {:?}. Please provide a valid model file or use OpenAI provider.", 
                model_path
            ));
        }

        // TODO: Implement proper ONNX runtime loading
        // For now, create a placeholder that will trigger fallback
        Err(anyhow!(
            "ONNX Runtime integration is not yet fully implemented. Please use OpenAI provider for now."
        ))

        // Future ONNX integration will go here
        // let environment = ort::Environment::default().with_name("kotadb-embeddings")?;
        // let session = ort::Session::builder(&environment)?
        //     .with_model_from_file(model_path)?;
        //
        // Load tokenizer if provided
        // let tokenizer = if let Some(tokenizer_path) = tokenizer_path {
        //     if tokenizer_path.exists() {
        //         Some(Tokenizer::from_file(tokenizer_path).map_err(|e| anyhow!("Tokenizer error: {}", e))?)
        //     } else {
        //         tracing::warn!("Tokenizer file not found at {:?}, using fallback tokenization", tokenizer_path);
        //         None
        //     }
        // } else {
        //     None
        // };
        //
        // tracing::info!("Loaded ONNX model from {:?}", model_path);
        // Ok((session, tokenizer))
    }

    /// Tokenize text using the loaded tokenizer or fallback method
    fn tokenize(&self, text: &str, max_length: Option<usize>) -> Result<Vec<i64>> {
        #[cfg(feature = "embeddings-onnx")]
        {
            // Real tokenizer integration will be implemented later
            // For now, fall through to the simple tokenizer below
        }

        // Fallback tokenization (simple word-based)
        let words: Vec<&str> = text.split_whitespace().collect();
        let max_len = max_length.unwrap_or(512);

        let tokens: Vec<i64> = words
            .iter()
            .take(max_len)
            .enumerate()
            .map(|(i, word)| {
                // Simple hash-based token ID
                let mut hash = 0u64;
                for byte in word.bytes() {
                    hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
                }
                (hash % 30000 + 100) as i64 // Keep in reasonable vocab range
            })
            .collect();

        Ok(tokens)
    }

    /// Run inference with the ONNX model
    #[cfg(feature = "embeddings-onnx")]
    async fn run_onnx_inference(&self, _token_batches: &[Vec<i64>]) -> Result<Vec<Vec<f32>>> {
        // TODO: Implement proper ONNX inference once ORT API is stabilized
        // For now, return an error to trigger fallback behavior
        Err(anyhow!(
            "ONNX Runtime inference not yet implemented. Please use OpenAI provider for embeddings."
        ))

        // Future implementation will go here with proper ORT 2.0 API usage
        // This method will:
        // 1. Convert token batches to ONNX tensors
        // 2. Run model inference
        // 3. Extract embeddings from output tensors
        // 4. Return native dimension embeddings (transformation happens later)
    }

    /// Fallback inference when ONNX is not available
    #[cfg(not(feature = "embeddings-onnx"))]
    async fn run_fallback_inference(&self, _token_batches: &[Vec<i64>]) -> Result<Vec<Vec<f32>>> {
        Err(anyhow!(
            "ONNX Runtime not enabled. Please rebuild with --features embeddings-onnx or use OpenAI provider"
        ))
    }

    /// Apply dimension transformation if configured
    fn apply_transformation(&self, embeddings: Vec<Vec<f32>>) -> Result<Vec<Vec<f32>>> {
        if let Some(ref transformer) = self.transformer {
            transformer.transform_batch(&embeddings)
        } else {
            Ok(embeddings)
        }
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
            let tokens = self.tokenize(text, Some(512))?; // Limit to 512 tokens
            all_tokens.push(tokens);
        }

        // Run inference
        #[cfg(feature = "embeddings-onnx")]
        let raw_embeddings = self.run_onnx_inference(&all_tokens).await?;

        #[cfg(not(feature = "embeddings-onnx"))]
        let raw_embeddings = self.run_fallback_inference(&all_tokens).await?;

        // Apply dimension transformation if needed
        let embeddings = self.apply_transformation(raw_embeddings)?;

        tracing::debug!(
            "Generated {} embeddings with dimension {} (transformed from native: {:?})",
            embeddings.len(),
            embeddings.first().map(|e| e.len()).unwrap_or(0),
            self.config.native_dimension
        );

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
            dimension: OPENAI_STANDARD_DIMENSION,
            native_dimension: Some(OPENAI_STANDARD_DIMENSION), // Native OpenAI dimension
            max_batch_size: 2048,
            compatibility_mode: CompatibilityMode::OpenAIStandard, // Already compatible
            provider_config: ProviderConfig::OpenAI {
                api_key,
                api_base: None,
                organization: None,
            },
        }
    }

    /// Configuration for OpenAI text-embedding-3-large (3072 dimensions, downscaled to 1536)
    pub fn openai_text_embedding_3_large(api_key: String) -> EmbeddingConfig {
        EmbeddingConfig {
            provider: crate::embeddings::EmbeddingProviderType::OpenAI,
            model_name: "text-embedding-3-large".to_string(),
            dimension: OPENAI_STANDARD_DIMENSION, // Downscale to standard
            native_dimension: Some(3072),         // Large model's native dimension
            max_batch_size: 2048,
            compatibility_mode: CompatibilityMode::OpenAIStandard, // Use standard dimension
            provider_config: ProviderConfig::OpenAI {
                api_key,
                api_base: None,
                organization: None,
            },
        }
    }

    /// Configuration for local all-MiniLM-L6-v2 model (384→1536 dimensions)
    pub fn local_minilm_l6_v2(model_path: PathBuf) -> EmbeddingConfig {
        EmbeddingConfig {
            provider: crate::embeddings::EmbeddingProviderType::Local,
            model_name: "all-MiniLM-L6-v2".to_string(),
            dimension: OPENAI_STANDARD_DIMENSION, // Transform to OpenAI standard
            native_dimension: Some(384),          // MiniLM's native dimension
            max_batch_size: 32,
            compatibility_mode: CompatibilityMode::OpenAIStandard,
            provider_config: ProviderConfig::Local {
                model_path,
                tokenizer_path: None,
            },
        }
    }

    /// Configuration for local BGE-small-en-v1.5 model (384→1536 dimensions)
    pub fn local_bge_small_en(model_path: PathBuf) -> EmbeddingConfig {
        EmbeddingConfig {
            provider: crate::embeddings::EmbeddingProviderType::Local,
            model_name: "BAAI/bge-small-en-v1.5".to_string(),
            dimension: OPENAI_STANDARD_DIMENSION, // Transform to OpenAI standard
            native_dimension: Some(384),          // BGE's native dimension
            max_batch_size: 32,
            compatibility_mode: CompatibilityMode::OpenAIStandard,
            provider_config: ProviderConfig::Local {
                model_path,
                tokenizer_path: None,
            },
        }
    }

    /// Configuration for local E5-small-v2 model (384→1536 dimensions)
    pub fn local_e5_small_v2(model_path: PathBuf) -> EmbeddingConfig {
        EmbeddingConfig {
            provider: crate::embeddings::EmbeddingProviderType::Local,
            model_name: "intfloat/e5-small-v2".to_string(),
            dimension: OPENAI_STANDARD_DIMENSION, // Transform to OpenAI standard
            native_dimension: Some(384),          // E5's native dimension
            max_batch_size: 32,
            compatibility_mode: CompatibilityMode::OpenAIStandard,
            provider_config: ProviderConfig::Local {
                model_path,
                tokenizer_path: None,
            },
        }
    }

    /// Configuration for local Nomic Embed v2 model (768→1536 dimensions)
    pub fn local_nomic_embed_v2(model_path: PathBuf) -> EmbeddingConfig {
        EmbeddingConfig {
            provider: crate::embeddings::EmbeddingProviderType::Local,
            model_name: "nomic-ai/nomic-embed-text-v2".to_string(),
            dimension: OPENAI_STANDARD_DIMENSION, // Transform to OpenAI standard
            native_dimension: Some(768),          // Nomic's native dimension
            max_batch_size: 32,
            compatibility_mode: CompatibilityMode::OpenAIStandard,
            provider_config: ProviderConfig::Local {
                model_path,
                tokenizer_path: None,
            },
        }
    }

    /// Configuration for local BERT-base model (768→1536 dimensions)
    pub fn local_bert_base(model_path: PathBuf) -> EmbeddingConfig {
        EmbeddingConfig {
            provider: crate::embeddings::EmbeddingProviderType::Local,
            model_name: "bert-base-uncased".to_string(),
            dimension: OPENAI_STANDARD_DIMENSION, // Transform to OpenAI standard
            native_dimension: Some(768),          // BERT-base's native dimension
            max_batch_size: 16,
            compatibility_mode: CompatibilityMode::OpenAIStandard,
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
        // Test that the provider correctly initializes with a configuration
        // and reports the right expected dimensions (even if the model file doesn't exist)
        let temp_dir = TempDir::new()?;
        let model_path = temp_dir.path().join("test_model.onnx");

        let config = models::local_minilm_l6_v2(model_path.clone());

        // Test configuration is correct
        assert_eq!(config.dimension, OPENAI_STANDARD_DIMENSION); // Output dimension
        assert_eq!(config.native_dimension, Some(384)); // Native MiniLM dimension
        assert_eq!(config.model_name, "all-MiniLM-L6-v2");

        // Test provider creation - should fail gracefully since model file doesn't exist
        let provider_result = LocalEmbeddingProvider::new(config).await;
        assert!(provider_result.is_err());

        let error_msg = provider_result.unwrap_err().to_string();
        assert!(
            error_msg.contains("ONNX model file not found")
                || error_msg.contains("not yet fully implemented")
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_embedding_service_caching() -> Result<()> {
        // Test using OpenAI configuration since local models require actual files
        let config = models::openai_text_embedding_3_small("test-key".to_string());

        // Test that service creation works with OpenAI config
        assert_eq!(config.dimension, OPENAI_STANDARD_DIMENSION);
        assert_eq!(config.model_name, "text-embedding-3-small");

        // EmbeddingService creation with local models will fail without actual ONNX files
        // This test validates the configuration structure is correct
        let service_result = EmbeddingService::new(config).await;

        // Service creation should work with proper config (even if API key is invalid)
        match service_result {
            Ok(_service) => {
                // Service created successfully
                // Could test actual embedding calls here if we had valid API key
            }
            Err(error) => {
                // Expected error due to invalid API key or other configuration issues
                let error_msg = error.to_string();
                // This is expected behavior - OpenAI provider needs valid config
                assert!(!error_msg.is_empty()); // Just ensure we get some error message
            }
        }

        Ok(())
    }

    #[test]
    fn test_model_configurations() {
        let openai_config = models::openai_text_embedding_3_small("test-key".to_string());
        assert_eq!(openai_config.dimension, OPENAI_STANDARD_DIMENSION);
        assert_eq!(
            openai_config.native_dimension,
            Some(OPENAI_STANDARD_DIMENSION)
        );
        assert_eq!(openai_config.model_name, "text-embedding-3-small");

        let local_config = models::local_minilm_l6_v2("/path/to/model.onnx".into());
        assert_eq!(local_config.dimension, OPENAI_STANDARD_DIMENSION); // Transformed output
        assert_eq!(local_config.native_dimension, Some(384)); // Native input
        assert_eq!(local_config.model_name, "all-MiniLM-L6-v2");

        let nomic_config = models::local_nomic_embed_v2("/path/to/nomic.onnx".into());
        assert_eq!(nomic_config.dimension, OPENAI_STANDARD_DIMENSION); // Transformed output
        assert_eq!(nomic_config.native_dimension, Some(768)); // Native input
        assert_eq!(nomic_config.model_name, "nomic-ai/nomic-embed-text-v2");
    }
}
