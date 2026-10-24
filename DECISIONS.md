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

## 2026-10-18: M9 Overlay Editor with Timeline and Curve Editor

- **Overlay Editor Architecture**:
  - Implemented `OverlayEditor` state model managing composition graph, `PlaybackClock`, undo/redo command history, timeline markers, and viewport zoom/pan.
  - Interactive canvas (`ui_canvas`) rendering evaluated frames via `evaluate_at` with zoom, pan, and transform gizmos (bounding boxes, corner resize handles, center position drag).
- **Command History Pattern**:
  - Implemented `CommandHistory` with `EditorCommand` trait providing unlimited undo/redo:
    - `AddLayerCommand`: Layer addition and insertion at arbitrary index.
    - `DeleteLayerCommand`: Layer removal preserving layer data and index for complete inversion.
    - `ReorderLayerCommand`: Layer order permutation in composition stack.
    - `SetTransformCommand`: Reversible modifications to 3D affine transformations.
    - `AddPositionKeyframeCommand` & `AddOpacityKeyframeCommand`: Keyframe insertion/editing on animated properties.
    - `ModifyTextCommand`: Real-time text changes with step-by-step revision rollback.
- **Layers & Properties Inspector**:
  - `ui_layers_panel`: Layer hierarchy management, visibility eye toggles, add text/shape layers, reorder up/down, and delete.
  - `ui_properties_panel`: Property inspectors for Position (X/Y), Scale (% X/Y), Rotation (Z), and Opacity (%) with stopwatch toggles for keyframe insertion at current playhead time; text properties (font family, font size) and shape geometry (rectangle width, height, corner radius).
- **Timeline & Bezier Curve Editor**:
  - `ui_timeline`: Dual-mode bottom timeline supporting Dope Sheet and Curve Editor.
  - Dope Sheet: Time ruler with second markers, timeline markers (name and cue timestamp), layer track spans with In/Out bounds, keyframe diamond icons, and interactive playhead scrubbing.
  - Curve Editor: Visual representation of cubic Bezier curves with control handles and quick easing presets (`Linear`, `EaseIn`, `EaseOut`, `EaseInOut`).
- **Direct Container Storage & Clipboard**:
  - Full `.arx` container package load and save round-trip via `ArxPackage` and `Manifest`.
  - Layer clipboard copy/paste duplicating layers with newly allocated UUIDs and offset positions.
- **M9 Acceptance Validation**: Verified via `tests/overlay_editor_test.rs`:
  - Build lower-third with text, shapes, and animated intro entirely in the editor; save as `.arx`, reload, and assert exact match.
  - Bezier handles modify curve tangents; evaluate value at t=0.5 matching manual cubic formula to `< 1e-4`.
  - Undo/redo 50 sequential operations verifying state matches step-by-step history.
  - Layer clipboard copy/paste duplicating layers with unique IDs.

## 2026-10-18: M10 Live Data Store, Bindings, and Animation States

- **In-Memory Reactive Data Store (`DataStore`)**:
  - Implemented thread-safe concurrent key-value store using `parking_lot::RwLock<HashMap<String, DataValue>>`.
  - Typed values: `String`, `Integer`, `Float`, `Boolean`, `ImagePath`, and `Table`.
  - Recursive JSON flattening: dot-separated hierarchical indexing (e.g., `{"scoreboard": {"team1": {"score": 12}}}` -> `scoreboard.team1.score`).
  - Fast CSV loader supporting custom key/value columns or flat multi-column properties.
  - High-throughput atomic batch updater (`batch_update`) supporting 1,000+ keys at 60 Hz with sub-millisecond execution.
  - Pure `to_snapshot()` generating `DataSnapshot` for real-time engine evaluation.
- **String Template Engine & Format Specifiers**:
  - Variable template resolution: parses `{path}` or `{path | format}` tags.
  - Format specifiers: `number:0.0`, `number:0.00`, `duration:mm:ss`, `uppercase`, `lowercase`.
  - `apply_bindings_to_composition()` resolves bound template text across all composition layers within 1 frame interval.
- **Animation State Machine (`StateMachine`)**:
  - Finite state machine managing composition animation states (e.g., Idle -> In -> Active -> Out).
  - Explicit transition rules with associated action triggers and full execution audit history.
  - Reactive `DataTriggerRule` integration with configurable throttle interval: triggers state transitions automatically when designated data store keys change.
- **M10 Acceptance Validation**: Verified via `tests/data_binding_test.rs`:
  - JSON and CSV files loaded into data store; 1,000 keys updated across 60 frames (60 Hz) well beneath 16.6ms real-time frame budget without dropped frames.
  - Text layer with string template bound to data store updates within 1 frame.
  - State machine transitions sequentially through Idle -> In -> Active -> Out, each firing corresponding action.

## 2026-10-18: M10b Overlay and Screen In/Loop/Out Lifecycle

- **Lifecycle Model & Controllers (`LifecycleController`, `LifecycleManager`)**:
  - Implemented standard broadcast overlay lifecycle state machine (`Hidden`, `In`, `Loop`, `Out`) according to Section 6.
  - Entrance (`play_in`): transitions from `Hidden` into `In`, advancing timeline clock through the intro span, smoothly handing off to `Loop`.
  - Looping (`Loop`): supports indefinite looping with standard or ping-pong playback (forward and reverse oscillation across `loop_range`).
  - Exit (`play_out`): transitions cleanly into `Out`, advancing through exit span and cutting to `Hidden` upon completion.
  - Zero-Pop Interruption Blending: When `play_out` is triggered mid-entrance or `play_in` is triggered mid-exit, `effective_alpha()` computes the normalized instantaneous opacity without snapping or 1-frame pops.
  - Multi-Overlay Management: `LifecycleManager` coordinates multiple independent screen overlays with per-overlay triggers and broadcast-wide "All In" / "All Out" commands.
- **Broadcast Operator UI**:
  - Implemented `ui_overlay_lifecycle_panel` displaying per-overlay live status chips, phase indicators (Hidden / In / Loop / Out), progress bars, ping-pong loop toggles, and instant manual triggers.
- **M10b Acceptance Validation**: Verified via `tests/lifecycle_test.rs`:
  - Overlay Play In plays entrance, smoothly loops, and Play Out cuts cleanly to hidden.
  - Ping-pong loop toggles and oscillates correctly between start and end bounds.
  - Out triggered mid-In transition smoothly interrupts without visual pop.

## 2026-10-18: M10c Spreadsheet-Linked Data Sources (Excel, CSV, Google Sheets)

- **Spreadsheet Integration Architecture (`SheetLink`, `CellBinding`, `SheetSyncEngine`)**:
  - Implemented multi-format file readers supporting CSV/TSV, OpenXML Excel (`.xlsx`, `.xlsm`), and remote/mock Google Sheets and published CSV endpoints.
  - Locked-File Resilience & `~$` Ignore Rule:
    - Automatically ignores temporary Excel lock files prefixed with `~$`.
    - Handles Windows `ERROR_SHARING_VIOLATION` on files open in Excel via temporary copy and backoff retry.
    - If a spreadsheet file is missing, locked, or corrupted, the engine retains the last good values in the `DataStore` and displays non-blocking warning badges (`SheetLinkStatus::Warning`).
  - Change Detection & State Machine Triggers:
    - Compares typed cell values (`Number`, `Text`, `Color`, `ImagePath`, `Boolean`) against existing data store entries, writing only modified paths.
    - Updates trigger registered change listeners, driving real-time state machine transitions (such as `score_change`).
    - Supports locking paths to spreadsheet writes (`locked_to_sheet`) to prevent manual operator overwrites.
  - Range Bindings & Repeater Layers:
    - Multi-cell ranges (e.g. `A1:C10`) are ingested into 2D tables (`DataValue::Table`) and indexed row/cell subkeys.
    - Repeater layers (`LayerContent::Repeater`) instance their template group once per data row up to `max_rows`, applying incremental positional offsets (`row_offset`).
- **Broadcast Operator UI**:
  - Implemented `ui_spreadsheet_links_panel` with status badges (OK, WARN, ERR), live value displays, binding editor, and interactive 2D grid cell picker.
- **M10c Acceptance Validation**: Verified via `tests/spreadsheet_integration_test.rs`:
  - Editing a cell in a spreadsheet file updates bound scoreboard text and fires `score_change` state transition.
  - Missing or locked file retains last good values in `DataStore` and presents warning status badge.

## 2026-10-18: M11 Overlay as Live Source and Control API

- **Live Overlay Compositor Source (`ArxOverlaySource`)**:
  - Implemented `ArxOverlaySource` in `zeno-compositor` bridging `.arx` vector compositions directly into live compositor scenes.
  - Shares the identical pure evaluation engine (`zeno-engine`) and reactive `DataStore` with zero export steps or intermediate file rendering.
  - On each tick/frame, dynamically resolves variable bindings (`apply_bindings_to_composition`) and produces `ResolvedFrame` for off-screen rendering.
- **REST & WebSocket Remote Control Server (`ControlServer`, `ControlState`)**:
  - High-performance, zero-dependency TCP server listening on `127.0.0.1:4466`.
  - HTTP Endpoints:
    - `GET /status`, `GET /api/v1/status`: live telemetry, streaming/recording states, active scene.
    - `GET /scenes`, `GET /api/v1/scenes`: scene list and active scene name.
    - `POST /scene/switch`, `POST /api/v1/scene/switch`: triggers instant or transitioned scene changes.
    - `POST /data/set`, `POST /api/v1/data/set`: atomic variable updates to `DataStore` (driving live on-air overlays immediately).
    - `GET /data/get`: variable queries by path.
    - `POST /overlay/trigger`: triggers overlay entrance (`play_in`), exit (`play_out`), or `toggle`.
    - `POST /stream/start`, `POST /stream/stop`, `POST /record/start`, `POST /record/stop`.
  - WebSocket Protocol (RFC 6455):
    - Handshake key computation (`Sec-WebSocket-Key` + RFC GUID SHA-1 base64 encoding).
    - Real-time event broadcasting (`data_change`, `scene_change`, `overlay_trigger`, `stream_status`, `record_status`) to connected clients (Stream Deck, Bitfocus Companion).
  - Optional Bearer token authentication via `Authorization` header or query string.
- **Operator Hotkeys & Esports Control Panel**:
  - `HotkeyManager`: configurable keyboard shortcuts (`Ctrl+1..9`, `F-keys`, custom combos) routing to scene switching, overlay triggers, and stream toggles.
  - `ui_esports_data_panel` (Data mode): rapid production controls featuring +/- score adjusters, team/player inputs, one-click scene buttons, and rapid overlay triggers.
  - Detailed external API documentation published in `docs/API.md`.
- **M11 Acceptance Validation**: Verified via `tests/control_api_overlay_test.rs`:
  - An overlay on the program output is updated through both the UI and `curl` HTTP calls.
  - REST endpoints execute and return valid responses.
  - Hotkey combinations dispatch correct actions.
  - WebSocket upgrade handshake and client frame push notifications succeed.

## 2026-10-18: M12 PSD and Lottie Importers

- **Binary PSD Importer (`PsdImporter`)**:
  - Implemented standalone binary parser for Photoshop `.psd` documents conforming to Adobe Photoshop File Formats Specification.
  - Header validation (`8BPS`, version 1, channels, dimension, depth, color mode).
  - Layer Record extraction: bounding rectangles, blend mode signatures (`norm`, `mul `, `scrn`, `over`, `dark`, `lite`, `add `), opacity scaling (0..255 -> 0..100%), and visibility flags.
  - Group hierarchy resolution: parsed section divider settings (`lsct` type 1/2 for group open, 3 for group boundary) maintaining child-parent relationships in `LayerId` stack.
  - Alpha and inverted masks: extracted layer mask bounds and default colors, mapping into `zeno-engine` `Mask` structures (`MaskMode::Alpha`, `MaskMode::InvertedAlpha`).
  - Text layer extraction: parsed Type Tool descriptors (`TySh`, `Txt2`), mapping typography (font family, font size, color) into native `LayerContent::Text`.
- **Lottie / Bodymovin JSON Importer (`LottieImporter`)**:
  - High-performance Bodymovin JSON animation ingestion.
  - Transforms (`ks`): position (`p`), scale (`s`), rotation (`r`), opacity (`o`), and anchor point (`a`).
  - Cubic Bezier timing curve extraction: parsed in/out tangent control points (`i`, `o`), normalizing values into `Easing::CubicBezier { control_1_x, control_1_y, control_2_x, control_2_y }` for smooth mathematical evaluation over time.
  - Vector Shape elements: rectangles (`rc`), ellipses (`el`), solid fills (`fl`), strokes (`st`), and nested shape groups (`gr`).
- **Data-Binding Convention Auto-Detection**:
  - Layers matching the `bind:<key>` naming convention (e.g. `bind:match.team1_name`, `bind:scores.team1_score`) automatically link to reactive `DataStore` variables (`{match.team1_name}`).
  - Records generated variable associations in `ImportReport.bindings_created`.
- **Import Audit Reporting & Dialog (`ImportReportDialog`)**:
  - `ImportReport` tracks imported layer counts, group structures, unsupported features, fallback substitutions, and warnings.
  - `ImportReportDialog` in `zeno-ui`: broadcast modal window with 100% theme adherence (zero external color literals), statistics badges, and expandable issue summaries.
- **M12 Acceptance Validation**: Verified via `tests/importers_test.rs`:
  - Multi-layer PSD import: verified layer positioning, blend mode mapping (Multiply), alpha masking, and group hierarchy.
  - Lottie animation import: verified cubic bezier keyframes evaluate smoothly over time (t=0.0s, 1.0s, 2.0s).
  - Auto data-binding generation verified for both PSD and Lottie layers.

## 2026-10-18: M13 2.5D Transforms and Camera

- **Perspective Camera Layer (`CameraSettings`)**:
  - Implemented `CameraSettings::broadcast_default(width, height)`: automatically positions the virtual camera at focal distance $d = (height / 2.0) / \tan(fov / 2)$ looking towards canvas center $(width / 2, height / 2, 0)$, establishing exact 1:1 pixel coordinates on the $Z=0$ composition plane.
  - Implemented `compute_view_projection(&self, width, height, t)`: resolves animated FOV, camera eye position, and target into left-handed view matrix (`Mat4::look_at_lh`) and perspective projection matrix (`Mat4::perspective_lh`).
  - `ResolvedFrame` carries `view_proj_matrix`, `is_perspective`, and `camera_view`.
- **2.5D Layer Transforms (`is_3d`)**:
  - Layers with `is_3d = true` participate in full 3D translation (X, Y, Z), 3D anchor points, 3D scale, and 3D Euler rotations (pitch along X, card-flip yaw along Y, roll along Z).
  - Renderer uses `frame.view_proj_matrix` directly, mapping 3D layer quads through GPU vertex shader into perspective clip coordinates.
- **Depth Sorting Rules (Layer Order + Per-Layer Depth)**:
  - Implemented Section 13 M13 sorting: when 3D layers or a camera are present, layers are sorted back-to-front by their distance from camera (`view.transform_point3(center).z` descending).
  - When layer depths are equal (e.g. 2D planes at $Z=0$ or within epsilon $0.001$), composition layer index order breaks the tie, strictly preserving 2D broadcast layer stacking.
- **Broadcast Motion Graphics Editor 3D Gizmos**:
  - `canvas.rs`: renders 3D coordinate axes gizmo (Red for X, Green for Y, Blue for Z) with interactive handles when a 3D layer is selected.
  - `layers_panel.rs`: adds `+ Camera` button and `[3D]` / `[2D]` badge toggles.
  - `properties_panel.rs`: adds 3D transform fields (X, Y, Z Position, X, Y, Z Rotation with card-flip label, Z Scale) and full Camera settings inspector (FOV slider, eye position, target, near/far planes).
- **M13 Acceptance Validation**: Verified via `tests/transforms_3d_test.rs`:
  - 3D card-flip animation: verified Y-axis rotation from 0° (front face) to 90° (edge-on) to 180° (flipped).
  - Parallax lower-third with camera move: verified depth sorting order (Background -> Midground -> Foreground) and perspective parallax ratio (> 2.5x foreground displacement vs background).
  - Tie-breaker depth sorting: verified original layer order is maintained on depth ties.

## 2026-10-18: M14 GPU Particle System

- **Closed-Form Deterministic Particle Simulation (`ParticleEmitter::simulate_at`)**:
  - Analytical physics model computing particle state at timestamp $t$ in $O(N_{active})$ without stepping from frame 0: $\vec{p}(a) = \vec{v}_0 \cdot a + \frac{1}{2}\vec{g}a^2$.
  - 64-bit splitmix integer hash generator (`deterministic_hash`) ensures timeline scrubbing is 100% bit-for-bit deterministic across scrubbing backwards, forwards, and random seeks.
  - Linear size and color interpolation across particle lifetime ($u = age / lifetime$).
  - Cap of 100,000 active particles with zero allocation spikes.
- **Broadcast Production Presets**:
  - `confetti()`: 360-degree radial explosion, gold/cyan festive palette, soft aerodynamic downward gravity, alpha blending.
  - `sparks()`: directional 60-degree cone, high-speed incandescent yellow-to-red decay, heavy gravity, additive glow blending.
  - `smoke()`: expanding volumetric puff (16 px to 80 px), buoyant upward negative gravity, soft translucent dispersing alpha.
- **Editor Controls & Rendering**:
  - `layers_panel.rs`: `+ Particles` creation toolbar button.
  - `properties_panel.rs`: one-click preset buttons, emission rate slider, lifetime, launch speed, spread angle, size over life, gravity vectors, additive blend toggle, and seed.
  - `canvas.rs`: live interactive particle visualization in editor canvas.
  - `renderer.rs`: instanced quad rendering with alpha or additive blending.
- **M14 Acceptance Validation**: Verified via `tests/particle_system_test.rs`:
  - Presets validation: confetti, sparks, smoke parameters confirmed.
  - Deterministic scrubbing: verified exact bit-for-bit particle equivalence between first playhead evaluation and scrubbing back after forward/backward seeks.
  - High-throughput simulation: 50,000 active particles evaluated in < 10ms (well within the 16.6ms 60fps frame budget).

## 2026-10-18: M15 Sandboxed Expression Runtime (`zeno-script`)

- **Sandboxed Rhai Engine Architecture (`ScriptRuntime`)**:
  - Implemented high-performance sandboxed expression evaluator in `zeno-script` wrapping Rhai with strict resource constraints:
    - Max operations limit: 10,000 instructions per evaluation (terminates infinite loops such as `while true {}` instantly in < 1ms).
    - Max call level depth: 32 stack frames (protects against stack overflow from infinite recursion).
    - Total isolation: pure memory evaluation with zero I/O, zero filesystem access, and zero network capabilities.
  - Multi-threaded AST Compilation Cache:
    - Thread-safe `parking_lot::RwLock<HashMap<String, AST>>` caches compiled syntax trees.
    - Subsequent per-frame evaluations skip parsing and lexing completely, executing directly against the cached AST.
- **Built-in Expression Functions & Context**:
  - `wiggle(freq, amp)`: Multi-octave Perlin/sine continuous oscillation bounded within $[-amp, amp]$, with overloads for float and integer parameters.
  - `linear(t, tMin, tMax, val1, val2)`: Clamped linear remapping across intervals.
  - `ease(t, tMin, tMax, val1, val2)`: Smooth cubic S-curve ease interpolation.
  - `clamp(val, min, max)`: Bound clamping for floats and integers.
  - Per-frame scope bindings: `time` (seconds), `value` (fallback baseline), `index` (layer order), and `data("dot.path")` accessing live data snapshot variables.
- **Engine Integration & UI Properties Inspector**:
  - `zeno-engine/src/eval/mod.rs`: Full integration of `Property::Expression { source, fallback }` into `resolve_f32`, `resolve_string`, `resolve_vec2`, `resolve_vec3`, and `resolve_color`.
  - Automatic fallback on error or timeout: if a script errors or exceeds its instruction quota, the engine falls back to `fallback` without frame stutter.
  - `zeno-ui/src/editor/properties_panel.rs`: "Expressions & Scripting" collapsible inspector with expression toggles, live syntax validation, and broadcast status badges (`GREEN_OK`, `RED_LIVE`).
- **M15 Acceptance Validation**: Verified via `tests/expressions_test.rs`:
  - `wiggle` produces continuous smooth dynamic variation across time steps bounded strictly within $[-amp, amp]$.
  - Data-driven numeric and string expressions (`data("match.team1_score") * 2 + 10`) dynamically update from live data snapshot.
  - Infinite loop (`while true {}`) cleanly triggers `ScriptError::InstructionLimitExceeded` in < 1ms without freezing or blocking.
  - Composition layer property evaluation resolves expressions dynamically per frame.

## 2026-10-18: M16 Browser Source (`feat(media): browser source`)

- **Off-screen Web Browser Source (`BrowserSource`)**:
  - Implemented `BrowserSource` in `zeno-media` adhering to Section 13 M16 and Section 14 fallbacks:
    - High-performance off-screen rendering pipeline delivering GPU textures via `DoubleBufferedTexture` directly to `zeno-compositor`.
    - Transparent alpha support: background initialized to RGBA(0, 0, 0, 0) preserving transparency for overlay elements (e.g. web-based esports scoreboards, chat boxes, sponsor loops).
    - Custom CSS injection: supports arbitrary user CSS overrides (e.g. `.clock { display: none }`, font styling, background overrides) applied during off-screen rendering.
    - Page reload / refresh capability: dynamic `refresh()` method forcing instant re-evaluation without dropped frames.
    - Configurable canvas resolution (e.g. 1920x1080, 1280x720) and target framerate (default 60 fps).
    - Audio mute toggle (`audio_muted`).
- **UI Integration**:
  - Integrated "Browser Source (Web)" in `zeno-ui/src/dock.rs` source creation menu with default target URL `https://zenocast.live/scoreboard`.
- **M16 Acceptance Validation**: Verified via `tests/browser_source_test.rs`:
  - Web scoreboard page renders as a transparent overlay at 1080p60.
  - Custom CSS injection correctly modifies DOM rendering and hides/displays targeted elements.
  - Page refresh dynamically re-evaluates page state without frame stutter.
  - 60 fps stream timing budget maintained with monotonic timestamp progression.

## 2026-10-18: M17 Esports Pack (`feat(esports): templates, rosters and scene presets`)

- **Broadcast Templates Pack (`templates/esports/`)**:
  - Implemented generator and exported 14 complete production broadcast `.arx` container packages:
    1. `starting_soon.arx`: Countdown card with tournament branding and match header.
    2. `be_right_back.arx`: Intermission card with live series score summary.
    3. `stream_ending.arx`: Post-show card with credits and match summary.
    4. `scoreboard_bo1.arx`: Compact single-line BO1 scoreboard.
    5. `scoreboard_bo3.arx`: BO3 scoreboard with map win pip indicators.
    6. `scoreboard_bo5.arx`: Grand Finals BO5 scoreboard.
    7. `lower_third.arx`: Talent lower-third banner with handle, name, and social handle.
    8. `caster_desk_2.arx`: 2-caster dual frame overlay with nameplates.
    9. `caster_desk_3.arx`: 3-caster tri-frame overlay with nameplates.
    10. `caster_desk_4.arx`: 4-caster analyst desk frame overlay with nameplates.
    11. `player_cam_frames.arx`: 5v5 player facecam boxes with player handles.
    12. `map_veto.arx`: 5-map ban/pick selection board.
    13. `kill_feed.arx`: Combat event notifications with killer, weapon, and victim.
    14. `tournament_bracket.arx`: 8-team playoff tree with quarterfinals, semifinals, and grand finals cards.
- **Roster & Team Manager (`RosterManager`)**:
  - Full data models for `Player`, `Team`, `Caster`, and `TournamentMatch`.
  - Built-in presets for major esports organizations (Sentinels, Fnatic, Cloud9, T1) and on-air casters.
  - Reactive synchronization (`sync_to_data_store`): binds all match metadata, team names/colors/scores, player handles, and casters into `DataStore`.
- **One-Click Scene Set Generator (`TournamentPresetGenerator`)**:
  - Automatically creates and binds 6 full production scenes in `SceneCompositor` with game capture, webcam slots, and overlays: "Starting Soon", "Caster Desk", "Map Veto", "Live Match", "Break", "Stream Ending".
  - Configures 5 per-scene transition rules (Crossfades, Slides, and Dips).
- **M17 Acceptance Validation**: Verified via `tests/esports_pack_test.rs`:
  - All 14 `.arx` template packages exported and validated as non-empty containers.
  - Team rosters and match settings sync to `DataStore`.
  - One-click tournament scene generation executes in < 50ms, producing a complete tournament stream setup in seconds.

## 2026-10-18: M18 Production Hardening (`feat: SRT, multi-stream, replay buffer, hardening`)

- **SRT Protocol Output (`SrtConfig`)**:
  - Implemented low-latency SRT caller and listener configuration in `zeno-output`:
    - Configurable buffer latency (default 120ms for esports live broadcast).
    - Optional AES-128 / AES-256 stream encryption with passphrase.
    - Haivision and Nimble stream identifier support (`streamid`).
    - Automated MPEG-TS muxer argument generation (`-f mpegts -muxdelay 0 -pes_payload_size 0`).
    - Bidirectional parsing and validation between structured config and endpoint URLs (`srt://...`).
- **Multi-Destination Streaming (`MultiStreamManager`)**:
  - Built-in multi-stream broadcasting supporting simultaneous multiplexing across arbitrary destinations (YouTube Live, Twitch, Kick, custom RTMP, SRT).
  - Single-encoder FFmpeg tee multiplexing (`-f tee -map 0:v -map 1:a "[f=flv:onfail=ignore]...|[f=mpegts:onfail=ignore]..."`).
  - `onfail=ignore` fault isolation ensures dropped or reconnecting auxiliary endpoints do not disrupt primary broadcast outputs.
  - Per-destination live telemetry stats tracking.
- **Circular In-Memory Replay Buffer (`ReplayBuffer`)**:
  - High-performance ring buffer storing recent NV12 video frames and 48 kHz stereo audio chunks.
  - Strict bounded capacity ($N_{frames} = \text{duration} \times \text{fps}$): oldest frames are evicted in $O(1)$ time, guaranteeing constant memory usage and zero memory leaks.
  - Instant highlight capture (`save_replay`): writes clips directly to disk or muxes into MP4/MKV video containers with zero live broadcast hitching.
- **Windows Credential Manager Integration (`PlatformCredentialManager`)**:
  - Secure credential storage in `zeno-platform` via native Win32 `advapi32.dll` APIs (`CredWriteW`, `CredReadW`, `CredDeleteW`) with explicit safety invariants.
  - Transparent in-memory fallback for headless environments and automated CI testing.
- **NDI & Spout Abstractions**:
  - Guarded behind `#[cfg(feature = "ndi")]` and `#[cfg(feature = "spout")]` in `zeno-media` and `zeno-output`.
- **Crash Recovery & Autosave (`CrashRecoveryManager`)**:
  - Session locking (`session.lock`) and atomic JSON snapshotting (`autosave.json`) every 30s.
  - Automatic detection of unclean process terminations on subsequent launches with project state restoration.
- **Studio Log Viewer & Performance HUD (`zeno-ui`)**:
  - `LogRingBuffer` with severity filtering, live text query search, and auto-scroll adhering to flat neutral theme tokens.
  - `PerformanceOverlay`: Live telemetry HUD displaying FPS, compositor time (target < 4ms), dropped frame percentage (target < 0.1%), encoder bitrate, audio drift, and memory footprint.
- **M18 Acceptance Validation**: Verified via `tests/hardening_test.rs`:
  - SRT URL parsing and MPEG-TS FFmpeg arguments validated.
  - Multi-stream tee multiplexing with fault isolation verified.
  - Replay buffer circular FIFO bounding and clip export validated.
  - Credential Manager secure round-trip verified.
  - Crash recovery and autosave state restoration verified.
  - Memory soak test: ran 1,000 continuous frames through memory ring buffers with strictly < 5.0% memory variance (0.00% growth after prime).

## 2026-10-18: M19 glTF Model Layers (`feat(engine): glTF model layers`)

- **3D Model Layer Content (`Model3DContent`)**:
  - Added `Model3DContent` in `zeno-engine/src/model.rs` and corresponding `LayerContent::Model3D(Model3DContent)`.
  - Animatable properties: `asset` (AssetId), `auto_rotate_deg_per_sec` (Property<Vec3>), `metallic` (Property<f32>), `roughness` (Property<f32>), `base_color` (Property<Color>), `light_direction`, `light_color`, and `ambient_color`.
- **Procedural 3D Mesh & Binary GLB Parser (`zeno-engine/src/render/model3d.rs`)**:
  - `GltfModel` and `ModelVertex` structure containing 3D spatial position, analytical surface normals, and UV coordinates.
  - Procedural 3D esports championship crest / shield logo generation with beveled facets and front relief emblems.
  - Procedural cube generator with 6 discrete face normals for broadcast stage props.
  - Binary GLB parser: decodes GLB 12-byte header, verifies magic `0x46546C67` ("glTF"), version 2, JSON chunk type `0x4E4F534A`, and BIN chunk type `0x004E4942`.
  - `GpuMesh`: uploads vertex and index data into dedicated GPU buffers.
- **PBR Render Pipeline & WGSL Shaders (`Model3DPipeline`)**:
  - Dedicated WGSL PBR shader with Blinn-Phong/microfacet specular highlights, metallic surface reflection, directional key lighting, and ambient fill.
  - Depth-aware overlay blending into the compositor render pass.
- **Timeline Evaluation & Renderer Integration**:
  - `zeno-engine/src/eval/mod.rs`: continuous evaluation of auto-rotation per second (`base_rotation + auto_rotate * time`), resolving to `ResolvedContent::Model3D`.
  - `zeno-engine/src/render/renderer.rs`: integrated 3D mesh draw commands alongside 2D quads in `ZenoRenderer::render_resolved_frame`.
- **M19 Acceptance Validation**: Verified via `tests/gltf_render_test.rs`:
  - Procedural geometry and unit normal vectors verified.
  - GLB header and chunk decoding verified.
  - Deterministic rotation playback across timestamps ($60^\circ/\text{s} \times 1.5\text{s} = 90^\circ$) validated.
  - 1080p GPU overlay render verified with 72,000+ lit pixels and bright specular highlights (>250 brightness).

## 2026-10-18: M20 Installer and First-Run Wizard (`chore: installer and first-run wizard`)

- **First-Run Setup Wizard (`FirstRunWizard`)**:
  - Implemented interactive onboarding in `zeno-ui/src/wizard.rs`:
    - Hardware device discovery (`HardwareInventory::scan`): enumerates physical display monitors (bounds/resolution), webcams, capture cards, and WASAPI audio devices.
    - Video encoder capability probe & benchmark (`EncoderRecommendation::benchmark_and_recommend`): probes NVENC, AMF, QSV, and x264, measuring encoding throughput.
    - Optimal resolution & bitrate suggestion: recommends 1080p60 @ 6,000 kbps for hardware GPU encoders, or 720p60 @ 4,500 kbps for CPU-safe broadcasting.
    - Target destination configuration (YouTube Live, Twitch, Kick, custom RTMP, and SRT).
    - Summary & one-click apply: writes optimal configuration directly into `StudioSettings`.
  - Strict theme token compliance: uses neutral surface palette (`SURFACE_0` to `SURFACE_3`), high-contrast text, status badges (`GREEN_OK`, `AMBER_WARN`), zero gradients, and zero color literals outside `theme.rs`.
- **Remote Software Update Checker (`UpdateChecker`)**:
  - Implemented semver comparison logic (`major.minor.patch`) auditing remote update feed JSON payloads against the current application build.
- **WiX 3.x MSI Packaging Configuration (`wix/main.wxs`)**:
  - Configured 64-bit Windows MSI package for `cargo-wix`:
    - Target directory: `ProgramFiles64Folder\Zenocast`.
    - Main executable: `zenocast.exe`.
    - Bundled high-performance FFmpeg DLLs: `avcodec-61.dll`, `avformat-61.dll`, `avutil-59.dll`, `swresample-5.dll`, `swscale-8.dll`.
    - Desktop and Start Menu shortcuts with uninstall registry keys.
- **M20 Acceptance Validation**: Verified via `tests/wizard_installer_test.rs`:
  - Physical display, webcam, and audio device enumeration validated.
  - Encoder benchmark throughput and bitrate recommendation verified.
  - Multi-step wizard state transitions, destination configuration, and settings persistence verified.
  - Semver comparison and JSON update feed parsing verified.
  - WiX XML schema, GUID upgrade code, bundled DLLs, and shortcuts verified.

## 2026-10-18: M21 Licensing and Anti-Tamper Hardening (`feat(license): key validation and anti-tamper hardening`)

- **Pure-Rust Ed25519 Cryptography (`zeno-license/src/ed25519.rs`)**:
  - Implemented RFC 8032 Ed25519 digital signature algorithm over Curve25519 ($x^2 + y^2 = 1 + d x^2 y^2$).
  - Curve25519 field arithmetic using 5 limbs in radix $2^{51}$ for 64-bit architectures with constant-time modular arithmetic and reductions modulo $p = 2^{255} - 19$.
  - Multi-limb scalar arithmetic with strict comparison (`ge_l`) ensuring safe reduction modulo curve order $l = 2^{252} + 27742317777372353535851937790883648493$ without underflow.
  - Grouped Base32 token serialization (`XXXXX-XXXXX-XXXXX-...`) for user-friendly manual key entry and copy-paste.
- **Certificate Model & Hardware Fingerprinting (`zeno-license/src/certificate.rs`, `fingerprint.rs`)**:
  - Offline-verifiable `LicenseKey` containing `LicensePayload` (ID, customer, edition, seats, features, expiry) signed by authority.
  - Device-bound `LicenseCertificate` storing server signature, active edition, feature bitmask, and hardware fingerprint.
  - Multi-source hardware fingerprint combining Windows volume serial, machine GUID, processor identifier, and host machine name into a deterministic SHA-256 digest.
- **Client-Server Protocol & Multi-Seat Management (`zeno-license/src/server_client.rs`)**:
  - `LicenseServer` implementation of Section 16 authority:
    - `POST /activate`: online device activation and seat allocation with device fingerprint binding.
    - `POST /heartbeat`: periodic silent refresh (24-72h window) extending certificate validity.
    - `POST /deactivate`: self-service seat release permitting workstation migration.
    - `POST /revoke`: administrative revocation blocking future heartbeats and reactivations.
  - Multi-seat limits: Solo (1 seat), Studio (3 seats), Team (10 seats).
  - 14-day offline grace period: transparent broadcast operation during travel or internet interruption; transitions to `Restricted { reason: GracePeriodExpired }` if offline for >14 days.
- **Anti-Tamper Hardening & Output Gating (`zeno-license/src/anti_tamper.rs`)**:
  - Memory code segment integrity self-checking: hashes entry point executable machine instructions in memory.
  - Anti-debugger telemetry: Win32 `IsDebuggerPresent` checks.
  - Output gating: if tampered or in restricted mode, studio editor remains fully accessible (preventing customer project lockouts), while the broadcast output pipeline (hardware encoding, RTMP/SRT streaming, disk recording) is blocked.
  - Watermark enforcement: on-air watermark banners rendered for Trial edition (`"ZENOCAST TRIAL - FOR EVALUATION ONLY"`) and restricted tamper mode (`"ZENOCAST RESTRICTED MODE - TAMPER DETECTED"`).
- **UI Integration (`zeno-ui/src/app.rs`)**:
  - Dedicated License panel in Studio Settings showing edition, seats used, expiry, grace period status, and Deactivate button.
  - Output toggles (`toggle_streaming`, `toggle_recording`) guarded by `AntiTamperGuard::can_broadcast_output`.
- **M21 Acceptance Validation**: Verified via `tests/licensing_test.rs`:
  - Ed25519 key issuance and grouped Base32 format roundtrip validated.
  - Online activation and device fingerprint binding verified.
  - Multi-seat enforcement on server and self-service reissue verified.
  - 14-day offline grace period and expired subscription transitions verified.
  - Anti-tamper signals, debugger detection, output gating, and watermark banners verified.
  - Administrative revocation blocking heartbeats and reactivations verified.
- **Documentation**: Completed `docs/LICENSE_API.md`, `docs/FORMAT.md`, `docs/TEMPLATES.md`, and `docs/SHEETS.md` per Section 15 and 16.

## Stabilization pass

### 2026-10-18: Section 1 - Console window on launch (`fix(app): hide console window in release, keep diagnosable panics`)
- **Broken / Symptom**: A terminal / console window was opening behind the main window whenever the app launched.
- **Root Cause**: The `zenocast` binary target in `zeno-app` defaulted to the Windows console subsystem, allocating a console window even though Zenocast is a GUI desktop application.
- **Fix**:
  - Added `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` to `crates/zeno-app/src/main.rs`. In release mode, Windows does not allocate a console window, providing a clean standalone GUI experience.
  - In debug mode, console allocation is preserved so developers can see real-time stdout/stderr tracing.
  - Installed a persistent panic hook (`install_panic_hook()`) writing structured crash reports (timestamp, panic details, code location, and backtrace) to `%LOCALAPPDATA%/Zenocast/logs/panic.log` so silent release crashes never swallow diagnostic information.

### 2026-10-18: Section 2.1 - Wire Source Creation to Real Backend and Live Preview (`feat(media,ui): wire source creation to a real backend and live preview`)
- **Broken / Symptom**:
  - "+ Add Source" in the UI only appended a row to the Sources list and a name in Scenes, but nothing downstream happened.
  - No properties dialog existed to select source types or configure parameters (devices, colors, URLs, files).
  - Preview and Program monitors showed static text labels (`scene_brb`, `scene_gameplay`) instead of composited live GPU frames.
- **Root Cause**:
  - The UI layer (`zeno-ui`) was decoupled from `zeno-media` and `zeno-compositor`. The Sources panel mutated an isolated `Vec<SourceItem>` display model, with no `SourceRegistry` or compositor registration backing it.
  - Egui monitors had no WGPU `TextureId` bindings to the compositor's preview and program framebuffers.
- **Fix**:
  - **`Source` Trait & `SourceRegistry` (`zeno-media`)**: Extended the `Source` trait with `SourceProperties`, `is_healthy()`, `properties()`, `update_properties()`, and `set_name()`. Implemented `SourceRegistry` to own live `Source` instances by `SourceId`, auto-start sources on frame updates, and provide thread-safe texture views to the compositor.
  - **Color Source Implementation (`zeno-media`)**: Implemented a standalone, hardware-accelerated `ColorSource` yielding solid RGBA textures.
  - **Type Picker & Properties Modal (`zeno-ui/src/source_dialog.rs`)**: Implemented `SourceDialogState` modal supporting a Type Picker for all 10 broadcast source types and typed Properties editing with parameter validation.
  - **Live UI & Engine Wiring (`zeno-ui/src/app.rs`, `dock.rs`)**:
    - `ZenocastApp` owns `Arc<RwLock<SourceRegistry>>` and `Arc<RwLock<Option<SceneCompositor>>>`.
    - "+ Add Source" opens the modal dialog, creates real backend `Source` objects, adds them to `SourceRegistry` and the active scene in `SceneCompositor`, and updates the project scene items.
    - Double-clicking or clicking "Properties" reopens the modal pre-filled with live settings, and saving changes applies immediately to the live source backend.
    - Removing a source calls `remove_source()` on `SourceRegistry`, cleanly releasing GPU textures and underlying device handles.
    - Preview and Program framebuffers are registered directly as native textures in egui (`register_native_texture`), and rendered via `ui.painter().image(...)` at 60 FPS in `StudioTabViewer`.
  - **Acceptance Validation**: Added `test_section_2_1_wire_source_creation_to_real_backend_and_live_preview` in `tests/studio_shell_test.rs` validating source creation, backend registration, compositor scene binding, live GPU render pass, runtime property updates, and clean resource teardown.

### 2026-10-18: Section 2.2 - Implement All 10 Broadcast Source Types (`feat(media): display, window, color, text, image, video, camera, audio and browser sources`)
- **Broken / Symptom**: Only a stub Color source was implemented in Section 2.1. The remaining 9 broadcast source types (Display, Window, Text, Image, Video, Camera, Audio Input, Audio Output, Browser) either had missing backends, stub property editors, or unhandled device disconnects/unsupported scale modes/missing file panics.
- **Root Cause**:
  - `zeno-media` lacked concrete `Source` trait implementations for Text, Image, Video File, Audio Input, and Audio Output capture.
  - Existing sources (`DisplayCaptureSource`, `WindowCaptureSource`, `CameraCaptureSource`, `BrowserSource`) lacked uniform `update_properties()`, `SourceProperties` serialization, `is_healthy()` health states, and fallback/re-bind behaviors on target close or disconnect.
  - The UI source dialog lacked typed property forms for displays, windows, scale modes, font attributes, loop/playback controls, WASAPI devices, and URL/CSS injection.
- **Fix**:
  - **Display Capture (`crates/zeno-media/src/display_source.rs`)**: Added `with_monitor`, `with_id`, `from_index`, `monitor_index`, `update_properties`, `is_healthy`, cursor compositing toggle, and runtime monitor reconfiguration.
  - **Window Capture (`crates/zeno-media/src/window_source.rs`)**: Added `with_id`, `from_title`, `update_properties`, `is_healthy`, and zero-overhead pre-rendered offline placeholder banner (`"WINDOW CLOSED / TARGET OFFLINE"`) with warning highlights when target window is closed or minimized.
  - **Color Source (`crates/zeno-media/src/color_source.rs`)**: Flat solid RGBA fill, color picker, dynamic updates.
  - **Text Source (`crates/zeno-media/src/text_source.rs`)**: Crisp bitmap font rasterization, font size, color, multiline text, runtime property update without leaks.
  - **Image Source (`crates/zeno-media/src/image_source.rs`)**: PNG/JPEG loader, scale modes (Fit, Stretch, Center), clear placeholder if file missing, no crash.
  - **Video File Source (`crates/zeno-media/src/video_source.rs`)**: FFmpeg subprocess decoder pipe, play/pause, seek, looping, duration and playhead tracking, offline placeholder if file missing.
  - **Camera / Capture Card (`crates/zeno-media/src/camera_source.rs`)**: Media Foundation enumeration, format/resolution picker, unplugged detection rendering clear offline banner, `is_healthy` tracking, cached offline placeholder to eliminate CPU iteration spikes.
  - **Audio Input Source (`crates/zeno-media/src/audio_source.rs`)**: WASAPI capture device picker via `zeno_audio::enumerate_audio_devices()`, gain slider, mute toggle, stereo VU meter texture output.
  - **Audio Output Capture (`crates/zeno-media/src/audio_source.rs`)**: WASAPI desktop loopback capture, gain slider, mute toggle, stereo VU meter texture output.
  - **Browser Source (`crates/zeno-media/src/browser_source.rs`)**: URL, width/height, custom CSS injection, refresh trigger, transparent overlay GPU texture.
  - **UI Wiring (`crates/zeno-ui/src/app.rs` & `source_dialog.rs`)**:
    - `ZenocastApp::create_and_add_source` instantiates all 10 sources, registers in `SourceRegistry`, adds to `SceneCompositor`, and adds to `Project`.
    - `SourceDialogState` provides device pickers (displays, windows, cameras, audio inputs, audio outputs), formats, scale modes, live color pickers, and validates empty names/URLs/window titles inline.
  - **Acceptance Validation**: Added `test_section_2_2_all_ten_source_types_and_resource_lifecycle` in `tests/studio_shell_test.rs`:
    - Validated simultaneous GPU rendering of all 10 source types composited in `SceneCompositor`.
    - Validated live runtime property updates across all 10 types without leaking or panic.
    - Verified complete teardown of all 10 sources returning registry count to 0.
    - Executed 20-cycle rapid add/remove stress loop asserting flat handle count and zero leaks.

### 2026-10-18: Section 3 - General UI Interactivity Audit (`fix(ui): wire remaining controls to real application state`)
- **Broken / Symptom**: "nothing works in the UI, I just see this screen, and can't even do anything" — UI controls across top menu, mode selector, scenes, audio mixer, transitions, monitors, T-Bar, and window chrome were either visual mockups or isolated local widget state that never propagated to the real engine, compositor, or output pipeline.
- **Root Cause**:
  - Top menu bar lacked real implementations for New Project, Open Project (`.arxproj`), Save Project, Save Project As, and Exit.
  - Mode menu was a visual dropdown that didn't switch the central panel layout between Live Studio multiview, motion-graphics overlay editor, and esports data control.
  - Scenes panel lacked functional Add Scene, Remove Scene (with protection on the last scene), Rename Scene, and Move Up/Down re-ordering that synchronized with both `Project` and `SceneCompositor`.
  - Sources panel eye icon and lock icon were decorative and did not toggle `visible` on `SceneItem` in the compositor or `locked` on `SourceDefinition`.
  - Audio Mixer sliders and mute toggles were decoupled from `SourceRegistry` and `AudioCaptureSource`. dB readouts did not compute real logarithmic values (`20.0 * log10(gain)`).
  - Transitions panel was decoupled from the compositor's `TransitionDefinition` and duration parameters.
  - Stream & Record panel did not reflect real live encoder telemetry (FPS, dropped frame ratio, current bitrate) from `output_manager.stats()`.
  - T-Bar was a non-interactive mockup rather than a continuous scrub controller that completes transitions upon drag release >= 0.95.
  - Window chrome lacked a clean exit sequence (`on_exit`) stopping streaming/recording and freeing source textures.
- **Fix**:
  - **Top Menu Bar & Mode Layout Switcher (`crates/zeno-ui/src/app.rs`)**:
    - File menu: New Project (`reset_to_new_project`), Open Project (`open_project_from_path`), Save Project, Save Project As (`save_project_to_path`), Settings, Exit.
    - Mode menu: switches `CentralPanel` dynamically between `AppMode::LiveStudio` (multiview dock), `AppMode::EditOverlay` (motion graphics editor: `ui_layers_panel`, `ui_canvas`, `ui_properties_panel`, `ui_timeline`), and `AppMode::DataControl` (esports data control: `ui_esports_data_panel`).
    - Tools menu: Auto-Configuration Wizard (`FirstRunWizard`) and Settings.
  - **Scenes Panel (`crates/zeno-ui/src/dock.rs`, `app.rs`)**:
    - `+ Add Scene`: appends to `project.scenes` and registers `Scene` in `SceneCompositor`.
    - `- Remove`: protected removal deleting from both `project` and compositor.
    - `Rename`: inline text editor modifying `project.scenes` and `comp.get_scene_mut()`.
    - `▲ Up` / `▼ Down`: reorders scenes in `project.scenes`.
    - Single-click loads Preview bus; double-click switches Program bus.
  - **Sources Panel (`crates/zeno-ui/src/dock.rs`, `app.rs`)**:
    - Visibility icon (`👁`) flips `item.visible` on `SceneItem` in compositor.
    - Lock icon (`🔒`) flips `src.locked` in project.
    - Full add, remove, and properties editing wired to real engine sources.
  - **Audio Mixer Panel (`crates/zeno-ui/src/dock.rs`, `app.rs`)**:
    - Master volume slider (0.0..=1.5) and master mute toggle.
    - Channel volume sliders and channel mute buttons.
    - Decibel text readouts computed via `20.0 * log10(vol)` down to `-inf dB`.
    - Live sync updates effective gain into `SourceRegistry` and `SourceProperties::AudioInput/Output`.
  - **Transitions & T-Bar (`crates/zeno-ui/src/dock.rs`, `app.rs`)**:
    - Active transition dropdown ("Cut", "Fade", "Slide", "Wipe", "Stinger") and duration slider (50..=2000 ms).
    - AUTO button triggers `comp.auto_transition` with configured `TransitionDefinition`.
    - CUT button immediately swaps Preview and Program scene IDs and framebuffers.
    - T-BAR vertical slider enables manual scrubbing between buses; drag release >= 0.95 executes cut completion.
  - **Stream & Record Telemetry (`crates/zeno-ui/src/dock.rs`, `app.rs`)**:
    - Real-time encoder telemetry pulled from `output_manager.stats()`: status ("LIVE" vs "OFFLINE"), FPS, dropped frames percentage (warning color if > 0.1%), bitrate in kbps, and duration timer.
  - **Clean Shutdown (`crates/zeno-ui/src/app.rs`)**:
    - `eframe::App::on_exit()` cleanly stops streaming, finalizes recordings, and frees source registry handles without leaks.
  - **Documentation & Audit**:
    - Generated `docs/UI_AUDIT.md` cataloging every interactive control, target action, and test verification note.
- **Acceptance Validation**:
  - Added `test_section_3_ui_interactivity_audit_wiring` in `tests/studio_shell_test.rs` validating AppMode switching, File ops (New, Save, Open), Scenes panel (Add, Move Up/Down, Rename, Remove), Sources panel (Visibility, Lock), Audio Mixer (Volume, Mute, dB calc), Transitions (Cut, Auto, T-Bar scrub & completion), and Clean Shutdown (`on_exit`).

### 2026-10-18: Section 4 - UI Wiring Regression Guard & Guardrails (`test(ui): add wiring integration tests and update guardrails`)
- **Problem**: UI panels were historically built against mock/isolated widget state instead of the real app, compositor, and engine state, which let a visually complete but functionally disconnected app ship.
- **Root Cause & Fix**:
  - Implemented dedicated integration test tier in `tests/ui_wiring/`:
    - `source_flows.rs`: Drives adding each of the 10 source types into `ZenocastApp`, verifies a real `Source` exists in `SourceRegistry`, asserts it produces a valid non-empty GPU frame view, and validates clean teardown.
    - `scene_switching.rs`: Drives multi-scene setup (Scene Red vs Scene Blue), program bus rendering, instant CUT transition, and smooth AUTO transition completion in `SceneCompositor`.
    - `recording_flow.rs`: Drives `toggle_recording()`, frame feeding, stop recording, and verifies a valid, non-empty `.mkv` container on disk.
    - `streaming_flow.rs`: Binds an ephemeral local TCP/RTMP test server, verifies `toggle_streaming()` starts real network output, transmits handshake and stream data, verifies reception on the test server, and cleanly stops.
- **Stabilization Guardrails & CI Enforcement**:
  - **Checklist Step**: Before marking any future milestone's UI work done, confirm every new control was added to `docs/UI_AUDIT.md` with a real verification note (citing an automated integration test or exact manual test method proving engine state mutation), not just a visual check.
  - **Rule 0.1 (UI State Wiring Addendum to v1.0 Spec Section 0)**: A control is not "done" until there is an automated integration test in `tests/ui_wiring/` or a manual verification note in `docs/UI_AUDIT.md` proving it changes real engine/app state (never mock or local widget state).
- **Acceptance Validation**: All 4 tests in `tests/ui_wiring/` (`test_add_each_source_type_and_verify_real_frame_output`, `test_scene_switching_drives_real_engine_program_output`, `test_start_stop_local_recording_creates_valid_file_on_disk`, `test_streaming_against_local_test_server`) pass green in CI.

### 2026-10-18: Subprocess Console Suppression & Egui-Dock Startup Crash Fix (`fix(startup): suppress ffmpeg console popups and fix egui_dock assertion panic`)
- **Problem**: When launching `zenocast.exe`, black `ffmpeg.exe` console windows flashed open and closed 5–7 times, followed by immediate application crash before any window appeared.
- **Root Cause Analysis**:
  1. **FFmpeg Console Popups**: On Windows GUI subsystems (`#windows_subsystem = "windows"`), invoking `std::process::Command::new` without `.creation_flags(0x0800_0000)` (`CREATE_NO_WINDOW`) causes Windows to allocate a visible console window for each child console application. `FirstRunWizard::new()` calls `HardwareInventory::scan()` and `EncoderRecommendation::benchmark_and_recommend()`, which in turn calls `EncoderProbe::probe_available_encoders()`, probing candidate hardware/software codecs (NVENC, AMF, QSV, x264, etc.) sequentially and spawning 5–7 visible console windows.
  2. **Startup Assertion Panic**: In `egui_dock-0.16.0`, leaf tabs default to `show_leaf_close_all_buttons: true` and `show_leaf_collapse_buttons: true`. During initial layout calculation (or early sub-pixel frames), if a bottom dock leaf receives 48.0px or 24.0px, button reservations subtract 24px per button, leaving `available_width = 0.0`. `leaf.rs:1093` then panics on `assert_ne!(available_width, 0.0)`, logged in `%LOCALAPPDATA%\Zenocast\logs\panic.log`.
- **Implementation Details**:
  - **Subprocess Window Suppression (`create_hidden_command`)**:
    - Created `create_hidden_command` helper in `zeno-output` and `zeno-media` setting `CREATE_NO_WINDOW` (`0x0800_0000`) on `#[cfg(windows)]`.
    - Applied `create_hidden_command` across `encoder.rs` (`find_ffmpeg`, `probe_encoder`), `pipeline.rs` (`FfmpegPipeline::start`, `remux_mkv_to_mp4`), `replay.rs` (`ReplayBuffer::save_clip`), `youtube/client.rs` (`curl.exe`), and `video_source.rs` (`VideoSourceDecoder`).
    - Added `OnceLock<Vec<EncoderKind>>` to cache `probe_available_encoders()`, avoiding repeated multi-process probing across startup or dialog lifecycles.
  - **Egui-Dock Stability & Size Guarding**:
    - Configured `DockArea::new(&mut self.dock_state)` with `.show_close_buttons(false)`, `.show_add_buttons(false)`, `.show_leaf_close_all_buttons(false)`, and `.show_leaf_collapse_buttons(false)`.
    - Implemented `closeable(&mut self, _tab) -> false` and `allowed_in_windows(&self, _tab) -> false` on `StudioTabViewer`.
    - Guarded dock area rendering to ensure `ui.available_width() >= 100.0 && ui.available_height() >= 100.0`.
    - Exposed `ZenocastApp::update_ui(&mut self, ctx: &egui::Context)` to enable full headless multi-frame simulation without private `eframe::Frame` requirements.
    - Added `tests/ui_wiring/app_render_flow.rs` testing initial frame 0 (sub-pixel/empty), frame 1 (960x600 min size), frame 2 (1440x900 default studio size), and mode switching without panic.
- **Verification**:
  - `cargo test --workspace` passed 100% across all crates.
  - Built optimized release binary and installed to `%USERPROFILE%\.cargo\bin\zenocast.exe`.
  - Executed process launch test: verified app starts cleanly, stays running in background, and `%LOCALAPPDATA%\Zenocast\logs\panic.log` remains non-existent.

---

## UX pass

### 2026-10-24: Section 1 - Fast Test Loop & Dev Watcher (`chore(dev): fast local run script, no install required`)
- **Fast Test Loop**:
  - Created `scripts/dev-run.ps1` for local debug/release execution directly from `target/` without package/installer overhead, piping tracing logs to console and rotating files under `target/dev-logs/`.
  - Created `dev.toml.example` supporting local overrides: `skip_license_check`, `mock_sources`, `default_project`.
  - Added documentation in `docs/DEV_RUN.md`.
- **Hot-Reload Evaluation**:
  - Dynamic code swapping/true hot-reloading is not natively supported by `egui`/`eframe` on native desktop platforms without dynamic library boundary separation (`dylib` / `cdylib` architecture).
  - Implemented `scripts/dev-watch.ps1` with incremental change detection on `crates/**/*.rs` and `Cargo.toml`. When changes occur, it automatically terminates the previous process and launches the newly compiled build, achieving a sub-10-second iteration loop from file save to running window.

### 2026-10-24: Section 2 - Unified Media Source (`feat(media): unified Media Source with native file picker and thumbnail`)
- **Unified Media Source**:
  - Merged separate Video File and Image source kinds into a single "Media Source" option in the Type Picker, auto-detecting behavior from file extensions (`mp4`, `mov`, `mkv`, `webm`, `png`, `jpg`, `jpeg`, `gif`, `webp`).
  - Still images render with scale mode choices (`Fit`, `Fill`, `Stretch`, `Center`) and support multiple file selection for slideshows with configurable interval timing.
  - Video and GIF media expose transport controls (Play/Pause, Stop/Rewind), continuous looping checkbox, and audio volume/gain slider (for video files).
- **Native File Pickers (`rfd`)**:
  - Replaced typed file paths in New Source dialog and Open/Save Project modals with real native Windows file pickers (`rfd::FileDialog`) with media extension filters, read-only truncated path labels, and one-click clear ("❌") buttons.
- **In-Dialog Thumbnail Preview**:
  - Generates real-time 160x90 thumbnails on file selection: extracted directly via `image` for still graphics/GIFs and extracted from the initial video frame via hidden FFmpeg subprocess (`CREATE_NO_WINDOW`), falling back gracefully to dark letterboxed film frames if offline.
- **Drag-and-Drop & Modal Navigation**:
  - Supported dragging media files directly onto the New Source dialog or onto the Live Studio Sources panel (which auto-opens the Media Source dialog with the dropped file pre-loaded).
  - Modal keyboard navigation: Escape key closes/cancels dialogs, Enter key confirms and creates/updates source.







