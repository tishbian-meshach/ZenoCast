# Zenocast: System Specification (v1.0 + v1.5 Additions)

This document specifies the architecture, data models, rendering pipelines, audio processing chains, and milestones for Zenocast: a real-time esports broadcast studio and motion-graphics editor.

---

## 1. Scope and Architecture Overview

Zenocast is built as a single application uniting live broadcast production (OBS-style camera switching, screen capture, dual-bus transitions, and multi-channel audio mixing) with a native motion-graphics authoring engine (`.arx`).

### 1.1 Core Principles
1. **Zero-Copy Performance Guarantee**: Live rendering operates at 1080p60 with hardware texture sharing, zero per-frame heap allocations on the hot path, and monotonic nanosecond clock synchronization.
2. **Unified Canvas**: Graphics edited in the overlay editor appear on-air without an intermediate export or render step.
3. **Clean-Room Reference Isolation**: All reverse-engineering findings (from OBS Studio 32.2.2 analysis) serve solely as behavioral evidence. Data models, struct layouts, and Rust implementations are authored independently with no proprietary or GPL-encumbered code.

---

## 2. Workspace and Subsystem Structure

The codebase is organized into isolated Cargo crates:
- `zeno-core`: Fundamental primitives, properties (`Property<T>`), time, colors, affine transforms, filter data models, and identifiers (`SourceId`, `SceneId`, `FilterId`).
- `zeno-engine`: Pure scene graph evaluation, keyframe interpolation, camera matrices, and layer render commands.
- `zeno-format`: `.arx` container serialization, migrations, and packaging.
- `zeno-import`: PSD and Lottie converters.
- `zeno-script`: Sandboxed Rhai expression evaluator.
- `zeno-data`: Reactive data store and spreadsheet synchronization (Excel, CSV, Sheets).
- `zeno-media`: Capture sources (Display, Window, Camera, Media) and lock-free double buffering.
- `zeno-audio`: 48 kHz float audio mixer, ITU-R BS.1770 loudness meters, drift correction, and DSP filter chains.
- `zeno-compositor`: Live GPU multi-scene compositor, Preview/Program buses, transition shaders, and ping-pong filter passes.
- `zeno-output`: Zero-copy GPU texture sharing for hardware encoders (NVENC/AMF/QSV), staged double-buffered CPU fallback (x264), and RTMP/SRT streaming.
- `zeno-control`: REST/WebSocket control server and global hotkey dispatcher.
- `zeno-platform`: Isolated Windows Win32/COM/GraphicsCapture bindings.
- `zeno-license`: Ed25519 offline license verification.
- `zeno-ui`: Broadcast dockable interface, timeline, curve editor, properties panels, and dialogs.
- `zeno-app`: Main desktop application binary.
- `zeno-cli`: Headless CLI renderer and validator.

---

## 6A. Filters System (The v1.5 Headline Addition)

Formalized clean-room filter architecture for both video conditioning/effects and audio DSP.

### 6A.1 Clean-Room Data Model

The filter data model is designed for Rust's ownership model, avoiding internal C pointers, manual mutexes, or fixed-offset arrays:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FilterCategory {
    Video,
    Audio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FilterKind {
    // Video Effect Filters
    ColorCorrection,
    ChromaKey,
    Blur,
    Sharpen,
    CropPad,
    Mask,

    // Audio DSP Filters
    Gain,
    NoiseGate,
    Compressor,
    Limiter,
    Eq,
    NoiseSuppression,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FilterDefinition {
    pub id: FilterId,
    pub kind: FilterKind,
    pub name: String,
    pub enabled: bool,
    pub settings: FilterSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FilterChain {
    pub filters: Vec<FilterDefinition>,
}
```

- **Filter Categories**:
  - `Video`: ColorCorrection, ChromaKey, Blur, Sharpen, CropPad, Mask.
  - `Audio`: Gain, NoiseGate, Compressor, Limiter, Eq, NoiseSuppression.
- **Typed Filter Settings**:
  - `ColorCorrection`: Brightness (-1.0..1.0), Contrast (0.0..2.0), Saturation (0.0..2.0), Hue shift (-180..180 deg), Opacity (0.0..1.0).
  - `ChromaKey`: Key color [R, G, B], Similarity (0.0..1.0), Smoothness (0.0..1.0), Spill reduction (0.0..1.0).
  - `Blur`: Radius (0.0..64.0 px, 7x7 Gaussian kernel).
  - `Sharpen`: Sharpness (0.0..2.0 unsharp masking).
  - `CropPad`: Left, Top, Right, Bottom pixel insets.
  - `Mask`: Optional mask asset ID and luminance invert flag.
  - `Gain`: Decibel adjustment (-60.0..+24.0 dB).
  - `NoiseGate`: Open threshold, close threshold, attack, hold, release ms.
  - `Compressor`: Ratio, threshold dB, attack ms, release ms, makeup gain dB.
  - `Limiter`: Ceiling dBFS (-24.0..0.0 dBFS), release ms.
  - `Eq`: 3-band biquad IIR (Low 100 Hz, Mid 1 kHz, High 8 kHz dB adjustments).
  - `NoiseSuppression`: Suppression strength (0.0..1.0), noise floor tracking, speech spectral preservation.

### 6A.2 Video Effect Processing Pipeline
- Video filters execute sequentially using an offscreen ping-pong render target pair (`filter_texture_a`, `filter_texture_b`) in `zeno-compositor`.
- Pre-allocated offscreen textures prevent per-frame GPU reallocations.
- The output of the final active filter feeds directly into the scene item composition pass.
- Bypassed (disabled) filters pass textures through with zero overhead.

### 6A.3 Audio DSP Filter Pipeline
- Audio filters slot directly into `zeno-audio`'s per-source real-time mixing worker.
- Filters execute in strict list order over 48 kHz stereo floating-point sample chunks.
- Meter integration tracks sample-accurate True-Peak and RMS before and after filter chain application.

### 6A.4 UI & Operator Workflow
- Dual-list modal dialog (`FilterDialogState`) accessible via:
  - Source toolbar **Filters...** button.
  - Right-click context menu on any source row: **Filters...**, **Copy Filters**, and **Paste Filters**.
  - Source properties dialog **Filters...** button.
- Left column: Ordered filter chain with:
  - Enable/disable toggle button.
  - Reordering buttons (`^` Move Up, `v` Move Down).
  - Removal button (`x`).
  - **Add Filter** popover picker grouped by category (`Video Effects` vs `Audio DSP`).
- Right column: Typed properties inspector rendering numeric drag values and sliders tailored to the active filter kind.
- Bottom toolbar: **Copy Filters**, **Paste Filters**, and **Close**.
- Strict design rule: Flat surfaces, no gradients, and theme tokens from `theme.rs`.

### 6A.5 Serialization
- Filter chains serialize under the `filters` field of each `SourceDefinition` in `.arxproj` studio projects.
- Motion graphics overlays serialize filter chains into `.arx` container JSON metadata.
- **Copy / Paste Filters**: Serializes the selected source's `FilterChain` to memory clipboard and deep-clones fresh filter instances with regenerated `FilterId`s onto destination sources.

---

## 7. Compositor Blending Modes and Blending Method

Zenocast supports 7 premultiplied-alpha blend modes mapped directly to GPU fixed-function blend states (`wgpu::BlendState`):

1. **Normal**: `src: One`, `dst: OneMinusSrcAlpha`, `op: Add` ($C_{out} = C_{src} + C_{dst} \cdot (1 - A_{src})$).
2. **Add**: `src: One`, `dst: One`, `op: Add` ($C_{out} = C_{src} + C_{dst}$).
3. **Subtract**: `src: One`, `dst: One`, `op: ReverseSubtract` ($C_{out} = C_{dst} - C_{src}$).
4. **Screen**: `src: One`, `dst: OneMinusSrc`, `op: Add` ($C_{out} = C_{src} + C_{dst} \cdot (1 - C_{src})$).
5. **Multiply**: `src: Dst`, `dst: OneMinusSrcAlpha`, `op: Add` ($C_{out} = C_{src} \cdot C_{dst} + C_{dst} \cdot (1 - A_{src})$).
6. **Lighten**: `src: One`, `dst: One`, `op: Max` ($C_{out} = \max(C_{src}, C_{dst})$).
7. **Darken**: `src: One`, `dst: One`, `op: Min` ($C_{out} = \min(C_{src}, C_{dst})$).

### Blending Methods
- `Linear` (default): Physically correct compositing in linear sRGB space.
- `SrgbOff` (legacy OBS emulation): De-linearizes color channels ($C^{1/2.2}$) before alpha multiplication, brightening semi-transparent alpha fringes.

---

## 10. Hardware Video Encoding and Zero-Copy Promotion

Zenocast's output subsystem (`zeno-output`) provides broadcast multiplexing:
- **Zero-Copy Shared GPU Texture Path**: Promoted to default requirement for NVENC, AMF, and QSV hardware encoders. Bypasses CPU RAM transfers by passing Direct3D / Vulkan shared texture handles directly to the encoder pipeline.
- **Staged Double-Buffered CPU Fallback**: Non-blocking staging ring buffer (`StagedDoubleBuffer`) used for software x264/x265 encoding and when host hardware sharing is unavailable. Decouples render framerates from CPU color conversion.
- **Strict Monotonic Nanosecond Timestamps**: End-to-end atomic presentation timestamp (PTS) progression matching Section 10 master timeline clock, preventing A/V drift across RTMP and SRT outputs.

---

## 13. Milestone Roadmap

- **M0**: Multi-crate Cargo workspace and CI configuration.
- **M1**: Foundation data structures, properties, and math.
- **M2**: WGPU renderer with shapes, images, and text.
- **M3**: Pure evaluation engine, keyframe solver, and CLI renderer.
- **M4**: Capture sources (Display, Window, Camera) with double-buffered textures.
- **M5**: Live multi-scene compositor, transitions, and Preview/Program buses.
- **M5b**: Complete transition palette (Wipe, Stinger, Zoom, Dip) and snapshot interruption.
- **M5c**: **Video Effect Filters.** Filter chain data model, offscreen-texture render pass integration, Color Correction, Chroma Key, Blur, Sharpen, Crop/Pad, Mask filters, Filters modal UI, drag-reorder, enable/disable, serialization. *Accept:* a Chroma Key filter on a camera source correctly keys out a green screen background in a live scene at 60 fps without dropping below budget.
- **M6**: Audio mixer, true-peak/RMS meters, clock drift correction, WASAPI ingest.
- **M6b**: **Audio DSP Filters.** Reorderable filter chain for audio sources, gain/noise-gate/compressor/limiter/EQ/noise-suppression filter kinds, enable/disable per filter, meter integration (pre- and post-filter monitoring). *Accept:* a microphone source with noise suppression and compressor correctly attenuates background noise while keeping speech levels within broadcast spec (-24 LKFS / -1 dBFS True-Peak ceiling).
- **M7**: Hardware video encoding (NVENC/AMF/QSV/x264) with promoted zero-copy shared texture path and double-buffered staging fallback, RTMP/SRT streaming, and MKV recording.
- **M7b**: Direct YouTube Live Streaming API v3 integration with PKCE OAuth flow.
- **M8**: Broadcast dockable UI, timeline editor, and theme strictness.
- **M9**: Complete broadcast production studio workflow.

