param(
    [ValidateSet("debug", "release")]
    [string]$Configuration = "release"
)

Write-Host "=== Prime Build Script ===" -ForegroundColor Cyan
Write-Host "Configuration: $Configuration" -ForegroundColor Yellow

# Check prerequisites
$hasRust = Get-Command "cargo" -ErrorAction SilentlyContinue
$hasNode = Get-Command "node" -ErrorAction SilentlyContinue
$hasNpm = Get-Command "npm" -ErrorAction SilentlyContinue

if (-not $hasRust) {
    Write-Error "Rust not found. Install from https://rustup.rs"
    exit 1
}
if (-not $hasNode -or -not $hasNpm) {
    Write-Error "Node.js/npm not found. Install from https://nodejs.org"
    exit 1
}

# Install npm dependencies
Write-Host "`n=== Installing npm dependencies ===" -ForegroundColor Cyan
npm install
if ($LASTEXITCODE -ne 0) {
    Write-Error "npm install failed"
    exit 1
}

# Build frontend
Write-Host "`n=== Building frontend ===" -ForegroundColor Cyan
npm run build
if ($LASTEXITCODE -ne 0) {
    Write-Error "Frontend build failed"
    exit 1
}

# Build Rust backend
Write-Host "`n=== Building Rust backend ===" -ForegroundColor Cyan
$buildFlag = if ($Configuration -eq "release") { "--release" } else { "" }
cargo build $buildFlag
if ($LASTEXITCODE -ne 0) {
    Write-Error "Rust build failed"
    exit 1
}

Write-Host "`n=== Build Complete ===" -ForegroundColor Green
Write-Host "Binary: target\$Configuration\prime.exe"
