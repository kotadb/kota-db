#!/bin/bash
# Demo script for KotaDB CLI

echo "🚀 KotaDB CLI Demo"
echo "=================="
echo ""

# Build the CLI
echo "Building KotaDB CLI..."
cargo build --bin kotadb 2>/dev/null

# Set the binary path
KOTADB="./target/debug/kotadb"

# Create a test database directory
DB_PATH="/tmp/kotadb-demo"
rm -rf $DB_PATH
mkdir -p $DB_PATH

echo "✅ Database created at: $DB_PATH"
echo ""

# Insert some documents
echo "📝 Inserting documents..."
echo ""

# Document 1
echo "# Rust Programming Guide

Rust is a systems programming language focused on safety, speed, and concurrency." | \
$KOTADB --db-path $DB_PATH insert "/docs/rust-guide.md" "Rust Programming Guide"

# Document 2
echo "# Database Design Patterns

This document covers common patterns in database design including:
- Normalization
- Indexing strategies
- Query optimization" | \
$KOTADB --db-path $DB_PATH insert "/docs/db-patterns.md" "Database Design Patterns"

# Document 3
echo "# KotaDB Architecture

KotaDB is built with a focus on:
- Type safety through validated types
- Pure functional core
- Comprehensive testing" | \
$KOTADB --db-path $DB_PATH insert "/docs/kotadb-arch.md" "KotaDB Architecture"

echo ""
echo "📋 Listing all documents..."
echo ""
$KOTADB --db-path $DB_PATH list

echo ""
echo "🔍 Searching for 'database'..."
echo ""
$KOTADB --db-path $DB_PATH search "database"

echo ""
echo "📊 Database statistics..."
echo ""
$KOTADB --db-path $DB_PATH stats

echo ""
echo "✅ Demo complete!"