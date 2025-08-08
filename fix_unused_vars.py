#!/usr/bin/env python3
"""
Script to fix unused variable warnings by adding underscore prefix.
"""
import re
import subprocess
import sys

def get_unused_variables():
    """Get unused variables from clippy output."""
    try:
        result = subprocess.run(
            ['cargo', 'clippy', '--all-targets', '--all-features', '--', '-D', 'warnings'],
            capture_output=True,
            text=True,
            cwd='.'
        )
        
        unused_vars = []
        lines = result.stderr.split('\n')
        
        for i, line in enumerate(lines):
            if 'unused variable:' in line:
                # Extract variable name and file
                var_match = re.search(r'unused variable: `([^`]+)`', line)
                if var_match:
                    var_name = var_match.group(1)
                    
                    # Find the file path from previous lines
                    for j in range(max(0, i-10), i):
                        if lines[j].strip().startswith('-->'):
                            file_match = re.search(r'--> ([^:]+):', lines[j])
                            if file_match:
                                file_path = file_match.group(1)
                                unused_vars.append((file_path, var_name))
                                break
        
        return unused_vars
        
    except Exception as e:
        print(f"Error getting clippy output: {e}")
        return []

def fix_unused_variable(file_path, var_name):
    """Fix a single unused variable by adding underscore prefix."""
    try:
        with open(file_path, 'r') as f:
            content = f.read()
        
        # Pattern 1: let var_name = 
        content = re.sub(
            f'(let\\s+){var_name}(\\s*=)',
            f'\\1_{var_name}\\2',
            content
        )
        
        # Pattern 2: struct field declarations
        content = re.sub(
            f'(\\s+){var_name}(:)',
            f'\\1_{var_name}\\2',
            content
        )
        
        # Pattern 3: function parameters
        content = re.sub(
            f'([,(\\s]+){var_name}(\\s*:)',
            f'\\1_{var_name}\\2',
            content
        )
        
        with open(file_path, 'w') as f:
            f.write(content)
            
        print(f"Fixed unused variable '{var_name}' in {file_path}")
        return True
        
    except Exception as e:
        print(f"Error fixing {var_name} in {file_path}: {e}")
        return False

def main():
    """Main function."""
    print("Finding unused variables...")
    unused_vars = get_unused_variables()
    
    if not unused_vars:
        print("No unused variables found!")
        return
    
    print(f"Found {len(unused_vars)} unused variables:")
    for file_path, var_name in unused_vars:
        print(f"  {var_name} in {file_path}")
    
    print("\nFixing unused variables...")
    fixed_count = 0
    for file_path, var_name in unused_vars:
        if fix_unused_variable(file_path, var_name):
            fixed_count += 1
    
    print(f"\nFixed {fixed_count}/{len(unused_vars)} unused variables.")

if __name__ == "__main__":
    main()
