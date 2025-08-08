#!/bin/bash

# Add allow directives to test files to suppress format string warnings
find tests/ benches/ -name "*.rs" -exec grep -L "allow(clippy::uninlined_format_args)" {} \; | while read file; do
    # Add allow directive at top of file after any existing module comments
    if grep -q "^#!" "$file"; then
        # File has inner attributes, add after them
        sed -i '' '1,/^#!\[/{ /^#!\[/a\
#![allow(clippy::uninlined_format_args)]
}' "$file"
    else
        # No inner attributes, add at the very top
        sed -i '' '1i\
#![allow(clippy::uninlined_format_args)]\
' "$file"
    fi
done

# Fix remaining genuinely unused variables with sed (specific patterns)
find tests/ benches/ -name "*.rs" -exec sed -i '' \
    -e 's/let index = kotadb::/let _index = kotadb::/g' \
    -e 's/let doc_id = ValidatedDocumentId::/let _doc_id = ValidatedDocumentId::/g' \
    -e 's/let doc_path = ValidatedPath::/let _doc_path = ValidatedPath::/g' \
    -e 's/let recovered_index = /let _recovered_index = /g' \
    -e 's/let new_doc_id = /let _new_doc_id = /g' \
    -e 's/let new_doc_path = /let _new_doc_path = /g' \
    -e 's/let index_path = /let _index_path = /g' \
    -e 's/let permit = semaphore/let _permit = semaphore/g' \
    -e 's/let final_results = /let _final_results = /g' \
    {} \;

# Fix specific unused results that don't use the variable
find tests/ -name "*.rs" -exec sed -i '' \
    -e 's/let results = index_guard\.search/let _results = index_guard.search/g' \
    -e 's/let results = primary_index\.search/let _results = primary_index.search/g' \
    -e 's/let results = trigram_index\.search/let _results = trigram_index.search/g' \
    -e 's/let results = idx\.search(&query)\.await\.unwrap()/let _results = idx.search(\&query).await.unwrap()/g' \
    {} \;
    
echo "Applied clippy fixes"
