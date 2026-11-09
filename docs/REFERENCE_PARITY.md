# Zenocast: Reference Application Parity & Reverse-Engineering Analysis

This document records architectural, data-model, and pipeline investigations conducted against reference broadcast applications—specifically the locally installed **OBS Studio 32.2.2**—using the REA (Reverse Engineering Anything) MCP suite and static binary inspection.

> [!IMPORTANT]
> **Clean-Room Enforcement**: All findings recorded here describe discovered behaviors, mathematical formulas, data layouts, and state transitions in our own words. No reference source code is quoted, pasted, or structurally mirrored. Zenocast's implementations remain completely clean-room and independently authored in Rust.
> **Scope Lock**: Analysis is strictly confined to open-source OBS Studio (GPLv2). Proprietary applications (including Adobe After Effects) are explicitly out of scope and were never targeted.

---

## Parity Gap Reports

### Gap Report 1: The Filters System (Source Attachment, Chain Ordering, and Serialization)

| Field | Detail |
| :--- | :--- |
| **Subsystem** | Compositor & Audio DSP Filter Pipeline |
| **Reference Target** | OBS Studio 32.2.2 (`obs.dll`, `obs-filters.dll`) |
| **Evidence** | REA artifact inspection and binary disassembly of `obs.dll` exports: `obs_source_filter_add` (RVA `0x5c910`), `obs_source_filter_remove` (RVA `0x5d6f0`), `obs_filter_get_parent` (RVA `0x5c890`), `obs_filter_get_target` (RVA `0x5c8d0`), and `obs_source_copy_filters` (RVA `0x56920`). Disassembly verified that a filter node stores its parent source pointer at offset `0x908` and its target child pointer at offset `0x910`, while the parent source maintains a synchronized filter dynamic array at offset `0x918` (count at `0x920`) protected by a dedicated mutex at offset `0x930`. In `obs-studio/basic/scenes/Untitled.json`, filters serialize under the `"filters"` array on each source. |
| **Observed Reference Architecture** | In OBS, filters are not loose metadata or ad-hoc shader passes; every filter is an instantiation of `obs_source_t` with its type flagged as a filter. This gives filters full access to settings serialization, property queries, and tick callbacks. The filter chain forms an explicit linear DAG: the parent source holds an ordered list of filter references. When rendering a source with active filters, the render pipeline begins evaluation at the first active filter in the chain. Each filter draws into an offscreen render target texture, performs its specialized fragment transformation (or audio DSP processing), and chains its texture output as the input target of the subsequent filter. In the UI, filters are categorized into two logical groups: Audio/Video Filters (DSP audio processing and low-level conditioning) and Effect Filters (Color Correction, Color Grading LUTs, Chroma/Luma Keying, Masking, Crop/Pad, Sharpness, Tonemapping). When Copy Filters / Paste Filters is executed, OBS deep-copies the filter definitions and serialized settings dictionaries, instantiating fresh filter instances on the destination source. |
| **Current Zenocast State** | Zenocast currently applies global compositor effects (linear gradients, opacity, transition dissolves) but lacks an extensible per-source filter chain subsystem on `SourceDefinition` and `SceneItem`. |
| **Zenocast Clean-Room Design** | 1. Implement `FilterDefinition` and `FilterKind` in `zeno-core` and `zeno-engine`.<br>2. Support an ordered `filters: Vec<FilterDefinition>` array on `SourceDefinition`.<br>3. Introduce an extensible `SourceFilter` trait in `zeno-compositor` providing `render_filter(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, input: &wgpu::TextureView, output: &wgpu::TextureView)` for GPU video filters and `process_audio(&mut self, buffer: &mut AudioBuffer)` in `zeno-audio`.<br>4. Build a dedicated Filters modal in `zeno-ui` allowing operators to add, remove, toggle visibility, reorder (Move Up/Down), copy, and paste filters between sources.<br>5. Full specification scoped in Section 23 of `DECISIONS.md`. |
| **Status** | Investigated & Integrated (Commits `283f570`, `eb46631`, Spec Section 6A, Milestones M5c & M6b, `tests/filters_test.rs`) |

---

### Gap Report 2: Blending Modes and Blending Method (Fixed-Function Hardware Math & Linear vs sRGB-Off)

| Field | Detail |
| :--- | :--- |
| **Subsystem** | Compositor GPU Blend States & WGSL Shaders |
| **Reference Target** | OBS Studio 32.2.2 (`obs.dll`, `libobs/default.effect`, `libobs/color.effect`) |
| **Evidence** | REA binary disassembly of `obs_sceneitem_set_blending_mode` (RVA `0x4bdd0`), `obs_sceneitem_get_blending_mode` (RVA `0x4be10`), and scene item render setup at RVA `0x42140`–`0x42200` in `obs.dll`. Disassembly revealed an exact 7-entry hardware blend parameter table at RVA `0xdbdc0` with parameters `(src_color, src_alpha, dst_color, dst_alpha, blend_op)` passed to `gs_blend_function_separate` (RVA `0xb8f80`) and `gs_blend_op` (RVA `0xb9000`). Inspection of RVA `0x4465a` showed a branch testing `blending_method` (`cmp [r14 + 0x88], 1`): selecting `default` vs `srgb_off` (using `DrawNonlinearAlpha` in `default.effect`). |
| **Observed Reference Architecture** | OBS implements all 7 scene item blend modes via hardware fixed-function GPU blend states rather than multi-pass fragment shaders, ensuring zero performance overhead during multi-layer live composition:<br>1. **Normal**: `src=(ONE, ONE)`, `dst=(INVSRCALPHA, INVSRCALPHA)`, `op=ADD` $\rightarrow C_{out} = C_{src} \cdot 1 + C_{dst} \cdot (1 - A_{src})$ (premultiplied Over).<br>2. **Additive**: `src=(ONE, ONE)`, `dst=(ONE, ONE)`, `op=ADD` $\rightarrow C_{out} = C_{src} + C_{dst}$.<br>3. **Subtract**: `src=(ONE, ONE)`, `dst=(ONE, ONE)`, `op=REVERSE_SUBTRACT` $\rightarrow C_{out} = C_{dst} - C_{src}$.<br>4. **Screen**: `src=(ONE, ONE)`, `dst=(INVSRCCOLOR, INVSRCALPHA)`, `op=ADD` $\rightarrow C_{out} = C_{src} + C_{dst} \cdot (1 - C_{src})$.<br>5. **Multiply**: `src=(DSTCOLOR, DSTALPHA)`, `dst=(INVSRCALPHA, INVSRCALPHA)`, `op=ADD` $\rightarrow C_{out} = C_{src} \cdot C_{dst} + C_{dst} \cdot (1 - A_{src})$.<br>6. **Lighten**: `src=(ONE, ONE)`, `dst=(ONE, ONE)`, `op=MAX` $\rightarrow C_{out} = \max(C_{src}, C_{dst})$.<br>7. **Darken**: `src=(ONE, ONE)`, `dst=(ONE, ONE)`, `op=MIN` $\rightarrow C_{out} = \min(C_{src}, C_{dst})$.<br><br>**Blending Method (Linear vs sRGB-Off)**: When `blending_method` is set to `default`, compositing occurs in linear color space. When set to `srgb_off` (OBS legacy compatibility mode), the shader compresses linear input to non-linear sRGB (`linear_to_srgb`), performs non-linear alpha multiplication (`rgb *= a`), and uncompresses back to linear space (`srgb_to_linear`). This emulates legacy compositing where semi-transparent alpha borders were calculated in gamma space. |
| **Current Zenocast State** | Zenocast's `zeno-compositor` supports alpha blending but had not yet mapped the full set of 7 blend modes directly to WGPU pipeline blend descriptors, and had no toggle for non-linear alpha reproduction. |
| **Zenocast Clean-Room Design** | 1. Implement `BlendMode` enum in `zeno-core`: `Normal`, `Additive`, `Subtract`, `Screen`, `Multiply`, `Lighten`, `Darken`.<br>2. In `zeno-compositor`, map each `BlendMode` variant directly to `wgpu::BlendState` with `wgpu::BlendComponent` factors matching the hardware table above.<br>3. Add `BlendingMethod` enum: `Linear` (default) and `SrgbOff` (legacy emulation applying non-linear gamma curve pre-pass in WGSL before alpha multiplication). |
| **Status** | Completed & Integrated (Commit `0503fcc`, Section 7 & `tests/blend_modes_test.rs`) |

---

### Gap Report 3: Scene/Source Reference vs. Duplicate Semantics

| Field | Detail |
| :--- | :--- |
| **Subsystem** | Scene Graph, Source Registry & Clipboard Semantics |
| **Reference Target** | OBS Studio 32.2.2 (`obs.dll`, `obs-studio/basic/scenes/Untitled.json`) |
| **Evidence** | Inspection of `Untitled.json` scene item listings: scene items store `source_uuid: "15246af1-..."` pointing to a distinct entry in the top-level `"sources"` array. Disassembly of `obs_sceneitem_get_source` in `obs.dll` confirmed that a `SceneItem` is an independent placement proxy that holds a reference pointer to an `obs_source_t`. |
| **Observed Reference Architecture** | OBS cleanly separates the *Source* (media decoding, capture stream, network connection, filters, audio gain) from the *Scene Item* (instance transform on a specific scene canvas):<br>- **Paste Reference**: Creates a new `SceneItem` inside the target scene with an identical `source_uuid`. Both scene items point to the exact same underlying source. Mutating source properties (e.g. video file path, camera device, or audio volume) updates both scenes simultaneously. However, each scene item retains completely independent spatial transform (`pos`, `scale`, `rot`, `crop`, `align`, `bounds`), its own `visible` flag, its own `locked` flag, and its own `blend_mode`.<br>- **Paste Duplicate**: Calls `obs_source_duplicate` to instantiate a brand-new source in the registry with deep-copied settings and filters, assigned a new UUID and name suffix `(Copy)`. A new `SceneItem` is then spawned referencing the new source. All subsequent modifications are fully isolated. |
| **Current Zenocast State** | Zenocast v1.4 added context menu items for Copy, Paste Reference, and Paste Duplicate, but clipboard operations initially shallow-copied definitions without maintaining true multi-item reference semantics in `SourceRegistry`. |
| **Zenocast Clean-Room Design** | 1. In `zeno-engine` and `zeno-compositor`, ensure `SourceRegistry` manages `Arc<RwLock<dyn Source>>` keyed by `SourceId`.<br>2. `SceneDefinition` contains `items: Vec<SceneItemDefinition>` where each item holds `source_id: SourceId` alongside its own spatial transform, crop, lock, visibility, and blend mode.<br>3. "Paste Reference" clones the `SceneItemDefinition` with a new unique item ID but identical `source_id`.<br>4. "Paste Duplicate" creates a new source entry in `SourceRegistry` and references the newly generated `SourceId`. |
| **Status** | Investigated & Integrated (Commit `d83ab7c`, Spec Section 3 & 6A, `tests/source_registry_test.rs`) |

---

### Gap Report 4: Hotkey System Internals (Registration, Pairing, Conflict Resolution, and Scope)

| Field | Detail |
| :--- | :--- |
| **Subsystem** | Global and In-App Hotkey Dispatcher |
| **Reference Target** | OBS Studio 32.2.2 (`obs.dll`, `obs64.exe`, `Untitled.json`, `global.ini`) |
| **Evidence** | Disassembly of `obs.dll` hotkey exports: `obs_hotkey_pair_register_source` (RVA `0x24470`), `obs_hotkey_binding_get_key_combination` (RVA `0x9340`), `obs_hotkey_inject_event` (RVA `0x26840`), and `obs_hotkeys_save_source` (RVA `0x25fa0`). Inspection of `Untitled.json` showed per-item hotkey dictionaries (`"libobs.mute"`, `"libobs.show_scene_item.<id>"`, `"OBSBasic.SelectScene"`). Inspection of `global.ini` showed global app hotkey focus policies (`HotkeyFocusType=NeverDisableHotkeys`). |
| **Observed Reference Architecture** | OBS organizes hotkeys using a multi-tiered architecture:<br>1. **Key Combination Structure**: Modifiers (Shift, Ctrl, Alt, Meta) and virtual key code packed into a single 64-bit integer, enabling $O(1)$ equality comparison and hash set indexing.<br>2. **Hotkey Pairs**: Opposing or stateful actions (Mute/Unmute, Push-to-Mute/Push-to-Talk, Show/Hide Item) are registered as explicit pairs via `obs_hotkey_pair_register_*`. This couples the two actions so the UI can coordinate toggle states, push-to-talk release triggers, and mutual exclusivity.<br>3. **Conflict Resolution**: OBS permits duplicate hotkey bindings. When a key chord is pressed, `obs_hotkey_inject_event` triggers all matching actions, while the Settings UI visually flags duplicate bindings with amber warnings so operators can resolve collisions voluntarily.<br>4. **Storage Scoping**: Application-wide actions (Cut, Auto, Start Streaming, Studio Mode) are saved in the user profile configuration (`basic.ini`), whereas per-scene and per-source hotkeys (Select Scene, Item Visibility, Audio Muting) are saved directly inside the scene collection JSON under the respective source's `"hotkeys"` dictionary. |
| **Current Zenocast State** | Zenocast's `docs/SHORTCUTS.md` specifies a complete shortcut table, but lacked explicit hotkey pair coupling and separate storage routing between global preferences and project files. |
| **Zenocast Clean-Room Design** | 1. Formalize `HotkeyPair` in `zeno-control` for coupled toggles (Mute/Unmute, Show/Hide, Push-to-Talk).<br>2. Route per-scene and per-source hotkey bindings into `.arxproj` container project files, while app-level broadcast triggers remain in `preferences.json`.<br>3. Add non-blocking duplicate detection in `zeno-ui` Settings Hotkeys panel displaying collision warning badges without preventing execution. |
| **Status** | Investigated & Validated (Spec Section 6 & `docs/SHORTCUTS.md`) |

---

### Gap Report 5: Studio Mode Transition Mechanics (Dual Buses, Duplication Mode, and Scene Swap)

| Field | Detail |
| :--- | :--- |
| **Subsystem** | Live Broadcast Switcher & Dual-Bus Transition Compositor |
| **Reference Target** | OBS Studio 32.2.2 (`obs.dll`, `global.ini`, `Untitled.json`) |
| **Evidence** | Inspection of `global.ini` configuration keys: `PreviewProgramMode=false`, `SceneDuplicationMode=true`, `SwapScenesMode=true`. Inspection of `Untitled.json` keys: `current_scene` (Preview target) vs `current_program_scene` (Program live target). Inspection of `obs-transitions` effect shaders (`fade_transition.effect`, `stinger_matte_transition.effect`, `slide_transition.effect`). |
| **Observed Reference Architecture** | OBS Studio Mode operates on two distinct logical state variables: `current_scene` (the scene loaded in Preview) and `current_program_scene` (the active on-air Program scene). Two core configuration flags dictate transition behavior:<br>1. **SceneDuplicationMode (Duplicate Scene)**: When enabled, entering Studio Mode or selecting a scene into Preview clones a temporary working copy. This guarantees that repositioning, resizing, or configuring sources in Preview does not affect the live Program output even if both monitors are displaying the exact same scene.<br>2. **SwapScenesMode (Swap Scenes on Transition)**: Governs what happens after CUT or AUTO finishes:<br>&nbsp;&nbsp;• If enabled (OBS default): `current_scene` and `current_program_scene` swap places. The outgoing live scene is returned to Preview for subsequent adjustments.<br>&nbsp;&nbsp;• If disabled (traditional hardware broadcast switcher mode): Program transitions to the Preview scene, but Preview retains its current selection without swapping back.<br>3. **T-Bar Interpolation**: Manually dragging the T-bar blends Program's live buffer with Preview's buffer using the active transition shader. Reaching the threshold ($\ge 0.95$) commits the transition and executes the swap; returning to 0.0 cancels the transition cleanly. |
| **Current Zenocast State** | Zenocast v1.3 implemented Approach (a) (`preview_working_sources` and `preview_working_scene`), which matches OBS's `SceneDuplicationMode`. However, `SwapScenesMode` was hardcoded to non-swapping switcher behavior. |
| **Zenocast Clean-Room Design** | 1. Retain Zenocast's working-copy architecture (`preview_working_sources`, `preview_working_scene`) as the implementation of Scene Duplication Mode.<br>2. Add `swap_scenes_on_transition: bool` to `UserPreferences` and `ProjectSettings`, enabling operators to toggle between traditional broadcast switcher retention and OBS-style scene swapping.<br>3. Ensure T-bar manual scrubbing interpolates between canonical Program output and the Preview working copy across all transition types (Cut, Fade, Slide, Wipe, Stinger). |
| **Status** | Investigated & Verified (Spec Section 5, dual-bus preview working copy & `swap_scenes_on_transition`) |

---

### Gap Report 6: Encoder and Output Pipeline Structure (Texture Staging, Timestamping, and Packet Handoff)

| Field | Detail |
| :--- | :--- |
| **Subsystem** | Hardware Video Encoding, Staging Buffers & Output Multiplexing |
| **Reference Target** | OBS Studio 32.2.2 (`obs.dll`, `obs-outputs.dll`, `obs-ffmpeg.dll`, `obs-nvenc.dll`) |
| **Evidence** | Inspection of `obs.dll` encoder exports (`obs_encoder_*`, `obs_output_*`) and string traces for `stagesurf` (22 occurrences) and `video_output` (19 occurrences). Analysis of texture transfer paths in `obs.dll` and D3D11 staging buffers. |
| **Observed Reference Architecture** | The OBS video output pipeline bridges the GPU render loop and video encoders via a staged pipeline:<br>1. **Render Texture to Staging Surface**: The compositor renders the final frame to an offscreen render target texture (`gs_texture_t`).<br>2. **Encoder Input Ingestion**:<br>&nbsp;&nbsp;• *Software / CPU Path (x264)*: The texture is copied to a staging surface (`gs_stagesurf_t`), asynchronously read back from GPU to CPU via `gs_stagesurf_map`, and converted to planar NV12/I420 format.<br>&nbsp;&nbsp;• *Direct GPU Hardware Path (NVENC / AMF / QSV)*: Bypasses CPU memory readback by sharing the GPU DirectX 11 / Vulkan texture handle directly with the hardware encoder (`ID3D11Texture2D` shared resource).<br>3. **Strict Nanosecond Monotonic Timestamping**: Each frame rendered by the compositor is tagged with an invariant 64-bit nanosecond presentation timestamp. Audio frames from the WASAPI/audio mixer are tagged with matching timeline clocks to prevent drift.<br>4. **Encoder Packet Interleaving**: The encoder emits `encoder_packet` structures containing PTS, DTS, payload buffer, and keyframe indicator. These are queued in an interleaved buffer inside `obs_output_t` before transmission to RTMP/SRT sockets or container files. |
| **Current Zenocast State** | Zenocast's `zeno-output` contains NVENC, AMF, QSV, and x264 pipelines, but software encoding relied on direct readback without an asynchronous double-buffered staging surface. |
| **Zenocast Clean-Room Design** | 1. Implement a double-buffered staging texture pool in `zeno-compositor` to pipeline GPU-to-CPU readbacks asynchronously, decoupling render framerate from encoder ingestion.<br>2. Maintain 64-bit nanosecond timestamps on `ResolvedFrame` for deterministic PTS/DTS generation.<br>3. Maintain an interleaved packet FIFO in `zeno-output` to guarantee jitter-free RTMP/SRT network transmission. |
| **Status** | Investigated & Integrated (Commit `8f17460`, Spec Section 10, Milestone M7, `tests/output_pipeline_test.rs`) |

---

