# KotaDB Examples

This directory contains examples demonstrating how to use KotaDB as a standalone system.

## Running Examples

### Standalone Usage Example

```bash
# From the project root
cargo run --example standalone_usage
```

This example demonstrates:
- Validated types that prevent invalid states
- Builder patterns for ergonomic construction  
- Wrapper components for automatic best practices
- Risk reduction methodology in action

### Expected Output

```
🔧 KotaDB Standalone Usage Example
===================================

1. 🛡️  Validated Types - Invalid States Unrepresentable
   ------------------------------------------------
   ✓ Safe path: /documents/research.md
   ✓ Unique ID: f47ac10b-58cc-4372-a567-0e02b2c3d479
   ✓ Clean title: 'Machine Learning Research'
   ✓ Positive size: 1024 bytes
   ✓ Valid timestamp: 1641024000
   ✓ Ordered timestamps: 1641024000 -> 1641024000
   ✓ Safe tag: machine-learning

2. 🏗️  Builder Patterns - Ergonomic Construction
   ----------------------------------------------
   ✓ Document: 'Machine Learning Papers' (75 bytes, 8 words)
   ✓ Query: 'attention mechanisms' with 2 tags
   ✓ Storage config: /data/ml-research (cache: 268435456 bytes)

3. 🔧 Wrapper Components - Automatic Best Practices
   ------------------------------------------------
   When storage engine is implemented, wrappers provide:
   ✓ TracedStorage    - Unique trace IDs for every operation
   ✓ ValidatedStorage - Input/output validation
   ✓ RetryableStorage - Exponential backoff on failures
   ✓ CachedStorage    - LRU caching with hit/miss metrics
   ✓ SafeTransaction  - RAII rollback on scope exit
   ✓ MeteredIndex     - Automatic performance metrics

4. 📊 Risk Reduction Summary
   -------------------------
   Stage 1: TDD                     -5.0 points
   Stage 2: Contracts               -5.0 points
   Stage 3: Pure Functions          -3.5 points
   Stage 4: Observability           -4.5 points
   Stage 5: Adversarial Testing     -0.5 points
   Stage 6: Component Library        -1.0 points
   ----------------------------------------
   Total Risk Reduction:            -19.5 points
   Success Rate: ~99% (vs ~78% baseline)

✅ Stage 6 implementation verified!
   All components working correctly
   Ready for storage engine implementation
```

## Example Files

- `standalone_usage.rs` - Comprehensive demonstration of Stage 6 components

## Adding Examples

When adding new examples:

1. Follow the 6-stage risk reduction methodology
2. Use Stage 6 components for all constructions
3. Include error handling and logging
4. Document the risk reduction benefits
5. Show both correct usage and prevented errors

## Library Integration

These examples also serve as templates for integrating KotaDB into other Rust projects:

```toml
[dependencies]
kotadb = { path = "../path/to/kotadb" }
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
```