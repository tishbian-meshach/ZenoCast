# Zenocast Architectural Decisions (DECISIONS.md)

This log records major architectural and implementation choices, defaults selected when requirements permit flexibility, and fallbacks invoked per Section 14.

---

## 2026-10-17: M0 Workspace and CI Architecture Setup

- **Workspace Layout**: Initialized standard multi-crate Cargo workspace containing 16 crates partitioned strictly by responsibility:
  - `zeno-core`: Foundation math, Property<T>, time, identifiers, common errors.
  - `zeno-engine`: Pure scene graph evaluation, keyframes, render commands (zero UI, zero IO).
  - `zeno-format`: `.arx` container serialization, migrations, format validation.
  - `zeno-import`: PSD and Lottie asset converters.
  - `zeno-script`: Sandboxed expression evaluator (Rhai).
  - `zeno-data`: Live observable JSON store, spreadsheet and manual bindings.
  - `zeno-media`: Camera, screen capture, video, audio input sources.
  - `zeno-audio`: Audio mixing, WASAPI loopback, audio DSP pipeline.
  - `zeno-compositor`: Live multi-scene GPU compositor, preview/program buses, transitions.
  - `zeno-output`: Hardware video encoding (NVENC/AMF/QSV/x264), RTMP/SRT streaming, recording.
  - `zeno-control`: HTTP/WebSocket remote control server and global hotkeys.
  - `zeno-platform`: Isolated Windows-specific Win32/COM/GraphicsCapture unsafe calls with safety invariants.
  - `zeno-license`: Offline Ed25519 license validation and device integrity verification.
  - `zeno-ui`: Egui/eframe broadcast interface, timeline, curve editor, docking panels.
  - `zeno-app`: Main desktop application binary.
  - `zeno-cli`: Headless CLI renderer and validator.
- **Dependency Rule**: `zeno-engine` is strictly isolated from UI, capture, and output crates to ensure deterministic golden tests and shared headless CLI rendering.
- **Unsafe Code Policy**: `unsafe` is restricted strictly to `zeno-platform` and external FFI glue, with mandatory `// SAFETY:` explanations on every block.
- **Toolchain & Linker**: Primary target is `x86_64-pc-windows-gnu` / MinGW-w64 on developer environments where MSVC Build Tools interactive UAC elevation is not available, with CI workflows verifying against `windows-latest`.

## 2026-10-17: M2 WGPU Renderer with Shapes, Images, and Text

- **Render Pipeline**: Implemented a standalone, zero-UI, zero-IO `ZenoRenderer` in `zeno-engine` backed by `wgpu` 24.0 (DX12/Vulkan):
  - Orthographic camera projection mapped to composition dimensions.
  - Transform hierarchy resolution traversal: builds absolute world matrices (`glam::Mat4`) and net opacity across parent-child layer DAGs.
  - Unit-quad instanced rendering pipeline with WGSL shaders (`shaders.wgsl`) supporting solid fills, textures, linear gradients, and layer opacity modulation.
  - Recyclable `TexturePool` implementing pooled textures to eliminate per-frame GPU texture reallocations.
  - `RasterCache` with bitmap text rendering and content-hash caching to eliminate per-frame re-rasterization.
- **Golden Image Acceptance**: Established deterministic headless 1080p golden image testing in `tests/golden_render_test.rs` producing `tests/golden/m2_sample_1080p.png` with 0 pixel difference on subsequent runs.
- **Windows GNU Toolchain Linkage**: Resolved a DEP violation (0xc0000005) caused by GNU `ld` failing to generate proper PE import directory descriptors for short-import objects produced by `windows-targets`/`raw-dylib` by configuring LLVM's `ld.lld` (LLD 23.1.3) and `llvm-dlltool` in the Rust self-contained environment.

