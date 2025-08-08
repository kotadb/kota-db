#!/usr/bin/env python3
"""
Script to systematically fix all clippy uninlined format args warnings.
"""

import re
import sys
from pathlib import Path

def fix_format_args(content):
    """Fix uninlined format args in content."""
    
    # Pattern 1: Simple format! with single variable
    # format!("text {}", var) -> format!("text {var}")
    content = re.sub(
        r'format!\(\s*"([^"]*)\{\}([^"]*)",\s*(\w+)\s*\)',
        lambda m: f'format!("{m.group(1)}{{{m.group(3)}}}{m.group(2)}")',
        content
    )
    
    # Pattern 2: format! with multiple variables  
    # format!("text {} {}", var1, var2) -> format!("text {var1} {var2}")
    def replace_multi_format(match):
        template = match.group(1)
        vars_str = match.group(2)
        vars_list = [v.strip() for v in vars_str.split(',')]
        
        # Replace {} placeholders with {var} in order
        result_template = template
        for var in vars_list:
            result_template = result_template.replace('{}', f'{{{var}}}', 1)
        
        return f'format!("{result_template}")'
    
    content = re.sub(
        r'format!\(\s*"([^"]*)",\s*([^)]+)\s*\)',
        replace_multi_format,
        content
    )
    
    # Pattern 3: format! with format specifiers like {:?}, {:.1}
    # format!("text {:?}", var) -> format!("text {var:?}")
    content = re.sub(
        r'format!\(\s*"([^"]*)\{([^}]*)\}([^"]*)",\s*(\w+)\s*\)',
        lambda m: f'format!("{m.group(1)}{{{m.group(4)}:{m.group(2)}}}{m.group(3)}")',
        content
    )
    
    # Pattern 4: println! with format args
    # println!("text {}", var) -> println!("text {var}")
    content = re.sub(
        r'println!\(\s*"([^"]*)\{\}([^"]*)",\s*(\w+)\s*\)',
        lambda m: f'println!("{m.group(1)}{{{m.group(3)}}}{m.group(2)}")',
        content
    )
    
    # Pattern 5: println! with format specifiers
    # println!("text {:?}", var) -> println!("text {var:?}")
    content = re.sub(
        r'println!\(\s*"([^"]*)\{([^}]*)\}([^"]*)",\s*(\w+)\s*\)',
        lambda m: f'println!("{m.group(1)}{{{m.group(4)}:{m.group(2)}}}{m.group(3)}")',
        content
    )
    
    return content

def fix_file(file_path):
    """Fix format args in a single file."""
    try:
        with open(file_path, 'r') as f:
            content = f.read()
        
        fixed_content = fix_format_args(content)
        
        if content != fixed_content:
            with open(file_path, 'w') as f:
                f.write(fixed_content)
            print(f"Fixed: {file_path}")
            return True
        else:
            print(f"No changes: {file_path}")
            return False
            
    except Exception as e:
        print(f"Error processing {file_path}: {e}")
        return False

def main():
    """Main function."""
    if len(sys.argv) < 2:
        print("Usage: python3 fix_format_strings.py <file1> [file2] ...")
        sys.exit(1)
    
    files_changed = 0
    for file_path in sys.argv[1:]:
        path = Path(file_path)
        if path.exists():
            if fix_file(path):
                files_changed += 1
        else:
            print(f"File not found: {file_path}")
    
    print(f"\nTotal files changed: {files_changed}")

if __name__ == "__main__":
    main()
