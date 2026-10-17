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

## 2026-10-17: M6 Audio Mixer, Meters, Filters, and Clock Drift Correction

- **Core Audio Architecture**: Implemented full broadcast-grade audio engine in `zeno-audio`:
  - Standardized internal master mixing rate to 48 kHz float (`f32` stereo interleaved).
  - 6-track audio routing per source, with master channel summing, per-source volume/gain, and mute controls.
  - Multi-threaded lock-free ring buffering between capture threads and the real-time render/mix pipeline.
- **DSP Filter Pipeline**:
  - `AudioFilter` trait enabling arbitrary per-source and master DSP chaining.
  - `GainFilter`: Smooth decibel-to-linear amplification.
  - `NoiseGateFilter`: Attack/release smoothing envelope with configurable threshold and attenuation ratio.
  - `CompressorFilter`: Peak/RMS feedforward gain reduction with adjustable threshold, ratio, attack, release, and makeup gain.
  - `LimiterFilter`: Lookahead brickwall limiter with fast attack and peak clamp guaranteeing signal never exceeds digital ceiling (e.g. -0.1 dBFS).
  - `EqualizerFilter`: 3-band parametric/peaking biquad IIR filter (Low 100 Hz, Mid 1 kHz, High 8 kHz) with bilinear transform coefficient derivation.
- **Meters and Ballistics**:
  - `AudioMeter` providing sample-accurate True-Peak (dBFS), RMS (dBFS), and ITU-R BS.1770 short-term LUFS loudness tracking.
  - IEC 60268-18 PPM ballistics with instantaneous attack and 1.7s decay release.
- **Rubato Resampling**:
  - Integrated `rubato` FastFixedIn resampler converting non-48kHz inputs (e.g., 44.1 kHz, 96 kHz) to 48 kHz stereo on ingest.
- **Clock Drift Correction**:
  - `DriftCorrector`: Monitors incoming audio timestamp progression against the master broadcast video clock.
  - Corrects clock skew (e.g. ±40 ppm hardware oscillator drift) using zero-overhead micro-resampling (interpolating or dropping one sample pair at ±5 ms skew bounds).
  - Keeps cumulative audio-video drift bounded strictly below 5 ms (well beneath the 20 ms tolerance over 30+ minutes of continuous streaming).
- **WASAPI Device Ingest**:
  - Implemented `enumerate_audio_devices()` via `cpal` to discover microphones, line inputs, and default loopback audio endpoints.
- **M6 Acceptance Validation**: Verified via `tests/audio_mixer_test.rs`:
  - Multi-source mix test with live peak and RMS meters.
  - 30-minute soak simulation (90,000 chunks) with +40 ppm skew verifying clock drift remains under 5 ms (< 20 ms budget requirement).

## 2026-10-17: M7 Hardware Video Encoding, RTMP Streaming, and Recording

- **Encoder Abstraction & Probing**:
  - Implemented `EncoderKind` supporting `NvencH264`, `AmfH264`, `QsvH264`, `X264`, HEVC variants (`NvencHevc`, `AmfHevc`, `QsvHevc`, `X265`), and AV1 variants (`NvencAv1`, `AmfAv1`, `QsvAv1`).
  - `EncoderProbe::auto_select_best_h264()` queries host hardware capabilities and auto-selects the optimal available hardware encoder (NVENC -> AMF -> QSV), gracefully falling back to `libx264` with a diagnostic notice per Section 14.
  - Configurable rate control (`Cbr`, `Vbr`, `Cqp`), keyframe GOP interval, bitrate, presets, and profile.
- **GPU RGBA to NV12 Color Conversion**:
  - Standard ITU-R BT.709 integer SIMD and WGSL compute shader (`RGBA_TO_NV12_WGSL`) mapping 32-bit RGBA frames to planar NV12 (Y + interleaved UV).
  - Pre-allocated `Nv12Buffer` reusable cache guarantees zero per-frame heap allocations during the real-time render loop (Rule 5).
- **FFmpeg Subprocess Muxing Pipeline**:
  - Subprocess architecture bypassing C-header/CRT ABI collisions while accessing full host hardware encoding features.
  - Dual-input TCP loopback: video stream on input 0, 48 kHz stereo float audio on input 1.
  - Injected probe frame handshake: dispatches an initial blank NV12 frame upon connection to instantly satisfy FFmpeg container format detection, eliminating multi-input socket deadlocks.
  - Standard I/O handles configured to `Stdio::null()` to prevent Windows OS pipe buffer deadlocks.
- **Real-Time Output Manager (`OutputManager`)**:
  - Dedicated concurrent video and audio worker threads reading from bounded crossbeam ring buffers (`VIDEO_QUEUE_CAPACITY`, `AUDIO_QUEUE_CAPACITY`).
  - Guarantees video and audio streams progress concurrently with zero cross-thread blocking.
  - Automatic frame shedding during network/encoder congestion to maintain real-time presentation constraints.
  - Auto-reconnect with exponential backoff on stream disruption.
  - Crash-resilient MKV recording with instant zero-reencode remuxing to MP4 on stop (`-c copy`).
- **Live Output Statistics**:
  - `LiveOutputStats` tracks total frames, dropped frames, dropped ratio/percentage, instantaneous bitrate, encode latency, CPU/GPU usage estimates, and duration.
- **M7 Acceptance Validation**: Verified via `tests/output_pipeline_test.rs`:
  - 10-minute continuous streaming budget simulation asserting dropped frame ratio remains strictly under 0.1% (< 0.001).
  - Synchronized video and 48 kHz stereo audio recording to MKV and lossless remux to MP4.
  - RTMP stream queueing with zero dropped frames.

## 2026-10-17: M7b Direct YouTube Live Streaming Integration

- **OAuth 2.0 PKCE Flow & Credential Storage**:
  - Implemented RFC 7636 PKCE code verifier and SHA-256 code challenge generation using pure Rust dependencies (`sha2` and `base64`).
  - Added ephemeral `LoopbackAuthListener` for catching OAuth redirects on local TCP ports.
  - `TokenStorage` provides in-memory caching and Windows Credential Manager integration for persistent refresh tokens.
  - Setup guide documented in `docs/YOUTUBE.md` detailing Google Cloud Console setup, OAuth consent, and credential storage.
- **Preflight Verification & Bitrate Guidance**:
  - `YouTubeBitrateTable` encodes official recommended bitrate ranges across resolutions (4K, 1440p, 1080p, 720p) and frame rates (30 fps vs 60 fps).
  - `PreflightValidator` provides pre-flight checks: signs-in verification, channel live streaming activation, 48 kHz stereo audio enforcement, GOP keyframe duration (1.0s to 4.0s, recommended 2.0s), and bitrate range warnings.
- **YouTube Live Streaming API v3 Client**:
  - Windows Schannel HTTP execution via built-in `curl.exe` avoiding C-header CRT compilation issues.
  - Specialized error parsing with actionable diagnostic messages:
    - HTTP 401 / `invalid_grant`: "YouTube authentication token is expired or revoked. Please sign in again via Settings > YouTube."
    - `liveStreamingNotEnabled`: "YouTube channel is not enabled for live streaming. Activation can take up to 24 hours. Visit https://www.youtube.com/live_dashboard to check status."
    - `quotaExceeded`: "YouTube API quota exceeded for the current 24-hour period. Please retry tomorrow or use a manual stream key."
- **Broadcast Lifecycle & Health Monitoring**:
  - `YouTubeBroadcastManager` implements one-click `go_live`: validates preflight -> creates broadcast (`liveBroadcasts.insert`) -> creates live stream (`liveStreams.insert`) -> binds broadcast to stream (`liveBroadcasts.bind`) -> hands ingestion target to `OutputManager` -> transitions lifecycle (`testing` -> `live`).
  - Starts 5-second background health poller tracking viewer counts and ingestion health without exceeding daily quota.
  - `end_broadcast` halts live streaming, transitions broadcast state to `complete`, and cleanly shuts down background poller.
  - Manual stream key fallback allows operators to bypass OAuth API and stream directly to YouTube RTMP.
- **M7b Acceptance Validation**: Verified via `tests/youtube_integration_test.rs`:
  - One-click Go Live lifecycle from signed-in state transitioning through testing to live and ending cleanly with broadcast `complete`.
  - Revoked/expired token returning readable diagnostic error.
  - Un-activated channel returning 24-hour verification error.
  - API quota exceeded returning diagnostic fallback hint.
  - Manual stream key fallback bypassing OAuth and streaming directly.
  - PKCE code verifier and SHA-256 S256 challenge generation.

## 2026-10-17: M8 App Shell and UI Foundation

- **Visual Design & Token Strictness**:
  - Implemented broadcast design token palette in `zeno-ui/src/theme.rs` according to Section 7:
    - Neutral surfaces: `SURFACE_0` (`#0E0F11`), `SURFACE_1` (`#15171A`), `SURFACE_2` (`#1D2024`), `SURFACE_3` (`#272B30`).
    - Typography: `TEXT` (`#E7E9EC`), `TEXT_MUTED` (`#8E959D`).
    - Semantic indicators: `ACCENT` (`#4C8DFF`), `RED_LIVE` (`#E5484D`), `AMBER_WARN` (`#F5A524`).
    - Layout: 8px spatial grid, 1px solid borders, 4px corner radii, 28px list row heights, 32px button heights.
  - Zero-gradient and no-color-literals policy enforced via static unit test scanner in `tests::test_strict_theme_and_no_color_literals_outside_theme`.
- **Dockable Studio Layout (`egui_dock`)**:
  - Implemented `create_default_live_layout()` providing dockable panels across 6 primary studio views:
    - `PreviewProgramTab`: Dual preview and program broadcast monitors with centered studio T-Bar slider and Quick Cut / Auto transition triggers.
    - `ScenesTab`: Scene list with Add, Duplicate, Remove, Rename, click-to-preview, and double-click-to-live.
    - `SourcesTab`: Scene item hierarchy supporting Display Capture, Window Capture, Camera Feed, and ARX Overlay sources with visibility toggling and transform locks.
    - `MixerTab`: 6-track audio mixer featuring master and per-source faders with logarithmic decibel scaling (-60 to +12 dB), mute toggling, and live visual meters.
    - `TransitionsTab`: Selector for Cut, Fade, Slide, Wipe, and Stinger transitions with adjustable duration (50ms - 2000ms).
    - `StreamControlsTab`: Live streaming toggle, crash-resilient recording toggle, and real-time statistics readout (FPS, dropped frame ratio, bitrate, duration).
- **Project Model, 30s Autosave & Crash Recovery**:
  - Project file serialization (`.arxproj`) preserving scene hierarchy, source settings, and output configurations.
  - `Autosaver` tracking 30-second intervals to write `.autosave.arxproj` crash markers, with automatic detection and restoration on startup.
- **Settings Dialog**:
  - Studio modal controlling video output resolution/FPS, audio sampling, hardware encoder selection, RTMP/YouTube destinations, and operator hotkeys.
- **M8 Acceptance Validation**: Verified via `tests/studio_shell_test.rs`:
  - Scene creation and source addition across all supported source types.
  - Cut & Auto transition switching between preview and program buses.
  - One-click Live streaming and recording toggles.
  - Full project serialization, deserialization, and 30s autosave crash recovery.
  - Strict broadcast UI token compliance.


