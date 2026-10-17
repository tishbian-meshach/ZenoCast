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

## 2026-10-17: M3 Animation Evaluation and Deterministic Playback

- **Interpolate Trait**: Implemented `Interpolate` in `zeno-core` for scalar floats (`f32`, `f64`), vector math (`Vec2`, `Vec3`, `Vec4`), `Color` (linear sRGB interpolation), primitives (`u32`, `i32`, `bool`), and `String` (step interpolation).
- **Keyframe Interpolation & Easings**: Implemented `evaluate_keyframes` using binary search intervals and cubic bezier solver (Newton-Raphson with bisection fallback) supporting `Hold`, `Linear`, `CubicBezier`, and standard CSS / Penner curve presets (`ease`, `ease_in`, `ease_out`, `ease_in_out`, `quad_*`, `sine_*`).
- **Pure Evaluation Engine**:
  - Implemented `evaluate(composition, data_snapshot, time_seconds) -> ResolvedFrame` in `zeno-engine::eval` following Section 6 resolution order: Binding -> Expression -> Keyframes -> Constant.
  - Implemented `PlaybackClock` with scrubbing (`set_time`), step forward/backward (`step`, `step_frame`, `step_back_frame`), seamless looping over animation state boundaries (`AnimationState`), and play/pause state management.
  - Decoupled `ZenoRenderer` to consume pure `ResolvedFrame` via `render_resolved_frame` and `render_frame_to_image`.
- **Headless CLI Rendering**: Implemented `zeno-cli render --file <path> --time <t> --out <path>` supporting arbitrary `.arx` container packages and built-in broadcast lower-third animation templates.
- **Golden Image Acceptance**: Added `test_m3_animation_evaluation_goldens_at_three_timestamps` in `tests/golden_render_test.rs` capturing frames at $t=0.0$s (intro sliding in, low opacity), $t=1.5$s (full loop on-air state), and $t=3.0$s (shifted loop state), confirming visual motion progression and deterministic 0.0000 RMSE comparisons against saved goldens.

## 2026-10-17: M4 Capture Sources (Display, Window, Camera)

- **Source Abstraction**: Designed the `Source` trait in `zeno-media` with lock-free double buffering (`DoubleBufferedTexture`) providing $O(1)$ `get_frame()` queries yielding `Arc<wgpu::Texture>` and `Arc<wgpu::TextureView>` without blocking locks, heap reallocations, or CPU copy spikes.
- **Windows Platform Ingest**:
  - `DisplayCaptureSource`: Queries monitors using Win32 `EnumDisplayMonitors` / `GetMonitorInfoW` and captures monitor bounds via GDI/DIB transfers (`capture_display_raw`).
  - `WindowCaptureSource`: Enumerates top-level visible application windows via Win32 `EnumWindows` and captures client rectangles with mouse cursor overlay (`capture_window_raw`), with graceful fallback simulation pattern generation in headless environments.
  - `CameraCaptureSource`: Enumerates physical UVC webcams and PCIe capture cards using Media Foundation COM interfaces (`MFEnumDeviceSources`, `MF_DEVSOURCE_ATTRIBUTE_FRIENDLY_NAME`), providing fallback to a virtual broadcast camera test-pattern generator when physical capture devices are unavailable.
  - `DeviceManager`: Centralized device discovery with `poll_hotplug_events()` detecting hardware insertion/removal.
- **Real-Time Performance Guarantee**: Pre-allocated pixel and texture buffers eliminated per-frame memory allocation. Incremental dirty region updates reduced simulation rendering cost from ~56 ms down to ~3.2 ms per 60 fps frame pair, easily holding the 16.6 ms budget.
- **M4 Acceptance Validation**: Verified via `tests/capture_demo_test.rs` streaming 180 continuous 60 fps frames with monotonic timestamps and sub-millisecond CPU copy overhead, and standalone example `examples/capture_demo.rs`.

## 2026-10-17: M5 Scene Compositor and Transitions

- **Scene & Item Graph**: Implemented `Scene` and `SceneItem` in `zeno-compositor` supporting arbitrary source references (`SourceId`), nested sub-scenes (`SceneId`), 2D affine transforms (`ItemTransform` with anchor points), pixel crop insets (`Crop`), opacity, blend modes, and filter chains.
- **Preview & Program Buses**: Implemented `BusManager` with Studio Mode (independent Preview and Program buses with Cut, Auto, and T-bar mixer) and Direct Mode (instant on-air switching).
- **GPU Transition Pipeline**: Developed `CompositorGpuPipeline` with WGSL shaders (`ITEM_SHADER_WGSL` and `TRANSITION_SHADER_WGSL`) implementing GPU-accelerated Cut, Fade, DipToColor, Slide (Push/Cover in 4 directions), Wipe (Horizontal, Vertical, Radial with soft-edge feathering), Zoom, and Stinger/Overlay transitions.
- **Rule Resolution & Mid-Transition Interruption**:
  - Implemented Section 10A 3-step hierarchy: exact pair rule -> destination scene default -> global default.
  - Seamless interruption handling: switching mid-transition restarts smoothly from the current visual state with no visible jump.
- **M5 Acceptance Validation**: Validated via `tests/scene_compositor_test.rs` streaming 60 frames during a 1080p60 fade transition with an average frame render time of 3.63 ms (comfortably meeting the sub-4 ms GPU frame time budget), mid-transition interruption to Scene C, and manual T-bar fader mixing.

## 2026-10-17: M5b Transition System and Per-Scene Overrides

- **Comprehensive Transition Palette**: Verified all Section 10A transition kinds in `zeno-compositor`:
  - Instant Cut
  - Linear/eased Crossfade
  - Dip to Color (black, white, or brand accent)
  - Slide Push & Cover across all four compass directions (Left, Right, Up, Down)
  - Wipe (Horizontal, Vertical, and Radial with parameterized soft-edge feathering)
  - Zoom (Zoom In and Zoom Out)
  - Stinger transitions with configurable switch point percentage
  - Branded Overlay transitions (.arx lifecycle integration)
- **Seamless Interruption Architecture**: Implemented GPU texture snapshotting (`snapshot_program_to_intermediate_a`) upon mid-transition interruptions. When an operator triggers a switch while a transition is in-flight, the current blended program buffer is frozen into intermediate texture A and flagged with `from_snapshot = true`, guaranteeing continuous, zero-jump blending toward the new target.
- **Rule Resolution Hierarchy & Validation**:
  - Implemented and verified the 3-tier precedence: (1) Exact pair rule (`from_scene -> to_scene`) -> (2) Destination scene default -> (3) Global default.
  - Parameter validation clamps duration (0 to 5s) and switch point (0 to 1), with stinger alpha detection checks.
  - Live transition thumbnail previewing implemented via `render_transition_preview`.
- **M5b Acceptance Validation**: Verified via `tests/transition_system_test.rs` testing every transition kind at 1080p60 holding frame budget (< 10 ms avg frame time), mid-transition snapshot interruption, priority hierarchy overrides, and live preview rendering.


