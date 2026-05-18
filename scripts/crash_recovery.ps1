<#
.SYNOPSIS
Runs Prime with crash recovery flags and diagnostics.

.DESCRIPTION
This script launches Prime with crash recovery mode enabled.
It checks for stale lock files, attempts to restore from last
checkpoint, and runs the application with recovery flags.

.PARAMETER Mode
Recovery mode: 'check' (only check), 'recover' (attempt recovery), 'run' (recover then run)

.PARAMETER DataDir
Path to Prime data directory

.EXAMPLE
.\scripts\crash_recovery.ps1 -Mode run
.\scripts\crash_recovery.ps1 -Mode check
#>

param(
    [ValidateSet("check", "recover", "run")]
    [string]$Mode = "run",
    [string]$DataDir = "$env:APPDATA\Prime"
)

$ErrorActionPreference = "Continue"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Resolve-Path "$ScriptDir\.."
$BinaryPath = Join-Path $ProjectRoot "src-tauri\target\release\prime.exe"

Write-Host "🔧 Prime Crash Recovery Tool" -ForegroundColor Cyan
Write-Host "  Mode:    $Mode" -ForegroundColor White
Write-Host "  Data:    $DataDir" -ForegroundColor White
Write-Host "  Binary:  $BinaryPath" -ForegroundColor White
Write-Host ""

# ── Check for stale lock files ──────────────────────────────────────────
function Check-StaleLocks {
    Write-Host "📋 Checking for stale lock files..." -ForegroundColor Yellow
    $lockFiles = @(
        "$DataDir\prime.lock",
        "$DataDir\index.lock",
        "$DataDir\memory.lock"
    )

    $found = @()
    foreach ($lf in $lockFiles) {
        if (Test-Path $lf) {
            $age = (Get-Date) - (Get-Item $lf).LastWriteTime
            Write-Host "  ⚠️  Stale lock: $lf ($($age.TotalMinutes) minutes old)" -ForegroundColor Red
            $found += $lf
        }
    }

    if ($found.Count -eq 0) {
        Write-Host "  ✅ No stale lock files found" -ForegroundColor Green
    }

    return $found
}

# ── Restore from last checkpoint ────────────────────────────────────────
function Restore-Checkpoint {
    Write-Host "📋 Checking for checkpoints..." -ForegroundColor Yellow
    $checkpointDir = "$DataDir\checkpoints"
    if (-not (Test-Path $checkpointDir)) {
        Write-Host "  ℹ️  No checkpoint directory found" -ForegroundColor Cyan
        return $false
    }

    $checkpoints = Get-ChildItem "$checkpointDir\*.zip" | Sort-Object LastWriteTime -Descending
    if ($checkpoints.Count -eq 0) {
        Write-Host "  ℹ️  No checkpoints found" -ForegroundColor Cyan
        return $false
    }

    $latest = $checkpoints[0]
    Write-Host "  📦 Latest checkpoint: $($latest.Name) ($((Get-Date) - $latest.LastWriteTime | Select-Object -ExpandProperty TotalMinutes) min ago)" -ForegroundColor Cyan

    if ($Mode -eq "recover" -or $Mode -eq "run") {
        Write-Host "  🔄 Restoring from checkpoint..." -ForegroundColor Yellow
        try {
            # Backup current state before restoring
            $backupDir = "$DataDir\pre_restore_backup"
            if (Test-Path $backupDir) { Remove-Item -Recurse -Force $backupDir }
            New-Item -ItemType Directory -Force -Path $backupDir | Out-Null

            @("memory.db", "index", "config.json") | ForEach-Object {
                $src = Join-Path $DataDir $_
                if (Test-Path $src) {
                    Copy-Item -Recurse $src $backupDir
                }
            }

            # Extract checkpoint
            Expand-Archive -Path $latest.FullName -DestinationPath "$DataDir\temp_restore" -Force

            # Copy restored files
            $restoreSrc = "$DataDir\temp_restore"
            if (Test-Path $restoreSrc) {
                Get-ChildItem $restoreSrc | ForEach-Object {
                    $target = Join-Path $DataDir $_.Name
                    if ($_.PSIsContainer) {
                        Copy-Item -Recurse -Force $_.FullName $target
                    } else {
                        Copy-Item -Force $_.FullName $target
                    }
                }
                Remove-Item -Recurse -Force $restoreSrc
            }

            # Remove stale lock files
            @("$DataDir\prime.lock", "$DataDir\index.lock", "$DataDir\memory.lock") | ForEach-Object {
                if (Test-Path $_) { Remove-Item -Force $_ }
            }

            Write-Host "  ✅ Restore complete" -ForegroundColor Green
            return $true
        } catch {
            Write-Host "  ❌ Restore failed: $_" -ForegroundColor Red
            return $false
        }
    }
    return $false
}

# ── Run Prime ───────────────────────────────────────────────────────────
function Run-Prime {
    if (-not (Test-Path $BinaryPath)) {
        Write-Host "❌ Binary not found at $BinaryPath" -ForegroundColor Red
        Write-Host "   Build with: cd src-tauri && cargo build --release" -ForegroundColor Yellow
        return
    }

    Write-Host "🚀 Starting Prime with crash recovery..." -ForegroundColor Cyan
    Write-Host ""

    $env:PRIME_RECOVERY_MODE = "true"
    $env:PRIME_DATA_DIR = $DataDir

    & $BinaryPath
}

# ── Main ────────────────────────────────────────────────────────────────
$locks = Check-StaleLocks

if ($locks.Count -gt 0 -and ($Mode -eq "recover" -or $Mode -eq "run")) {
    Write-Host ""
    Write-Host "🔧 Attempting recovery..." -ForegroundColor Yellow
    foreach ($lf in $locks) {
        try {
            Remove-Item $lf -Force
            Write-Host "  ✅ Removed: $lf" -ForegroundColor Green
        } catch {
            Write-Host "  ❌ Failed to remove: $lf - $_" -ForegroundColor Red
        }
    }
}

if ($Mode -eq "recover" -or $Mode -eq "run") {
    $restored = Restore-Checkpoint
    if (-not $restored) {
        Write-Host "  ℹ️  No checkpoint restored, starting fresh" -ForegroundColor Cyan
    }
}

if ($Mode -eq "run") {
    Write-Host ""
    Run-Prime
}

Write-Host ""
Write-Host "✅ Crash recovery check complete" -ForegroundColor Green
