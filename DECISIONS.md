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
