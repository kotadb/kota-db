# Migration Guide - KotaDB v0.5.1

## Critical Performance Fix and Breaking Changes

KotaDB v0.5.1 includes a critical performance fix that resolves a 675x performance regression in search operations. This fix requires behavioral changes that may affect existing workflows.

## What Changed

### 🚨 Breaking Change: Default Search Context

**Before (v0.5.0 and earlier):**
- Default context: `medium` 
- All non-wildcard searches used expensive LLM processing
- Search operations took 79+ seconds

**After (v0.5.1):**
- Default context: `minimal`
- Fast trigram search by default
- Search operations take ~0.5 seconds (151x improvement)

### Search Context Options

| Context | Behavior | Use When | Performance |
|---------|----------|----------|-------------|
| `none` | Fast search, minimal output | Scripting, automation | ~100ms |
| `minimal` | **New default** - Fast search, clean output | Daily usage, AI assistants | ~500ms |
| `medium` | LLM-enhanced search with analysis | In-depth code exploration | ~2-5s |
| `full` | Maximum LLM analysis and context | Complex architectural analysis | ~5-10s |

## Migration Steps

### For Individual Users

#### No Action Required (Recommended)
The new default provides 151x better performance while maintaining full search functionality. Most users will benefit immediately.

#### To Restore Previous Behavior
If you specifically need LLM-enhanced search by default:

```bash
# Create an alias with medium context
alias kotadb-enhanced='kotadb search -c medium'

# Or set an environment variable (if your shell supports it)
export KOTADB_DEFAULT_CONTEXT=medium
```

### For Scripts and Automation

#### Update Scripts Using Search
```bash
# Old: Relied on medium context default
kotadb -d ./data search "async function"

# New: Explicitly specify context if needed
kotadb -d ./data search -c medium "async function"  # LLM analysis
kotadb -d ./data search -c minimal "async function" # Fast search (new default)
```

#### CI/CD Pipeline Updates
```yaml
# Update your CI scripts to be explicit about context
- name: Search codebase
  run: |
    # For fast CI searches (recommended)
    kotadb -d ./analysis search -c minimal "TODO"
    
    # For detailed analysis (if needed)
    kotadb -d ./analysis search -c medium "complex logic"
```

### For AI Assistant Integration

#### Claude Code and Similar Tools
AI assistants will automatically benefit from the 151x performance improvement. No changes needed.

#### Custom AI Integrations
Update API calls or command wrappers:

```javascript
// JavaScript example
const searchOptions = {
  context: 'minimal', // Explicit for clarity
  // context: 'medium', // Use for enhanced analysis when needed
};
```

```python
# Python example
def search_codebase(query, enhanced=False):
    context = 'medium' if enhanced else 'minimal'
    return subprocess.run([
        'kotadb', 'search', '-c', context, query
    ], capture_output=True, text=True)
```

## Performance Impact

### Before v0.5.1 (Broken State)
- All searches: 79+ seconds
- Unusable for real-time AI assistance
- Blocked by expensive LLM processing

### After v0.5.1 (Fixed)
- Default searches: ~0.5 seconds (151x improvement)
- Enhanced searches: Still available with `-c medium/full`
- Optimal for AI assistant workflows

## Verification Steps

### Test Your Migration

1. **Verify Performance Improvement:**
```bash
# This should complete in <1 second
time kotadb -d ./data search "your typical query"
```

2. **Test Enhanced Search (if needed):**
```bash
# This should provide detailed LLM analysis
kotadb -d ./data search -c medium "complex architectural question"
```

3. **Check Script Compatibility:**
```bash
# Run your existing scripts - they should be much faster
./your-search-script.sh
```

## Troubleshooting

### "My searches are too fast/simple now"
```bash
# Use medium or full context for enhanced analysis
kotadb search -c medium "your query"
kotadb search -c full "your query"
```

### "I want the old default back"
```bash
# Create a shell alias
alias search='kotadb search -c medium'
```

### "My CI pipeline is faster but missing details"
This is expected and beneficial. Use `-c medium` only for specific analysis steps that require LLM enhancement.

### "Search returns no results for edge cases"
v0.5.1 includes a sophisticated fallback mechanism that progressively relaxes search thresholds when strict precision filtering eliminates all results. If you encounter cases where this fails:

```bash
# Try different context levels
kotadb search -c minimal "query"  # Fastest, most precise
kotadb search -c medium "query"   # Enhanced analysis
```

## Support

### Getting Help
- Check the updated CLI help: `kotadb search --help`
- Review performance in logs with `RUST_LOG=debug`
- Report issues: [GitHub Issues](https://github.com/jayminwest/kota-db/issues)

### Rollback (Not Recommended)
If you must use the previous version:
```bash
# This will restore the broken 79-second search behavior
git checkout v0.5.0
cargo install --path .
```

**Note:** Rollback is not recommended as it restores the 675x performance regression.

## Benefits Summary

✅ **151x faster searches** (79s → 0.5s)  
✅ **AI assistant compatibility** restored  
✅ **Enhanced search still available** with `-c medium/full`  
✅ **Backward compatibility** for scripts (just faster)  
✅ **Intelligent fallback** mechanism for edge cases  
✅ **Better precision** with improved matching algorithms  

The migration to v0.5.1 provides immediate performance benefits while maintaining all functionality through explicit context options.