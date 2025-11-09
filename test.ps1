# Test script for Windows (PowerShell)

Write-Host "🧪 Running tests..." -ForegroundColor Cyan

cargo test --workspace

if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ All tests passed!" -ForegroundColor Green
} else {
    Write-Host "❌ Some tests failed!" -ForegroundColor Red
    exit 1
}

