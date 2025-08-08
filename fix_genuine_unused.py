#!/usr/bin/env python3
"""
Script to fix only genuinely unused variables by carefully analyzing the code.
"""
import re
import ast

def is_variable_used(content, var_name, definition_line):
    """Check if a variable is actually used after its definition."""
    lines = content.split('\n')
    
    # Look for usage after the definition line
    for i, line in enumerate(lines[definition_line:], definition_line):
        # Skip the definition line itself
        if i == definition_line:
            continue
            
        # Check if variable is used (not just assigned)
        if re.search(f'\\b{re.escape(var_name)}\\b', line):
            # Make sure it's not another assignment
            if not re.match(f'\\s*let\\s+{re.escape(var_name)}\\s*=', line.strip()):
                return True
    
    return False

def fix_unused_variable_in_file(file_path, var_name, line_number):
    """Fix a single unused variable in a file."""
    try:
        with open(file_path, 'r') as f:
            lines = f.readlines()
        
        if line_number >= len(lines):
            return False
            
        line = lines[line_number]
        
        # Check if the variable is actually used
        content = ''.join(lines)
        if is_variable_used(content, var_name, line_number):
            print(f"Variable '{var_name}' is actually used, skipping")
            return False
        
        # Replace the variable name with underscore prefix
        new_line = re.sub(f'\\blet\\s+{re.escape(var_name)}\\b', f'let _{var_name}', line)
        
        if new_line != line:
            lines[line_number] = new_line
            
            with open(file_path, 'w') as f:
                f.writelines(lines)
                
            print(f"Fixed unused variable '{var_name}' in {file_path}:{line_number+1}")
            return True
        
        return False
        
    except Exception as e:
        print(f"Error fixing {var_name} in {file_path}: {e}")
        return False

# Manually specify genuinely unused variables based on clippy output
unused_variables = [
    ("benches/query_routing_performance.rs", "use_primary", 88),
    ("tests/primary_index_tests.rs", "results", 298), 
    ("tests/primary_index_edge_cases_test.rs", "index", 28),
    ("tests/primary_index_edge_cases_test.rs", "doc_id", 31),
    ("tests/primary_index_edge_cases_test.rs", "doc_path", 32),
    ("tests/btree_algorithms_test.rs", "doc_id", 183),
    ("tests/btree_algorithms_test.rs", "path", 184),
    ("tests/storage_index_integration_test.rs", "doc_id", 54), 
    ("tests/storage_index_integration_test.rs", "doc_id", 90),
    ("tests/storage_index_integration_test.rs", "doc_id", 125),
    ("tests/storage_index_integration_test.rs", "doc_id", 183),
    ("tests/storage_index_integration_test.rs", "storage_guard", 260),
    ("tests/storage_index_integration_test.rs", "doc_id", 235),
    ("tests/storage_index_integration_test.rs", "doc_id", 371),
    ("tests/storage_index_integration_test.rs", "doc_id", 404),
    ("benches/concurrent_performance.rs", "reader_id", 293),
    ("tests/query_routing_stress.rs", "doc_ids", 273),
    ("tests/query_routing_stress.rs", "permit", 304),
    ("tests/query_routing_stress.rs", "doc_ids", 427),
    ("tests/query_routing_stress.rs", "doc_ids", 598),
    ("tests/query_routing_stress.rs", "permit", 611),
    ("tests/query_routing_stress.rs", "permit", 648),
    ("tests/query_routing_stress.rs", "doc_ids", 756),
    ("tests/query_routing_stress.rs", "doc_ids", 905),
    ("tests/query_routing_stress.rs", "use_primary", 945),
    ("tests/production_configuration_test.rs", "docs", 832),
    ("tests/production_configuration_test.rs", "storage", 47),
    ("tests/production_configuration_test.rs", "optimized_index", 57),
    ("tests/production_configuration_test.rs", "stress_duration", 605),
    ("tests/observability_integration_test.rs", "perf_timer", 121),
]

def main():
    """Main function."""
    fixed_count = 0
    
    for file_path, var_name, line_number in unused_variables:
        if fix_unused_variable_in_file(file_path, var_name, line_number):
            fixed_count += 1
    
    print(f"\nFixed {fixed_count} genuinely unused variables.")

if __name__ == "__main__":
    main()
