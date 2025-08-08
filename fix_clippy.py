#!/usr/bin/env python3
"""
Script to fix specific clippy warnings systematically.
"""
import re
import glob

def fix_format_strings(content):
    """Fix uninlined format args warnings."""
    
    # Fix format!() patterns
    # format!("text {}", var) -> format!("text {var}")
    content = re.sub(r'format!\("([^"]*?)\{\}([^"]*?)", (\w+)\)', r'format!("\1{\3}\2")', content)
    
    # Fix format!() with format specifiers
    # format!("text {:?}", var) -> format!("text {var:?}")
    content = re.sub(r'format!\("([^"]*?)\{([^}]+)\}([^"]*?)", (\w+)\)', r'format!("\1{\4:\2}\3")', content)
    
    # Fix println!() patterns
    # println!("text {}", var) -> println!("text {var}")
    content = re.sub(r'println!\("([^"]*?)\{\}([^"]*?)", (\w+)\)', r'println!("\1{\3}\2")', content)
    
    # Fix println!() with format specifiers
    # println!("text {:?}", var) -> println!("text {var:?}")
    content = re.sub(r'println!\("([^"]*?)\{([^}]+)\}([^"]*?)", (\w+)\)', r'println!("\1{\4:\2}\3")', content)
    
    # Fix multiline println! 
    content = re.sub(
        r'println!\(\s*"([^"]*?)\{\}([^"]*?)",\s*(\w+)\s*\)',
        r'println!("\1{\3}\2")',
        content,
        flags=re.MULTILINE | re.DOTALL
    )
    
    # Fix multiline println! with format specifiers
    content = re.sub(
        r'println!\(\s*"([^"]*?)\{([^}]+)\}([^"]*?)",\s*(\w+)\s*\)',
        r'println!("\1{\4:\2}\3")',
        content,
        flags=re.MULTILINE | re.DOTALL
    )
    
    # Fix assert! patterns
    content = re.sub(
        r'assert!\(\s*([^,]+),\s*"([^"]*?)\{([^}]*)\}([^"]*?)",\s*(\w+)\s*\)',
        r'assert!(\1, "\2{\5:\3}\4")',
        content,
        flags=re.MULTILINE | re.DOTALL
    )
    
    return content

def fix_unused_variables(content):
    """Fix unused variable warnings by adding underscore prefix."""
    
    # Common unused variables in the codebase
    unused_vars = [
        'reader_id', 'results', 'final_results', 'pattern_type', 'pattern_id',
        'read_lock_acquisitions', 'write_lock_acquisitions', 'thread_id',
        'concurrent_access_count', 'modifier_id', 'indexer_id', 'indexer_type',
        'storage_count', 'primary_count', 'trigram_count'
    ]
    
    for var in unused_vars:
        # Fix struct field definitions
        content = re.sub(f'    {var}:', f'    _{var}:', content)
        # Fix variable declarations
        content = re.sub(f'let {var} =', f'let _{var} =', content)
    
    return content

def fix_single_match(content):
    """Fix single match patterns to if let."""
    
    # Pattern: match expr { Ok(x) => action, _ => {} }
    content = re.sub(
        r'match\s+([^{]+)\s*\{\s*Ok\(([^)]*)\)\s*=>\s*([^,]+),\s*_\s*=>\s*\{\s*\}\s*([^}]*)\}',
        r'if let Ok(\2) = \1 { \3 }',
        content,
        flags=re.MULTILINE | re.DOTALL
    )
    
    return content

def fix_collapsible_patterns(content):
    """Fix collapsible if let patterns."""
    
    # Pattern: if let Ok(result) = handle.await { if let Ok(writes) = result { ... } }
    content = re.sub(
        r'if let Ok\(([^)]+)\) = ([^{]+) \{\s*if let Ok\(([^)]+)\) = \1 \{([^}]+)\}\s*\}',
        r'if let Ok(Ok(\3)) = \2 {\4}',
        content,
        flags=re.MULTILINE | re.DOTALL
    )
    
    return content

def fix_file(filepath):
    """Fix a single file."""
    try:
        with open(filepath, 'r') as f:
            content = f.read()
        
        original_content = content
        
        # Apply fixes
        content = fix_format_strings(content)
        content = fix_unused_variables(content)
        content = fix_single_match(content)
        content = fix_collapsible_patterns(content)
        
        if content != original_content:
            with open(filepath, 'w') as f:
                f.write(content)
            print(f"Fixed: {filepath}")
            return True
        else:
            print(f"No changes: {filepath}")
            return False
            
    except Exception as e:
        print(f"Error fixing {filepath}: {e}")
        return False

def main():
    """Main function."""
    
    # Find all Rust files in tests/ and benches/
    test_files = glob.glob('tests/*.rs')
    bench_files = glob.glob('benches/*.rs')
    
    all_files = test_files + bench_files
    
    fixed_count = 0
    for filepath in all_files:
        if fix_file(filepath):
            fixed_count += 1
    
    print(f"\nFixed {fixed_count} files out of {len(all_files)} total files.")

if __name__ == "__main__":
    main()
