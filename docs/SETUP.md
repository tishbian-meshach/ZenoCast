# Zenocast: Development Environment Setup

Zenocast targets **Windows 10/11 x64** using the Rust stable MSVC toolchain.

## Prerequisites

Run PowerShell as Administrator to install required dependencies:

```powershell
# Core build tools and runtimes
winget install --id Rustlang.Rustup -e
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
winget install --id Git.Git -e
winget install --id LLVM.LLVM -e
winget install --id Kitware.CMake -e
winget install --id Microsoft.EdgeWebView2Runtime -e

# Toolchain setup
rustup default stable
rustup component add clippy rustfmt

# FFmpeg development libraries via vcpkg
git clone https://github.com/microsoft/vcpkg $env:USERPROFILE\vcpkg
& $env:USERPROFILE\vcpkg\bootstrap-vcpkg.bat
& $env:USERPROFILE\vcpkg\vcpkg install ffmpeg[core,avcodec,avformat,avfilter,swscale,swresample,nvcodec,amf,qsv,x264,srt]:x64-windows
[Environment]::SetEnvironmentVariable("VCPKG_ROOT", "$env:USERPROFILE\vcpkg", "User")
[Environment]::SetEnvironmentVariable("FFMPEG_DIR", "$env:USERPROFILE\vcpkg\installed\x64-windows", "User")

# Additional cargo tools
cargo install cargo-nextest cargo-deny cargo-wix
```

## Building the Workspace

```powershell
# Build all workspace crates
cargo build --workspace

# Run formatting check
cargo fmt --all -- --check

# Run linter
cargo clippy --workspace --all-targets -- -D warnings

# Run all unit and integration tests
cargo test --workspace
```
