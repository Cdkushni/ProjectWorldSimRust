#!/bin/bash
# Test script for Linux/Mac

echo "🧪 Running tests..."

cargo test --workspace

if [ $? -eq 0 ]; then
    echo "✅ All tests passed!"
else
    echo "❌ Some tests failed!"
    exit 1
fi

