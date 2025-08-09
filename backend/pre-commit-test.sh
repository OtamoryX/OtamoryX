#!/bin/bash

# Pre-commit test script for OtamoryX backend
# This script runs all tests before allowing commits

set -e

echo "🧪 Running OtamoryX Backend Tests..."
echo "=================================="

# Change to the backend directory
cd "$(dirname "$0")"

# Check if database exists and create if needed
if [ ! -f "otamoryx.db" ]; then
    echo "📁 Database not found, creating..."
    sqlite3 otamoryx.db < migrations/init.sql
fi

# Run cargo check for fast compilation check
echo "🔍 Checking compilation..."
cargo check

# Run cargo clippy for linting
echo "🔧 Running linting..."
cargo clippy -- -D warnings

# Run unit tests
echo "🧩 Running unit tests..."
cargo test --lib

# Run integration tests
echo "🔗 Running integration tests..."
cargo test --test integration_tests

# Clean up any test databases
echo "🧹 Cleaning up test databases..."
rm -f test_*.db

echo "✅ All tests passed! Ready to commit."