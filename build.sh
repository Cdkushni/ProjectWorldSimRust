#!/bin/bash
# Build script for Linux/Mac

echo "🔨 Building World Simulation Server..."

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Cargo not found. Please install Rust from https://rustup.rs/"
    exit 1
fi

echo "📦 Building in release mode..."
cargo build --release

if [ $? -eq 0 ]; then
    echo "✅ Build successful!"
    echo ""
    echo "To run the server:"
    echo "  ./target/release/sim_server"
    echo ""
    echo "Admin API will be available at:"
    echo "  http://127.0.0.1:8080"
else
    echo "❌ Build failed!"
    exit 1
fi

