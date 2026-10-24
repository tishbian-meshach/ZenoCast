<#
.SYNOPSIS
    Build and run Zenocast locally for development -- no install required.

.DESCRIPTION
    Builds zeno-app (debug by default) and launches the binary from target/,
    with console output attached for tracing logs. Supports -Release for
    optimized builds, -Clean for from-scratch builds, and reads dev.toml for
    per-developer overrides.

.PARAMETER Release
    Build with --release for performance testing.

.PARAMETER Clean
    Delete target/ before building for a clean compile.

.EXAMPLE
    .\scripts\dev-run.ps1
    .\scripts\dev-run.ps1 -Release
    .\scripts\dev-run.ps1 -Clean
    .\scripts\dev-run.ps1 -Clean -Release
#>
param(
    [switch]$Release,
    [switch]$Clean
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# --- Resolve paths ---
$ProjectRoot = (Resolve-Path "$PSScriptRoot\..").Path
$DevToml     = Join-Path $ProjectRoot "dev.toml"
$LogDir      = Join-Path $ProjectRoot "target" "dev-logs"

# --- Environment setup (same as CI) ---
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Program Files\LLVM\bin;C:\ffmpeg\bin;$env:PATH"
$env:RUST_LOG = "info,zeno_ui=debug,zeno_output=debug,zeno_media=debug,zeno_compositor=debug,wgpu=warn,naga=warn"
$env:RUST_BACKTRACE = "1"

# --- Parse dev.toml if present ---
$DevConfig = @{}
if (Test-Path $DevToml) {
    Write-Host "[dev-run] Loading dev.toml..." -ForegroundColor DarkCyan
    Get-Content $DevToml | ForEach-Object {
        $line = $_.Trim()
        if ($line -and -not $line.StartsWith('#')) {
            $parts = $line -split '\s*=\s*', 2
            if ($parts.Length -eq 2) {
                $key = $parts[0].Trim()
                $val = $parts[1].Trim().Trim('"').Trim("'")
                $DevConfig[$key] = $val
            }
        }
    }
}

# Propagate dev.toml flags as environment variables for the app to read
if ($DevConfig.ContainsKey("skip_license_check") -and $DevConfig["skip_license_check"] -eq "true") {
    $env:ZENOCAST_SKIP_LICENSE = "1"
}
if ($DevConfig.ContainsKey("default_project")) {
    $env:ZENOCAST_DEV_PROJECT = $DevConfig["default_project"]
}
if ($DevConfig.ContainsKey("mock_sources") -and $DevConfig["mock_sources"] -eq "true") {
    $env:ZENOCAST_MOCK_SOURCES = "1"
}

# --- Clean if requested ---
if ($Clean) {
    Write-Host "[dev-run] Cleaning target/..." -ForegroundColor Yellow
    $targetDir = Join-Path $ProjectRoot "target"
    if (Test-Path $targetDir) {
        Remove-Item -Recurse -Force $targetDir
    }
}

# --- Build ---
$buildProfile = if ($Release) { "release" } else { "debug" }
$buildArgs = @("build", "-p", "zeno-app")
if ($Release) {
    $buildArgs += "--release"
}

Write-Host ""
Write-Host "================================================================" -ForegroundColor Cyan
Write-Host "  Zenocast Dev Build ($buildProfile)" -ForegroundColor Cyan
Write-Host "================================================================" -ForegroundColor Cyan
Write-Host ""

$buildStart = Get-Date
Write-Host "[dev-run] cargo $($buildArgs -join ' ')" -ForegroundColor DarkGray

Push-Location $ProjectRoot
try {
    & cargo @buildArgs
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[dev-run] Build failed (exit code $LASTEXITCODE)." -ForegroundColor Red
        exit $LASTEXITCODE
    }
} finally {
    Pop-Location
}

$buildElapsed = ((Get-Date) - $buildStart).TotalSeconds
Write-Host "[dev-run] Build completed in $([math]::Round($buildElapsed, 1))s" -ForegroundColor Green

# --- Resolve binary ---
$exePath = "$ProjectRoot\target\$buildProfile\zenocast.exe"
if (-not (Test-Path $exePath)) {
    Write-Host "[dev-run] ERROR: Binary not found at $exePath" -ForegroundColor Red
    exit 1
}

# --- Set up rotating dev log ---
if (-not (Test-Path $LogDir)) {
    New-Item -ItemType Directory -Path $LogDir -Force | Out-Null
}
$timestamp = Get-Date -Format "yyyy-MM-dd_HH-mm-ss"
$logFile = Join-Path $LogDir "zenocast_$timestamp.log"

# Keep only the 10 most recent log files
Get-ChildItem $LogDir -Filter "zenocast_*.log" | Sort-Object LastWriteTime -Descending | Select-Object -Skip 10 | Remove-Item -Force -ErrorAction SilentlyContinue

# --- Launch ---
Write-Host ""
Write-Host "================================================================" -ForegroundColor Cyan
Write-Host "  Launching: $exePath" -ForegroundColor Cyan
Write-Host "  Log file:  $logFile" -ForegroundColor Cyan
Write-Host "  Press Ctrl+C to stop" -ForegroundColor DarkGray
Write-Host "================================================================" -ForegroundColor Cyan
Write-Host ""

# Run the binary, piping output to both console and log file via Tee-Object
& $exePath 2>&1 | Tee-Object -FilePath $logFile

$exitCode = $LASTEXITCODE
if ($exitCode -ne 0) {
    Write-Host "[dev-run] Zenocast exited with code $exitCode" -ForegroundColor Yellow
} else {
    Write-Host "[dev-run] Zenocast exited cleanly." -ForegroundColor Green
}
exit $exitCode
