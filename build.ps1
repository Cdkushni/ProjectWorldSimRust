# Build script for Windows (PowerShell)

Write-Host "🔨 Building World Simulation Server..." -ForegroundColor Cyan

# Check if Rust is installed
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "❌ Cargo not found. Please install Rust from https://rustup.rs/" -ForegroundColor Red
    exit 1
}

Write-Host "📦 Building in release mode..." -ForegroundColor Yellow
cargo build --release

if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ Build successful!" -ForegroundColor Green
    Write-Host ""
    Write-Host "To run the server:" -ForegroundColor Cyan
    Write-Host "  .\target\release\sim_server.exe" -ForegroundColor White
    Write-Host ""
    Write-Host "Admin API will be available at:" -ForegroundColor Cyan
    Write-Host "  http://127.0.0.1:8080" -ForegroundColor White
} else {
    Write-Host "❌ Build failed!" -ForegroundColor Red
    exit 1
}

