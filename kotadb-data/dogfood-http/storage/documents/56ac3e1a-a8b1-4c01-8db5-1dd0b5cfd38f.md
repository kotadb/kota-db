---
tags:
- file
- kota-db
- ext_rs
---
// Embedding Transformer Module - Standardizes embeddings to OpenAI-compatible dimensions
// Provides transformation between different embedding model dimensions for compatibility

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

/// Standard dimension for OpenAI text-embedding-3-small compatibility
pub const OPENAI_STANDARD_DIMENSION: usize = 1536;

/// Methods for transforming embeddings between different dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransformationMethod {
    /// Pad smaller vectors with zeros at the end
    ZeroPadding,
    /// Repeat the vector cyclically to reach target dimension
    CyclicRepeat,
    /// Linear interpolation to upscale/downscale
    LinearInterpolation,
    /// Truncate larger vectors (for downscaling)
    Truncation,
    /// Normalize and pad (most common for upscaling)
    NormalizeAndPad,
}

/// Configuration for embedding dimension standardization
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompatibilityMode {
    /// Ensure embeddings are exactly 1536 dimensions (OpenAI standard)
    OpenAIStandard,
    /// Use the model's native dimensions
    Native,
    /// Automatically transform to target dimension
    Transform {
        target_dimension: usize,
        method: TransformationMethod,
    },
}

impl Default for CompatibilityMode {
    fn default() -> Self {
        Self::OpenAIStandard
    }
}

/// Transforms embeddings between different dimensions for cross-provider compatibility
#[derive(Debug)]
pub struct EmbeddingTransformer {
    source_dimension: usize,
    target_dimension: usize,
    method: TransformationMethod,
}

impl EmbeddingTransformer {
    /// Create a new transformer for the given dimensions and method
    pub fn new(
        source_dimension: usize,
        target_dimension: usize,
        method: TransformationMethod,
    ) -> Result<Self> {
        if source_dimension == 0 || target_dimension == 0 {
            return Err(anyhow!("Dimensions must be greater than zero"));
        }

        // Validate method compatibility
        if method == TransformationMethod::Truncation && target_dimension > source_dimension {
            return Err(anyhow!(
                "Truncation cannot increase dimension from {} to {}",
                source_dimension,
                target_dimension
            ));
        }

        Ok(Self {
            source_dimension,
            target_dimension,
            method,
        })
    }

    /// Create transformer to standardize to OpenAI dimensions (1536)
    pub fn to_openai_standard(source_dimension: usize) -> Result<Self> {
        let method = if source_dimension < OPENAI_STANDARD_DIMENSION {
            TransformationMethod::NormalizeAndPad
        } else if source_dimension > OPENAI_STANDARD_DIMENSION {
            TransformationMethod::LinearInterpolation
        } else {
            // Already correct dimension, no transformation needed
            return Self::identity(source_dimension);
        };

        Self::new(source_dimension, OPENAI_STANDARD_DIMENSION, method)
    }

    /// Create identity transformer (no transformation)
    pub fn identity(dimension: usize) -> Result<Self> {
        Self::new(dimension, dimension, TransformationMethod::ZeroPadding)
    }

    /// Check if transformation is needed
    pub fn is_identity(&self) -> bool {
        self.source_dimension == self.target_dimension
    }

    /// Transform a single embedding vector
    pub fn transform(&self, embedding: &[f32]) -> Result<Vec<f32>> {
        if embedding.len() != self.source_dimension {
            return Err(anyhow!(
                "Input embedding dimension {} doesn't match expected {}",
                embedding.len(),
                self.source_dimension
            ));
        }

        if self.is_identity() {
            return Ok(embedding.to_vec());
        }

        match self.method {
            TransformationMethod::ZeroPadding => self.zero_pad(embedding),
            TransformationMethod::CyclicRepeat => self.cyclic_repeat(embedding),
            TransformationMethod::LinearInterpolation => self.linear_interpolation(embedding),
            TransformationMethod::Truncation => self.truncate(embedding),
            TransformationMethod::NormalizeAndPad => self.normalize_and_pad(embedding),
        }
    }

    /// Transform a batch of embedding vectors
    pub fn transform_batch(&self, embeddings: &[Vec<f32>]) -> Result<Vec<Vec<f32>>> {
        embeddings.iter().map(|emb| self.transform(emb)).collect()
    }

    /// Zero-pad embedding to target dimension
    fn zero_pad(&self, embedding: &[f32]) -> Result<Vec<f32>> {
        if embedding.len() > self.target_dimension {
            return Err(anyhow!(
                "Cannot zero-pad: source dimension {} > target {}",
                embedding.len(),
                self.target_dimension
            ));
        }

        let mut result = embedding.to_vec();
        result.resize(self.target_dimension, 0.0);
        Ok(result)
    }

    /// Repeat embedding cyclically to reach target dimension
    fn cyclic_repeat(&self, embedding: &[f32]) -> Result<Vec<f32>> {
        let mut result = Vec::with_capacity(self.target_dimension);

        for i in 0..self.target_dimension {
            result.push(embedding[i % embedding.len()]);
        }

        Ok(result)
    }

    /// Linear interpolation to transform embedding dimension
    fn linear_interpolation(&self, embedding: &[f32]) -> Result<Vec<f32>> {
        if self.target_dimension == self.source_dimension {
            return Ok(embedding.to_vec());
        }

        let mut result = Vec::with_capacity(self.target_dimension);
        let scale = (embedding.len() - 1) as f32 / (self.target_dimension - 1) as f32;

        for i in 0..self.target_dimension {
            let src_index = i as f32 * scale;
            let lower_idx = src_index as usize;
            let upper_idx = (lower_idx + 1).min(embedding.len() - 1);

            if lower_idx == upper_idx {
                result.push(embedding[lower_idx]);
            } else {
                let fraction = src_index - lower_idx as f32;
                let interpolated =
                    embedding[lower_idx] * (1.0 - fraction) + embedding[upper_idx] * fraction;
                result.push(interpolated);
            }
        }

        Ok(result)
    }

    /// Truncate embedding to target dimension
    fn truncate(&self, embedding: &[f32]) -> Result<Vec<f32>> {
        if embedding.len() < self.target_dimension {
            return Err(anyhow!(
                "Cannot truncate: source dimension {} < target {}",
                embedding.len(),
                self.target_dimension
            ));
        }

        Ok(embedding[..self.target_dimension].to_vec())
    }

    /// Normalize embedding and pad with zeros
    fn normalize_and_pad(&self, embedding: &[f32]) -> Result<Vec<f32>> {
        // L2 normalize the original embedding
        let norm = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        let normalized: Vec<f32> = if norm > 0.0 {
            embedding.iter().map(|x| x / norm).collect()
        } else {
            embedding.to_vec()
        };

        // Pad to target dimension
        let mut result = normalized;
        result.resize(self.target_dimension, 0.0);

        // Re-normalize to maintain unit length
        let new_norm = result.iter().map(|x| x * x).sum::<f32>().sqrt();
        if new_norm > 0.0 {
            for val in result.iter_mut() {
                *val /= new_norm;
            }
        }

        Ok(result)
    }

    /// Get source dimension
    pub fn source_dimension(&self) -> usize {
        self.source_dimension
    }

    /// Get target dimension
    pub fn target_dimension(&self) -> usize {
        self.target_dimension
    }

    /// Get transformation method
    pub fn method(&self) -> TransformationMethod {
        self.method
    }
}

/// Utility functions for common transformations
impl EmbeddingTransformer {
    /// Transform 384-dimensional embedding (common local models) to OpenAI 1536
    pub fn minilm_to_openai() -> Result<Self> {
        Self::to_openai_standard(384)
    }

    /// Transform 768-dimensional embedding (Nomic, BERT-base) to OpenAI 1536
    pub fn bert_base_to_openai() -> Result<Self> {
        Self::to_openai_standard(768)
    }

    /// Transform 1024-dimensional embedding to OpenAI 1536
    pub fn large_model_to_openai() -> Result<Self> {
        Self::to_openai_standard(1024)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_transformer() -> Result<()> {
        let transformer = EmbeddingTransformer::identity(768)?;
        assert!(transformer.is_identity());

        let embedding = vec![1.0, 2.0, 3.0];
        // This should fail since dimensions don't match
        assert!(transformer.transform(&embedding).is_err());

        Ok(())
    }

    #[test]
    fn test_zero_padding() -> Result<()> {
        let transformer = EmbeddingTransformer::new(3, 5, TransformationMethod::ZeroPadding)?;
        let embedding = vec![1.0, 2.0, 3.0];

        let result = transformer.transform(&embedding)?;
        assert_eq!(result, vec![1.0, 2.0, 3.0, 0.0, 0.0]);

        Ok(())
    }

    #[test]
    fn test_cyclic_repeat() -> Result<()> {
        let transformer = EmbeddingTransformer::new(3, 7, TransformationMethod::CyclicRepeat)?;
        let embedding = vec![1.0, 2.0, 3.0];

        let result = transformer.transform(&embedding)?;
        assert_eq!(result, vec![1.0, 2.0, 3.0, 1.0, 2.0, 3.0, 1.0]);

        Ok(())
    }

    #[test]
    fn test_truncation() -> Result<()> {
        let transformer = EmbeddingTransformer::new(5, 3, TransformationMethod::Truncation)?;
        let embedding = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        let result = transformer.transform(&embedding)?;
        assert_eq!(result, vec![1.0, 2.0, 3.0]);

        Ok(())
    }

    #[test]
    fn test_linear_interpolation() -> Result<()> {
        let transformer =
            EmbeddingTransformer::new(3, 5, TransformationMethod::LinearInterpolation)?;
        let embedding = vec![1.0, 3.0, 5.0];

        let result = transformer.transform(&embedding)?;
        assert_eq!(result.len(), 5);

        // Check that endpoints are preserved
        assert_eq!(result[0], 1.0);
        assert_eq!(result[4], 5.0);

        Ok(())
    }

    #[test]
    fn test_normalize_and_pad() -> Result<()> {
        let transformer = EmbeddingTransformer::new(2, 4, TransformationMethod::NormalizeAndPad)?;
        let embedding = vec![3.0, 4.0]; // Length 5, should normalize to unit vector

        let result = transformer.transform(&embedding)?;
        assert_eq!(result.len(), 4);

        // Check that result is approximately unit length
        let norm: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);

        Ok(())
    }

    #[test]
    fn test_to_openai_standard() -> Result<()> {
        // Test upscaling from 384 to 1536
        let transformer = EmbeddingTransformer::to_openai_standard(384)?;
        assert_eq!(transformer.target_dimension(), OPENAI_STANDARD_DIMENSION);
        assert_eq!(transformer.method(), TransformationMethod::NormalizeAndPad);

        // Test downscaling from 2048 to 1536
        let transformer = EmbeddingTransformer::to_openai_standard(2048)?;
        assert_eq!(transformer.target_dimension(), OPENAI_STANDARD_DIMENSION);
        assert_eq!(
            transformer.method(),
            TransformationMethod::LinearInterpolation
        );

        // Test no transformation needed
        let transformer = EmbeddingTransformer::to_openai_standard(1536)?;
        assert!(transformer.is_identity());

        Ok(())
    }

    #[test]
    fn test_batch_transformation() -> Result<()> {
        let transformer = EmbeddingTransformer::new(2, 3, TransformationMethod::ZeroPadding)?;
        let embeddings = vec![vec![1.0, 2.0], vec![3.0, 4.0], vec![5.0, 6.0]];

        let results = transformer.transform_batch(&embeddings)?;
        assert_eq!(results.len(), 3);

        for result in &results {
            assert_eq!(result.len(), 3);
            assert_eq!(result[2], 0.0); // Zero padding
        }

        assert_eq!(results[0], vec![1.0, 2.0, 0.0]);
        assert_eq!(results[1], vec![3.0, 4.0, 0.0]);
        assert_eq!(results[2], vec![5.0, 6.0, 0.0]);

        Ok(())
    }

    #[test]
    fn test_common_transformations() -> Result<()> {
        // Test MiniLM to OpenAI transformation
        let transformer = EmbeddingTransformer::minilm_to_openai()?;
        assert_eq!(transformer.source_dimension(), 384);
        assert_eq!(transformer.target_dimension(), 1536);

        // Test BERT-base to OpenAI transformation
        let transformer = EmbeddingTransformer::bert_base_to_openai()?;
        assert_eq!(transformer.source_dimension(), 768);
        assert_eq!(transformer.target_dimension(), 1536);

        Ok(())
    }

    #[test]
    fn test_error_cases() {
        // Zero dimensions should fail
        assert!(EmbeddingTransformer::new(0, 100, TransformationMethod::ZeroPadding).is_err());
        assert!(EmbeddingTransformer::new(100, 0, TransformationMethod::ZeroPadding).is_err());

        // Truncation with wrong dimensions should fail
        assert!(EmbeddingTransformer::new(100, 200, TransformationMethod::Truncation).is_err());
    }
}
