<#
.SYNOPSIS
    Watch for source changes and auto-rebuild + relaunch Zenocast.

.DESCRIPTION
    Uses a polling loop to detect .rs file changes under crates/, then
    rebuilds and relaunches zenocast.exe automatically. Falls back to
    relaunch (not hot-reload) since egui/eframe doesn't support live
    code swapping.

.PARAMETER Release
    Build with --release.

.PARAMETER PollSeconds
    How often to check for file changes. Default: 2.
#>
param(
    [switch]$Release,
    [int]$PollSeconds = 2
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ProjectRoot = (Resolve-Path "$PSScriptRoot\..").Path
$DevRunScript = Join-Path $PSScriptRoot "dev-run.ps1"

function Get-SourceHash {
    $files = Get-ChildItem -Path (Join-Path $ProjectRoot "crates") -Recurse -Include "*.rs","Cargo.toml" -File
    $hashes = $files | ForEach-Object {
        "$($_.FullName):$($_.LastWriteTime.Ticks)"
    }
    return ($hashes | Sort-Object) -join "`n"
}

Write-Host ""
Write-Host "================================================================" -ForegroundColor Magenta
Write-Host "  Zenocast Dev Watch (polling every ${PollSeconds}s)" -ForegroundColor Magenta
Write-Host "  Ctrl+C to stop watching" -ForegroundColor DarkGray
Write-Host "================================================================" -ForegroundColor Magenta
Write-Host ""

$lastHash = ""
$proc = $null

function Start-App {
    $buildProfile = if ($Release) { "release" } else { "debug" }
    $buildArgs = @("build", "-p", "zeno-app")
    if ($Release) { $buildArgs += "--release" }

    $env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Program Files\LLVM\bin;C:\ffmpeg\bin;$env:PATH"
    $env:RUST_LOG = "info,zeno_ui=debug,wgpu=warn,naga=warn"
    $env:RUST_BACKTRACE = "1"

    Write-Host "[dev-watch] Building..." -ForegroundColor Cyan
    $buildStart = Get-Date
    Push-Location $ProjectRoot
    try {
        & cargo @buildArgs
        if ($LASTEXITCODE -ne 0) {
            Write-Host "[dev-watch] Build FAILED. Waiting for next change..." -ForegroundColor Red
            return $null
        }
    } finally {
        Pop-Location
    }
    $elapsed = [math]::Round(((Get-Date) - $buildStart).TotalSeconds, 1)
    Write-Host "[dev-watch] Built in ${elapsed}s -- launching..." -ForegroundColor Green

    $exePath = "$ProjectRoot\target\$buildProfile\zenocast.exe"
    $p = Start-Process -FilePath $exePath -PassThru -NoNewWindow
    return $p
}

try {
    while ($true) {
        $currentHash = Get-SourceHash
        if ($currentHash -ne $lastHash) {
            if ($lastHash -ne "") {
                Write-Host ""
                Write-Host "[dev-watch] Source change detected -- rebuilding..." -ForegroundColor Yellow
            }

            # Kill previous instance
            if ($proc -and -not $proc.HasExited) {
                Write-Host "[dev-watch] Stopping previous instance (PID $($proc.Id))..." -ForegroundColor DarkGray
                Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
                Start-Sleep -Milliseconds 500
            }

            $proc = Start-App
            $lastHash = $currentHash
        }

        Start-Sleep -Seconds $PollSeconds
    }
} finally {
    if ($proc -and -not $proc.HasExited) {
        Write-Host ""
        Write-Host "[dev-watch] Shutting down Zenocast..." -ForegroundColor DarkGray
        Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    }
}
