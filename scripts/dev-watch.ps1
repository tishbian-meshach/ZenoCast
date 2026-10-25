<#
.SYNOPSIS
    Watch for source changes with true in-place hot reload for Zenocast UI.

.DESCRIPTION
    Uses in-place dylib hot reloading via zeno-ui-hot:
    - Edits in crates/zeno-ui-hot trigger an incremental rebuild of only
      zeno-ui-hot.dll and swap inside the still-running window without state loss.
    - Edits in static engine crates (zeno-engine, zeno-compositor, etc.)
      fall back to full rebuild + relaunch, printing a clear explanation.
    - Release mode builds statically and uses standard rebuild+relaunch.

.PARAMETER Release
    Build with --release (static linking, hot-reload disabled).

.PARAMETER Clean
    Delete target/ before initial build.

.PARAMETER PollSeconds
    How often to check for source changes. Default: 1.
#>
param(
    [switch]$Release,
    [switch]$Clean,
    [int]$PollSeconds = 1
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ProjectRoot = (Resolve-Path "$PSScriptRoot\..").Path
$DevToml     = Join-Path $ProjectRoot "dev.toml"
$HotUiDir    = Join-Path $ProjectRoot "crates\zeno-ui-hot"

# --- Environment setup (same as dev-run and CI) ---
$env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Program Files\LLVM\bin;C:\ffmpeg\bin;$env:PATH"
$env:RUST_LOG = "info,zeno_ui=debug,zeno_ui_hot=debug,zeno_app=debug,wgpu=warn,naga=warn"
$env:RUST_BACKTRACE = "1"

# --- Parse dev.toml if present ---
$DevConfig = @{}
if (Test-Path $DevToml) {
    Write-Host "[dev-watch] Loading dev.toml..." -ForegroundColor DarkCyan
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

# Propagate dev.toml flags as environment variables
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
    Write-Host "[dev-watch] Cleaning target/..." -ForegroundColor Yellow
    $targetDir = Join-Path $ProjectRoot "target"
    if (Test-Path $targetDir) {
        Remove-Item -Recurse -Force $targetDir
    }
}

function Get-SourceSnapshot {
    $cratesDir = Join-Path $ProjectRoot "crates"
    $files = Get-ChildItem -Path $cratesDir -Recurse -Include "*.rs","Cargo.toml" -File
    $snapshot = @{}
    foreach ($f in $files) {
        $snapshot[$f.FullName] = $f.LastWriteTime.Ticks
    }
    return $snapshot
}

function Start-HostApp {
    $buildProfile = if ($Release) { "release" } else { "debug" }
    $env:PATH = "$env:USERPROFILE\.cargo\bin;C:\Program Files\LLVM\bin;C:\ffmpeg\bin;$env:PATH"

    Write-Host "[dev-watch] Building Zenocast host ($buildProfile)..." -ForegroundColor Cyan
    $buildStart = Get-Date
    Push-Location $ProjectRoot
    try {
        if (-not $Release) {
            # Build hot dylib first so latest exported symbols are present
            & cargo build -p zeno-ui-hot
            if ($LASTEXITCODE -ne 0) {
                Write-Host "[dev-watch] Hot dylib build FAILED. Waiting for next change..." -ForegroundColor Red
                return $null
            }
            # Remove any stale shadow libraries from previous runs
            Remove-Item (Join-Path $ProjectRoot "target\debug\zeno_ui_hot-hot-*.dll") -Force -ErrorAction SilentlyContinue
        }

        $buildArgs = @("build", "-p", "zeno-app")
        if ($Release) {
            $buildArgs += "--release"
        } else {
            $buildArgs += @("--features", "hot-reload")
        }

        & cargo @buildArgs
        if ($LASTEXITCODE -ne 0) {
            Write-Host "[dev-watch] Host build FAILED. Waiting for next change..." -ForegroundColor Red
            return $null
        }
    } finally {
        Pop-Location
    }
    $elapsed = [math]::Round(((Get-Date) - $buildStart).TotalSeconds, 1)
    Write-Host "[dev-watch] Host built in ${elapsed}s -- launching..." -ForegroundColor Green

    $exePath = "$ProjectRoot\target\$buildProfile\zenocast.exe"
    $p = Start-Process -FilePath $exePath -WorkingDirectory $ProjectRoot -PassThru -NoNewWindow
    return $p
}


function Build-HotDylib {
    Write-Host "[dev-watch] UI change detected in zeno-ui-hot -- rebuilding hot dylib..." -ForegroundColor Cyan
    $buildStart = Get-Date
    Push-Location $ProjectRoot
    $buildExitCode = 0
    try {
        & cargo build -p zeno-ui-hot
        $buildExitCode = $LASTEXITCODE
    } finally {
        Pop-Location
    }

    $elapsed = [math]::Round(((Get-Date) - $buildStart).TotalSeconds, 1)
    if ($buildExitCode -eq 0) {
        Write-Host "[dev-watch] Hot dylib rebuilt in ${elapsed}s -- swapped in-place." -ForegroundColor Green
        return $true
    } else {
        Write-Host "[dev-watch] Hot dylib build failed. App running on last valid state." -ForegroundColor Red
        return $false
    }
}

Write-Host ""
Write-Host "================================================================" -ForegroundColor Magenta
Write-Host "  Zenocast True In-Place Hot Reload Watcher" -ForegroundColor Magenta
if ($Release) {
    Write-Host "  Mode: RELEASE (static linking, relaunch on change)" -ForegroundColor Yellow
} else {
    Write-Host "  Mode: DEBUG (zeno-ui-hot dylib hot reload active)" -ForegroundColor Green
}
Write-Host "  Polling every ${PollSeconds}s. Press Ctrl+R to reload/relaunch. Press Ctrl+C to stop." -ForegroundColor DarkGray
Write-Host "================================================================" -ForegroundColor Magenta
Write-Host ""

$lastSnapshot = Get-SourceSnapshot
$proc = Start-HostApp

try {
    while ($true) {
        $manualReload = $false

        # Responsive sleep loop with console key detection (every 100ms)
        $subIntervals = [math]::Max(1, [int]($PollSeconds * 10))
        for ($step = 0; $step -lt $subIntervals; $step++) {
            try {
                if ([System.Console]::KeyAvailable) {
                    $key = [System.Console]::ReadKey($true)
                    $isCtrlR = ($key.Key -eq [System.ConsoleKey]::R -and ($key.Modifiers -band [System.ConsoleModifiers]::Control))
                    $isPlainR = ($key.Key -eq [System.ConsoleKey]::R -and ($key.Modifiers -eq 0))
                    if ($isCtrlR -or $isPlainR) {
                        $manualReload = $true
                        break
                    }
                }
            } catch {
                # Non-interactive / redirected console
            }
            Start-Sleep -Milliseconds 100
        }

        # 1. Manual reload requested via console keypress
        if ($manualReload) {
            Write-Host ""
            Write-Host "[dev-watch] Manual reload requested (Ctrl+R) -- rebuilding and relaunching..." -ForegroundColor Yellow
            if ($proc -and -not $proc.HasExited) {
                Write-Host "[dev-watch] Stopping previous host instance (PID $($proc.Id))..." -ForegroundColor DarkGray
                Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
                Start-Sleep -Milliseconds 500
            }
            $proc = Start-HostApp
            $lastSnapshot = Get-SourceSnapshot
            continue
        }

        # 2. Host process exited on its own (or via Ctrl+R in app window)
        if ($null -ne $proc -and $proc.HasExited) {
            Write-Host ""
            Write-Host "[dev-watch] Host process exited, relaunching..." -ForegroundColor Yellow
            $proc = Start-HostApp
            $lastSnapshot = Get-SourceSnapshot
            continue
        }

        # 3. File change detection
        $currentSnapshot = Get-SourceSnapshot
        $changedFiles = @()

        foreach ($file in $currentSnapshot.Keys) {
            if (-not $lastSnapshot.ContainsKey($file) -or $lastSnapshot[$file] -ne $currentSnapshot[$file]) {
                $changedFiles += $file
            }
        }
        foreach ($file in $lastSnapshot.Keys) {
            if (-not $currentSnapshot.ContainsKey($file)) {
                $changedFiles += $file
            }
        }

        if ($changedFiles.Count -gt 0) {
            $isUiOnly = (-not $Release)
            foreach ($file in $changedFiles) {
                if (-not $file.StartsWith($HotUiDir, [System.StringComparison]::OrdinalIgnoreCase)) {
                    $isUiOnly = $false
                    break
                }
            }

            if ($isUiOnly -and $proc -and -not $proc.HasExited) {
                # Hot reload in place -- do not kill the host process
                $null = Build-HotDylib
            } else {
                if ($isUiOnly -and ($null -eq $proc -or $proc.HasExited)) {
                    Write-Host "[dev-watch] Host process exited, relaunching..." -ForegroundColor Yellow
                } else {
                    Write-Host "[dev-watch] Engine/static crate code changed, hot reload unavailable, relaunching..." -ForegroundColor Yellow
                }

                if ($proc -and -not $proc.HasExited) {
                    Write-Host "[dev-watch] Stopping previous host instance (PID $($proc.Id))..." -ForegroundColor DarkGray
                    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
                    Start-Sleep -Milliseconds 500
                }

                $proc = Start-HostApp
            }

            $lastSnapshot = $currentSnapshot
        }
    }
} finally {
    if ($proc -and -not $proc.HasExited) {
        Write-Host ""
        Write-Host "[dev-watch] Shutting down Zenocast..." -ForegroundColor DarkGray
        Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    }
}

