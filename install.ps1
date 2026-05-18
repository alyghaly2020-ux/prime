# Prime Installer for Windows
# Usage: powershell -c "irm https://raw.githubusercontent.com/alyghaly2020-ux/prime/master/install.ps1 | iex"

$Repo = "alyghaly2020-ux/prime"
$ErrorActionPreference = "Stop"

function Write-Info  { Write-Host "  $_" -ForegroundColor Cyan }
function Write-Ok   { Write-Host "  $_" -ForegroundColor Green }
function Write-Warn { Write-Host "  $_" -ForegroundColor Yellow }
function Write-Err  { Write-Host "  $_" -ForegroundColor Red }

function Get-LatestVersion {
    $api = "https://api.github.com/repos/$Repo/releases/latest"
    $release = Invoke-RestMethod -Uri $api -Headers @{ "User-Agent" = "prime-installer" }
    return $release.tag_name -replace "^v", ""
}

function Install-Prime {
    Write-Host ""
    Write-Host "  Prime Installer" -ForegroundColor Cyan
    Write-Host ""

    $version = Get-LatestVersion
    $asset = "Prime_${version}_x64_en-US.exe"
    $url = "https://github.com/$Repo/releases/download/v${version}/$asset"
    $tmp = "$env:TEMP\prime-installer.exe"

    Write-Info "Downloading Prime ${version} for Windows..."
    Invoke-WebRequest -Uri $url -OutFile $tmp -UseBasicParsing

    Write-Info "Running installer..."
    Start-Process -FilePath $tmp -Wait

    Remove-Item -Force $tmp -ErrorAction SilentlyContinue

    Write-Host ""
    Write-Ok "Prime ${version} installed!"
    Write-Host "  Launch from Start Menu or run:  prime" -ForegroundColor Cyan
    Write-Host "  Headless:  prime headless --port 9876" -ForegroundColor Cyan
    Write-Host ""
    Write-Warn "Need the full installer package or previous versions?"
    Write-Host "  → https://github.com/$Repo/releases" -ForegroundColor Gray
    Write-Host ""
}

Install-Prime
