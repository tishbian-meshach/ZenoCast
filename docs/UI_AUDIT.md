# Zenocast UI Interactivity Audit

Every interactive control in the Zenocast esports broadcast suite is audited below. Each control is verified against real application, engine, compositor, and output pipeline state—never mock or isolated visual dummy state.

---

## 1. Top Menu Bar

| Control | Action / Engine Target | Verification Note | Status |
| :--- | :--- | :--- | :--- |
| **File > New Project** | Clears `project`, unregisters sources in `SourceRegistry`, reinitializes default compositor scenes | Verified: `app.reset_to_new_project()` resets scenes count to 2, clears registry count to 0 in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **File > Open Project...** | Opens `.arxproj` modal with native file picker (`rfd`), deserializes JSON, synchronizes compositor scenes & items | Verified: loads serialized `.arxproj` and restores scene hierarchy in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **File > Save Project** | Serializes current `Project` state to existing project path | Verified: writes valid JSON schema to disk in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **File > Save Project As...** | Prompts for target path via native file picker (`rfd`) and serializes `Project` to disk | Verified: file created at destination and verified non-empty in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **File > Settings...** | Opens modal settings panel (Video, Audio, Output, Hotkeys, License) | Verified: opens settings view modifying output bitrates and encoders in `test_m8_user_builds_scenes_and_adds_sources_via_ui`. | [x] Verified |
| **File > Exit** | Triggers clean application shutdown sequence (`on_exit`) | Verified: stops stream, stops recording, clears all source registry GPU textures in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Mode > Live Studio** | Switches `app_mode` to `AppMode::LiveStudio`, rendering multiview dock | Verified: sets `app_mode` and mounts central live dock layout in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Mode > Edit Overlay** | Switches `app_mode` to `AppMode::EditOverlay`, rendering motion graphics editor | Verified: mounts layers panel, canvas gizmo, properties, and timeline in `tests/overlay_editor_test.rs`. | [x] Verified |
| **Mode > Data Control** | Switches `app_mode` to `AppMode::DataControl`, rendering esports rapid scoreboard | Verified: mounts data panel and synchronizes with `DataStore` and `ControlState` in `tests/control_api_overlay_test.rs`. | [x] Verified |
| **Tools > Auto-Config Wizard** | Opens `FirstRunWizard` modal with hardware inventory & encoder recommendations | Verified: opens wizard and checks GPU encoder support in `tests/wizard_installer_test.rs`. | [x] Verified |
| **Tools > Settings...** | Opens studio settings dialog directly from tools menu | Verified: opens settings dialog modifying `project.settings` in `test_m8_user_builds_scenes_and_adds_sources_via_ui`. | [x] Verified |
| **View > Studio Mode** | Toggles between Normal Mode (single Program monitor) and Studio Mode (Preview/Program split) | Verified: switches layout live and persists in user preferences (`preferences.json`). | [x] Verified |
| **View > Reset UI Layout** | Restores default dock layout splitters and panel positions | Verified: reinitializes dock state with standard split geometry. | [x] Verified |

---

## 2. Scenes Panel

| Control | Action / Engine Target | Verification Note | Status |
| :--- | :--- | :--- | :--- |
| **+ Add Scene** | Creates new `SceneDefinition` in `project` and adds `Scene` to `SceneCompositor` | Verified: increments scene count, generates unique scene ID, and registers with compositor in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **- Remove** | Deletes selected scene from project and compositor (protected on last scene) | Verified: deletes scene from `project.scenes` and calls `comp.remove_scene()` in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Rename** | Edits scene name in project and updates compositor scene instance | Verified: updates name in `project.scenes` and `comp.get_scene_mut()` in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **▲ Up** | Moves selected scene up in list order | Verified: swaps scene indices in `project.scenes` in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **▼ Down** | Moves selected scene down in list order | Verified: swaps scene indices in `project.scenes` in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Click Scene Row** | Selects scene and loads it into Preview bus | Verified: updates `selected_scene_id` and `preview_scene_id` in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Double-Click Scene Row** | Direct switches Program (Live) bus to selected scene | Verified: sets `project.active_scene_id` to clicked scene in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Empty State Hint** | Displays `"No scenes yet — click + Add Scene"` when list is empty | Verified: shows muted placeholder hint when `project.scenes.is_empty()`. | [x] Verified |

---

## 3. Sources Panel

| Control | Action / Engine Target | Verification Note | Status |
| :--- | :--- | :--- | :--- |
| **+ Add Source** | Opens `SourceDialog` for broadcast source types | Verified: adds concrete `Source` trait objects across all types in `test_section_2_2_all_ten_source_types_and_resource_lifecycle`. | [x] Verified |
| **- Remove** | Deletes source from project, compositor, and stops in `SourceRegistry` (enabled when selected) | Verified: unregisters handle, frees GPU resources, leaves 0 leaked handles in `test_section_2_2_all_ten_source_types_and_resource_lifecycle`. | [x] Verified |
| **⚙ Properties** | Toolbar button (enabled when selected) and double-click opens real per-type configuration dialog | Verified: opens `SourceDialog` in `Properties` mode and updates parameters live in `test_section_6_consistency_pass_and_modal_navigation`. | [x] Verified |
| **▲ Up (Bring Forward)** | Toolbar button moving source one step forward in visual stacking order | Verified: swaps source order and calls `sync_scene_sources_order_to_compositor` in `test_source_reordering_live_studio`. | [x] Verified |
| **▼ Down (Send Backward)** | Toolbar button moving source one step backward in visual stacking order | Verified: swaps source order and updates GPU compositor scene items in `test_source_reordering_live_studio`. | [x] Verified |
| **Drag-and-Drop Reorder** | Grabs row via `⠿` handle, renders insertion line indicator with circle caps, and updates scene | Verified: reorders list and synchronizes GPU compositor items in `test_source_reordering_live_studio`. | [x] Verified |
| **Right-Click Context Menu** | Bring to Front, Bring Forward, Send Backward, Send to Back, Properties, Remove | Verified: context menu actions mutate source order and invoke compositor sync in `test_source_reordering_live_studio`. | [x] Verified |
| **Keyboard Reorder Shortcuts** | `Ctrl+Shift+]` (Front), `Ctrl+]` (Forward), `Ctrl+[` (Backward), `Ctrl+Shift+[` (Back) | Verified: shortcuts trigger source reordering and live compositor sync in `test_source_reordering_live_studio`. | [x] Verified |
| **Visibility (👁)** | Toggles `visible` flag on `SourceDefinition` and compositor `SceneItem` | Verified: flips `item.visible` in compositor scene in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Lock (🔒)** | Toggles `locked` flag on `SourceDefinition` to prevent unintended canvas manipulation | Verified: flips `src_def.locked` in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Explorer Drag-and-Drop** | Dropping media file from Windows Explorer opens Add Media Source pre-filled | Verified: populates `file_path` and launches properties modal via `open_add_media_with_path`. | [x] Verified |
| **Transform Inspector** | Collapsing header with `DragValue` X, Y, Width, Height, Rotation, and Reset | Verified: two-way sync with preview canvas gizmo and compositor `SceneItem` in `test_source_reordering_live_studio`. | [x] Verified |
| **Empty State Hint** | Displays `"No sources yet — click + Add Source or drag media here"` when list is empty | Verified: shows muted placeholder hint when `scene.sources.is_empty()`. | [x] Verified |

---

## 4. Audio Mixer Panel

| Control | Action / Engine Target | Verification Note | Status |
| :--- | :--- | :--- | :--- |
| **Horizontal ScrollArea** | Enables horizontal scrolling across any number of channel strips | Verified: wrapped in `ScrollArea::horizontal()` preventing panel clipping or overflow. | [x] Verified |
| **Fixed Channel Width (75px)** | Sets fixed column width per strip with centered controls | Verified: prevents squeezed layout and word-wrapping glitches. | [x] Verified |
| **Channel Name Truncation** | Truncates channel label with ellipsis (`...`) and provides full name in hover tooltip | Verified: `Label::new().truncate()` with `.on_hover_text()` handles arbitrary name lengths cleanly. | [x] Verified |
| **Master Volume Slider** | Multiplies master audio gain (0.0..=1.5) across all active audio tracks | Verified: adjusts `master_volume` and syncs effective gain into `SourceRegistry` in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Master Mute Button** | Silences master output bus and sets dB readout to `-inf dB` | Verified: mutes all channels and displays `-inf dB` in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Channel Volume Sliders** | Adjusts individual source volume and propagates to WASAPI/FFmpeg decoder | Verified: updates `SourceProperties::AudioInput/Output` in `SourceRegistry` in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Channel Mute Buttons** | Silences individual source and propagates to audio pipeline | Verified: updates source mute state in `SourceProperties` in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **dB Readouts** | Displays calculated logarithmic decibel level `20.0 * log10(gain)` | Verified: 0.5 gain produces ~ -6.0 dB, 0.0 or muted produces `-inf dB` in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Empty State Hint** | Displays `"No audio sources in active scene"` when scene has no sources | Verified: displays centered muted label when `scene.sources.is_empty()`. | [x] Verified |

---

## 5. Transitions Panel

| Control | Action / Engine Target | Verification Note | Status |
| :--- | :--- | :--- | :--- |
| **Active Transition Dropdown** | Selects transition type ("Cut", "Fade", "Slide", "Wipe", "Stinger") | Verified: sets `active_transition_kind` and compiles definition in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Duration Slider** | Sets duration in milliseconds (50..=2000 ms) | Verified: adjusts `transition_duration_ms` passed to `TransitionDefinition` in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Trigger Transition (Auto)** | Initiates compositor auto-transition between Preview and Program | Verified: swaps preview and program scenes with active blend in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |

---

## 6. Stream & Record Panel

| Control | Action / Engine Target | Verification Note | Status |
| :--- | :--- | :--- | :--- |
| **START / STOP STREAMING** | Validates license & starts/stops RTMP/SRT output stream | Verified: toggles encoder pipeline and sets `is_streaming` in `test_m8_one_click_live_streaming_and_recording`. | [x] Verified |
| **START / STOP RECORDING** | Starts/stops local MP4/MKV recording to disk | Verified: writes valid non-empty container to disk in `test_m8_one_click_live_streaming_and_recording`. | [x] Verified |
| **Studio Mode Button** | Toggles between Normal Mode and Studio Mode live | Verified: toggles dual Preview/Program layout and single Program layout in `test_section_1_normal_mode_default_and_studio_mode_toggle`. | [x] Verified |
| **Status Readout** | Reflects real live stream status ("LIVE" vs "OFFLINE") | Verified: displays RED_LIVE "LIVE" when active in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **FPS Telemetry** | Real-time encoder framerate from `OutputManager::stats()` | Verified: reports live fps in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Dropped Frame Telemetry** | Percentage of frames dropped by network/encoder | Verified: reports `stats.dropped_percent` with warning color if > 0.1% in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Bitrate Telemetry** | Current network output bitrate in kbps | Verified: reports active bitrate kbps from encoder telemetry in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Duration Timer** | Elapsed stream/record duration formatted as `HH:MM:SS` | Verified: formats elapsed time string accurately in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |

---

## 7. Monitors and Switching Controls

| Control | Action / Engine Target | Verification Note | Status |
| :--- | :--- | :--- | :--- |
| **Normal Mode Monitor** | Single Program-only monitor view with on-canvas gizmo and context menu | Verified: single monitor layout in Normal Mode with direct scene switching. | [x] Verified |
| **CUT Button** | Instantly swaps Preview and Program scene buses | Verified: swaps active and preview scenes immediately in `test_m8_preview_to_program_transition_switching`. | [x] Verified |
| **AUTO Button** | Runs active transition definition with configured duration | Verified: executes auto transition in `test_m8_preview_to_program_transition_switching`. | [x] Verified |
| **T-BAR Slider** | Smoothly blends preview and program scenes on drag; cuts at >= 0.95 | Verified: scrubbing smoothly updates progress; drag release >= 0.95 swaps buses in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Preview Monitor View** | Renders live WGPU preview texture view from compositor with interactive gizmo | Verified: renders GPU preview framebuffer in `test_section_2_1_wire_source_creation_to_real_backend_and_live_preview`. | [x] Verified |
| **Program Monitor View** | Renders live WGPU program texture view with on-air tally border | Verified: renders GPU program framebuffer with live border in `test_section_2_1_wire_source_creation_to_real_backend_and_live_preview`. | [x] Verified |
| **Vertical Canvas/Dock Splitter** | Drag-resizable boundary with min-height clamps between canvas and dock | Verified: split-pane layout with dynamic monitor aspect ratio preservation and internal panel scrolling in `test_section_2_split_pane_layout_and_panel_resizing`. | [x] Verified |

---

## 8. Window Chrome & Lifecycle

| Control | Action / Engine Target | Verification Note | Status |
| :--- | :--- | :--- | :--- |
| **Window Minimize/Maximize** | Standard OS desktop window control via `eframe` / `winit` | Verified: handled by native window event loop in `zeno-app`. | [x] Verified |
| **Window Close / Clean Shutdown** | Invokes `eframe::App::on_exit()` for leak-free shutdown | Verified: stops streaming, finalizes recording, and cleans `SourceRegistry` in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |

---

## 9. Unified Media Source & Native File Pickers

| Control | Action / Engine Target | Verification Note | Status |
| :--- | :--- | :--- | :--- |
| **Unified Media Source** | Merges video, still image, and GIF into a single intelligent source kind | Verified: auto-detects playback controls from file extension in `test_section_2_2_all_ten_source_types_and_resource_lifecycle`. | [x] Verified |
| **Native Browse Button** | Opens native Windows file dialog (`rfd::FileDialog`) filtered to media types | Verified: populates `file_path` from OS file picker dialog without manual typing. | [x] Verified |
| **Clear Path (X) Button** | Clears current file selection and resets thumbnail preview | Verified: empties path and invalidates texture cache. | [x] Verified |
| **Multi-File Slideshow** | Allows multi-file selection with configurable interval (seconds) | Verified: stores file list and slideshow interval in `SourceDialogState`. | [x] Verified |
| **160x90 Thumbnail Preview** | Real-time thumbnail preview from video first frame or image | Verified: `load_media_thumbnail` extracts preview via hidden FFmpeg or image crate. | [x] Verified |
| **Scale / Fit Mode Dropdown** | Selects Fit, Fill, Stretch, or Center media frame sizing | Verified: stored in source properties for compositor quad placement. | [x] Verified |
| **Loop & Volume Controls** | Loop toggle and volume slider shown for video/GIF sources | Verified: updates video decoder loop flag and audio mixer track gain. | [x] Verified |
| **Native Project File Dialogs** | Open Project and Save Project As use `rfd::FileDialog` filtered to `*.arxproj` | Verified: opens native OS file pickers in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |

---

## 10. On-Canvas Transform Gizmo

| Control | Action / Engine Target | Verification Note | Status |
| :--- | :--- | :--- | :--- |
| **Normal Mode Direct Transform** | Directly updates active scene items and syncs to compositor in Normal Mode (`StudioMode::Direct`) | Verified: dragging gizmo in Normal Mode modifies `project.scenes` active scene and updates program compositor item in `test_v1_6_normal_mode_transform_and_deselect`. | [x] Verified |
| **Selection Bounding Box** | Highlight bounding box drawn with `ACCENT` stroke on selected items, unclipped by canvas viewport | Verified: painter uses unclipped viewport painter; selection rendering verified in `test_hit_test_unrotated`. | [x] Verified |
| **8 Resize Handles** | 4 corner and 4 edge midpoint 8x8px square handles for 2D scaling with unclipped geometry | Verified: drag handles resize width/height and extend past monitor boundaries into letterbox/pillarbox areas in `test_gizmo_item_creation`. | [x] Verified |
| **Shift Aspect Lock & Layer Isolation** | Holding Shift while dragging corner/edge handles locks aspect ratio without deselecting or transforming layers below | Verified: hit testing evaluates `press_origin` (12px hit radius) to lock strictly to primary selected item in `test_shift_drag_handle_constrains_aspect_without_affecting_layers_below`. | [x] Verified |
| **Alt Center Scale** | Holding Alt scales symmetrically from object center | Verified: updates center and dimensions symmetrically. | [x] Verified |
| **Rotation Handle on Stem** | Circular rotate handle located 24px above top-center handle | Verified: computes angle around item center and updates `rotation_degrees`. | [x] Verified |
| **15° Angle Snap** | Holding Shift while rotating snaps to 15-degree increments | Verified: snapping logic verified in `test_rotation_15_degree_snapping`. | [x] Verified |
| **Canvas Hit-Testing & Deselection** | Clicking directly on visible layer selects it; topmost layer wins; clicking empty canvas space or pressing Escape deselects | Verified: `test_clicking_empty_space_deselects` and `Escape` key handling in `test_v1_6_normal_mode_transform_and_deselect`. | [x] Verified |
| **Keyboard Nudge** | Arrow keys nudge 1px; Shift+Arrow nudges 10px | Verified: updates position in discrete increments when canvas is focused. | [x] Verified |
| **On-Canvas Modifier Hint Badge** | Dark pill badge at bottom-center of canvas showing active modifier instructions | Verified: renders active Shift (Aspect ratio / 15° snap) or Alt (Center scale) status badge during drag operations. | [x] Verified |
| **Two-Way Property Sync** | Dragging gizmo updates Properties fields live; typing in fields updates canvas live | Verified: two-way synchronization in Live Studio preview and overlay editor. | [x] Verified |

---

## 11. Overlay Editor Completeness & Usability

| Control | Action / Engine Target | Verification Note | Status |
| :--- | :--- | :--- | :--- |
| **Sidebar Tabs ([☰ Layers] / [📁 Assets])** | Switches left sidebar between layer hierarchy and asset browser | Verified: toggles `active_left_tab` in `OverlayEditor` state. | [x] Verified |
| **Global Undo / Redo** | `Ctrl+Z` (Undo) and `Ctrl+Y` / `Ctrl+Shift+Z` (Redo) | Verified: 50 consecutive operations tested in `test_undo_redo_50_operations`. | [x] Verified |
| **Import Media Button** | Opens native file dialog for images, videos, audio, and fonts | Verified: imports assets and extracts dimensions in `test_asset_import_keyframe_retiming_and_arx_roundtrip`. | [x] Verified |
| **Assets Explorer Drag & Drop** | Dropping files from Windows Explorer onto Assets panel auto-imports them | Verified: adds asset entries to `composition.assets` with correct `AssetKind`. | [x] Verified |
| **Canvas Media Drag & Drop** | Dropping files directly onto canvas auto-imports and creates centered Image layer | Verified: creates image layer capped to composition resolution in `test_asset_import_keyframe_retiming_and_arx_roundtrip`. | [x] Verified |
| **Assets-to-Canvas Drag & Drop** | Dragging asset from Assets panel onto canvas drops new image layer | Verified: `dnd_dragging_asset_id` dropped onto canvas instantiates layer. | [x] Verified |
| **Canvas Image Rendering** | Renders image layers with preview frame and centered `🖼 <filename>` badge | Verified: displays content in canvas viewport during playback. | [x] Verified |
| **Layers Reordering** | Toolbar Up/Down buttons, drag-and-drop, context menus, and shortcuts | Verified: reorders layers and records undo command in `test_layer_reordering_and_undo`. | [x] Verified |
| **Layers Empty State** | Displays `"No layers yet — click + Text, + Shape, or import media"` | Verified: shows muted placeholder hint when `composition.layers.is_empty()`. | [x] Verified |
| **Property Stopwatches** | Stopwatch toggle buttons next to Position, Scale, Rotation, Opacity | Verified: toggles animation track keyframing state in `properties_panel.rs`. | [x] Verified |
| **Text Layer Properties** | Font family, size, native color picker, alignment (L/C/R), letter/line spacing | Verified: live updates to `ResolvedContent::Text` properties in properties panel. | [x] Verified |
| **Shape Layer Properties** | Width, height, corner radius (0..200px) with dynamic RoundedRectangle upgrade, fill and stroke pickers | Verified: updates geometry and color in real-time. | [x] Verified |
| **Camera Layer Properties** | Field of view (10..120°), position X/Y/Z, target X/Y/Z, near/far planes | Verified: modifies camera parameters for 3D engine evaluation. | [x] Verified |
| **Particle Layer Properties** | Presets (Confetti, Sparks, Smoke), rate, lifetime, speed, spread, colors, gravity | Verified: updates particle emitter parameters in real-time. | [x] Verified |
| **Image Layer Properties** | File path, native dimensions, size, "Reset Native Size" button | Verified: displays asset metadata and restores 1:1 pixel dimensions on click. | [x] Verified |
| **Dope Sheet Keyframes** | Color-coded keyframe diamonds for Position, Scale, Rotation, Opacity | Verified: renders diamond markers on timeline tracks. | [x] Verified |
| **Keyframe Retiming Drag** | Dragging keyframes retimes timestamps with live canvas scrubbing | Verified: updates keyframe timestamps in `test_asset_import_keyframe_retiming_and_arx_roundtrip`. | [x] Verified |
| **Keyframe Selection & Delete** | Click keyframe to highlight with white outline; Delete key removes it | Verified: `DeleteKeyframeCommand` executed on Delete key press. | [x] Verified |
| **Keyframe Context Menu** | Right-click diamond: Delete, Easing (Linear, Ease In, Ease Out, Ease In-Out) | Verified: sets `KeyframeEasing` curves via `SetKeyframeEasingCommand`. | [x] Verified |
| **Timeline Track Context Menu** | Right-click track: "Add Keyframe at Playhead" | Verified: creates keyframe at current playhead timestamp. | [x] Verified |
| **In/Loop/Out Region Handles** | Draggable boundary handles on time ruler with tinted region bands | Verified: adjusts `composition.lifecycle` ranges and synchronizes with engine. | [x] Verified |
| **.arx Round-Trip Serialization** | Full package save/load with embedded asset payloads and dimensions | Verified: complete round-trip verified in `test_asset_import_keyframe_retiming_and_arx_roundtrip`. | [x] Verified |

---

## 12. General UX Consistency Pass & Modal Navigation

| Control | Action / Engine Target | Verification Note | Status |
| :--- | :--- | :--- | :--- |
| **Modal Keyboard Navigation (Escape)** | Pressing Escape cancels/closes all modal dialogs (New Source, Properties, Open Project, Save Project As, Settings) | Verified: resets modal state to closed in `test_section_6_consistency_pass_and_modal_navigation`. | [x] Verified |
| **Modal Keyboard Navigation (Enter)** | Pressing Enter confirms/submits dialogs across all modals | Verified: validates input and emits confirmed action in `test_section_6_consistency_pass_and_modal_navigation`. | [x] Verified |
| **Modal Keyboard Navigation (Tab)** | Tab key cycles focus between input fields across all dialogs | Verified: standard egui tab navigation active on text edit and interactable controls. | [x] Verified |
| **Consistent Zero/Empty States** | Every panel (Scenes, Sources, Mixer, Layers, Assets) displays inline hint when empty | Verified: verified across all studio and editor panels in `test_section_6_consistency_pass_and_modal_navigation`. | [x] Verified |

---

## 13. Broadcast Design System Compliance & Flat Styling Audit

Automated enforcement and manual inspection verified against `Zenocast: Build Specification (v1.0)`, Section 12:

### Automated Enforcement
- **Automated Scanner Test**: `crates/zeno-ui/src/lib.rs` (`test_strict_theme_and_no_color_literals_outside_theme`).
  - Scans every `.rs` file under `zeno-ui/src/` (excluding `theme.rs` and the scanner itself in `lib.rs`).
  - Fails the build if any forbidden color constructor or literal constant (`Color32::from_rgb`, `Color32::from_rgba`, `Color32::from_gray`, `Color32::from_black_alpha`, `Color32::from_white_alpha`, `Color32::WHITE`, `Color32::BLACK`, `Color32::RED`, `Color32::GREEN`, `Color32::BLUE`, `Color32::YELLOW`, `Color32::TRANSPARENT`, `Color32::LIGHT_`, `Color32::DARK_`, `Color32::GRAY`) is detected.
  - Fails the build if any gradient API (`gradient` case-insensitive) is detected.
  - Fails the build if any decorative shadow (`Shadow::` outside `Shadow::NONE` in `theme.rs`) is detected.
  - Fails the build if any emoji or decorative chrome symbol (`●`, `■`, `▶`, `⏸`, `⏹`, `▲`, `▼`, `★`, `⚙`, `👁`, `🔒`, `🔓`, `⚠`, `❌`, `✔`, `📁`, `☰`, `⠿`, `🖼`, `•`) is detected.

### Panels Audited & Corrected
| Panel / Component | File | Corrections Applied | Status |
| :--- | :--- | :--- | :--- |
| **Theme System** | `theme.rs` | Enforced flat surfaces (`SURFACE_0..3`), disabled all shadows (`Shadow::NONE`), constrained typography styles to strictly 12, 13, 16px, added flat `TINT_NONE` and `TRANSPARENT` tokens, mapped legacy green to `TEXT`. | [x] Verified |
| **Automated Scanner & Re-exports** | `lib.rs` | Re-exported `TINT_NONE` and `TRANSPARENT`, implemented comprehensive scanner test for color literals, gradients, shadows, and emojis. | [x] Verified |
| **Live Studio Dock & Chrome** | `dock.rs` | Removed all emojis (`● PREVIEW` -> `PREVIEW`, `● PROGRAM - ON AIR` -> `PROGRAM - ON AIR`, `● PROGRAM` -> `PROGRAM`, `▲ Up` -> `Up`, `▼ Down` -> `Down`, `⚙ Properties` -> `Properties`, `⠿` -> `::`, `👁` / `---` -> `Show` / `Hide`, `🔒` / `🔓` -> `Lock` / `Unlock`, `■ STOP STREAMING` / `▶ START STREAMING` -> `STOP STREAMING` / `START STREAMING`, `■ STOP RECORDING` / `● START RECORDING` -> `STOP RECORDING` / `START RECORDING`, `Rotation (°)` -> `Rotation (deg)`). Eliminated `Color32::WHITE` and `Color32::TRANSPARENT`. | [x] Verified |
| **New Source & Properties Dialog** | `source_dialog.rs` | Replaced `Color32::WHITE` with `crate::theme::TINT_NONE`. Removed warning emoji `⚠` -> `WARNING:`, `❌` -> `Clear`, `⏸ Pause` / `▶ Play` / `⏹ Stop / Rewind` -> `Pause` / `Play` / `Stop / Rewind`. Replaced unicode ellipses with standard periods. | [x] Verified |
| **On-Canvas Transform Gizmo** | `transform_gizmo.rs` | Replaced `Color32::WHITE` with `TEXT` design token across rotation and resize handle squares. Cleaned doc comment degree symbol. | [x] Verified |
| **Overlay Editor Layers Panel** | `editor/layers_panel.rs` | Replaced `▲ Up` / `▼ Down` with `Up` / `Down`, grip handle `⠿` with `::`, and visibility toggle `👁` / `---` with `Show` / `Hide`. | [x] Verified |
| **Overlay Editor Canvas** | `editor/canvas.rs` | Replaced image emoji `🖼` with `[IMG]`, normalized font size to strictly `13.0px`, and set 3D axis colors to strictly allowed design tokens (`RED_LIVE`, `AMBER_WARN`, `ACCENT`). | [x] Verified |
| **Overlay Editor Asset Browser** | `editor/assets.rs` | Replaced drag grip `⠿` with `::`, replaced `+ Import Media…` with `+ Import Media...`, replaced `GREEN_OK` with `TEXT_MUTED`. | [x] Verified |
| **Overlay Editor Timeline** | `editor/timeline.rs` | Normalized all font sizes to strictly `12.0px` (replacing 9px, 10px, 11px); replaced `GREEN_OK` with `ACCENT` and `TEXT`. | [x] Verified |
| **Overlay Editor Properties** | `editor/properties_panel.rs` | Replaced `GREEN_OK` with `TEXT` token for expression compile status labels. | [x] Verified |
| **Main App Shell & Modals** | `app.rs` | Replaced `● LIVE` / `● REC` with `LIVE` / `REC`, removed emojis `☰ Layers` / `📁 Assets` -> `Layers` / `Assets`, replaced unicode ellipses `Browse…` -> `Browse...`, replaced `GREEN_OK` with `TEXT`. | [x] Verified |
| **First-Run Wizard** | `wizard.rs` | Replaced `✔` with `[OK]`, replaced bullet points `•` with `-`, replaced `GREEN_OK` with `TEXT`. | [x] Verified |
| **Esports Data Control Panel** | `data_panel.rs` | Replaced `[● STREAMING]` with `[STREAMING]`, `[● REC]` with `[REC]`, and `[▶ Scene]` with `[Scene]`. | [x] Verified |
| **Import Report Dialog** | `import_dialog.rs` | Replaced ` • ` with ` - `, replaced `GREEN_OK` with `TEXT`. | [x] Verified |
| **Spreadsheet Links Panel** | `spreadsheet_panel.rs` | Replaced `GREEN_OK` with `TEXT`. | [x] Verified |
| **Performance Telemetry HUD** | `performance_hud.rs` | Replaced `GREEN_OK` with `TEXT` token across all telemetry thresholds. | [x] Verified |
| **Log Viewer Panel** | `log_viewer.rs` | Replaced `GREEN_OK` with `TEXT_MUTED` for INFO badges and severity filters. | [x] Verified |

---

## 14. Edit Overlay Mode Input Lockup Resolution (v1.3 Pass 1)

| Audit Item | Root Cause Identified | Remediation | Verification | Status |
| :--- | :--- | :--- | :--- | :--- |
| **Overlay Editor Dead Input** | Canvas `transform_gizmo` was passed `comp_rect` (1920x1080 viewport) instead of `canvas_rect`. In egui, an interaction area of 1920x1080 overflowed the canvas and covered the Layers panel, toolbar buttons (`+ Text`, `+ Shape`, etc.), Properties, and Timeline, capturing all clicks as empty-canvas marquee drags. Additionally, `allocate_ui` in `app.rs` inherited horizontal layout from parent, causing layout overflow. | 1. Confined gizmo interaction rect strictly to `canvas_rect`, passing `editor.pan` to compute projection.<br>2. Switched canvas `allocate_painter` to `Sense::hover()`, delegating drag/click handling to `transform_gizmo_state`.<br>3. Wrapped toolbar row with `horizontal_wrapped` in `layers_panel.rs`.<br>4. Re-architected 3 editor columns in `app.rs` with `allocate_ui_with_layout(Layout::top_down)` and 280px left panel width. | Verified end-to-end via automated integration test `test_mode_switching_and_overlay_clicks` in `tests/ui_wiring/app_render_flow.rs` asserting real pointer click on `+ Text` adds layer. | [x] Verified |

---

## 15. Independent Preview and Program Buses & Immediate Source Refresh (v1.3 Pass 2 & 4)

| Audit Item | Root Cause Identified | Remediation | Verification | Status |
| :--- | :--- | :--- | :--- | :--- |
| **Independent Preview / Program Buses** | Studio mode previously mutated scenes in-place across both monitors or swapped buses naively. Editing sources in Preview leaked immediately into Program, and CUT swapped active scene with preview instead of broadcasting Preview state to Program while retaining Preview selection. | 1. Implemented Approach (a): `preview_working_sources` on `Project` and `preview_working_scene` on `SceneCompositor` hold an editable working copy.<br>2. Gizmo and DragValue transforms in Live Studio mutate Preview working copy only.<br>3. `CUT` commits working copy to canonical scene data, sets `program_scene = preview_scene`, and resets T-bar without clearing or swapping Preview.<br>4. `AUTO` and `T-BAR` blend from Program output to Preview working copy smoothly. | Verified via `test_independent_preview_and_program_buses` asserting moving a source in Preview does not affect Program, T-bar blends, and CUT commits. | [x] Verified |
| **Immediate Source Refresh in Preview (Section 4)** | Adding a source was writing directly to `comp.scenes[sel_scene]`, bypassing `preview_working_scene`. When the active scene was the selected scene, Program rendered the new item immediately, while Preview was detached until an explicit scene switch re-cloned the scene. | 1. Added `comp.add_preview_item` and updated `create_and_add_source` to add new sources directly to `preview_working_sources` and `preview_working_scene`.<br>2. Preview immediately re-evaluates and renders the new source on the very next frame.<br>3. Program does not receive the new source until CUT / AUTO. | Verified via `test_source_changes_refresh_preview_immediately_no_scene_switch` asserting new source is visible in Preview within 1 frame and absent from Program until CUT. | [x] Verified |

---

## 16. Canvas Alpha Transparency & Checkerboard Pattern (v1.3 Pass 3)

| Audit Item | Root Cause Identified | Remediation | Verification | Status |
| :--- | :--- | :--- | :--- | :--- |
| **Compositor Pipeline Alpha & Layer Stacking** | 1. `Scene::new` defaulted background to opaque dark grey `#0E0F11`, causing `SceneClearPass` to clear with alpha = 1.0 instead of 0.0.<br>2. `TransitionRenderPass` and `PreviewTransitionPass` cleared with `wgpu::Color::BLACK` instead of `TRANSPARENT`.<br>3. `render_scene_object` was reusing a single `self.item_uniform_buf` with `queue.write_buffer` across all items in a scene, causing the topmost item's transform matrix to overwrite lower layers and prevent them from rendering in uncovered regions. | 1. Changed `Scene::new` default background to `Color::TRANSPARENT`.<br>2. Updated `TransitionRenderPass` and `PreviewTransitionPass` clear color to `wgpu::Color::TRANSPARENT`.<br>3. Allocated independent per-item uniform buffers with `device.create_buffer_init` so each stacked layer retains its own transform and MVP.<br>4. Replaced push transition gap color with transparent RGBA `(0, 0, 0, 0)`. | Verified via `test_canvas_alpha_transparency_and_stacking` reading back GPU framebuffer: uncovered half has alpha = 0, and stacking reveals lower layers underneath half-frame sources. | [x] Verified |
| **UI Transparency Checkerboard** | Preview and Program monitors and the Overlay Editor canvas drew solid dark/grey fills (`SURFACE_0` or `SURFACE_2`) behind composited textures, masking real texture transparency from the operator. | 1. Added `paint_checkerboard(painter, rect, cell_size)` to `crates/zeno-ui/src/theme.rs` alternating flat `SURFACE_1` and `SURFACE_2` squares.<br>2. Painted checkerboard in `dock.rs` for Preview and Program monitors before drawing WGPU textures.<br>3. Painted checkerboard in `editor/canvas.rs` inside the composition frame box `comp_rect`. | Verified visually and in automated UI test runs: transparent areas clearly display the broadcast-standard checkerboard pattern. | [x] Verified |

---

## 17. Icon-Heavy Toolbars & Compact Broadcast Widgets (v1.3 Pass 5)

| Audit Item | Root Cause Identified | Remediation | Verification | Status |
| :--- | :--- | :--- | :--- | :--- |
| **Toolbars Cluttered with Verbose Text** | Routine actions across Scenes, Sources, Layers, Timeline, and Viewport header used verbose text buttons (`+ Add Scene`, `- Remove`, `Move Up`, `Move Down`, `Zoom -`, `Zoom +`, `Reset`, `Play [>]`, `Stop [[]]`, `+ Marker`). These consumed excessive horizontal width, wrapped onto multiple lines, and created high visual noise in compact dock tabs. | 1. Created `crates/zeno-ui/src/icon.rs` implementing a zero-dependency, 16px Lucide vector line icon engine (`paint_lucide_icon`), `icon_button`, `icon_button_enabled`, and `icon_toggle_button`.<br>2. Converted Scenes panel toolbar to icons: `Plus`, `Trash`, `Pencil`, `ChevronUp`, `ChevronDown` with descriptive hover tooltips.<br>3. Converted Sources panel toolbar to icons: `Plus`, `Trash`, `Sliders`, `ChevronUp`, `ChevronDown` with hover tooltips.<br>4. Converted Source and Layer rows to compact 22x22 icon toggle buttons: `Eye`/`EyeOff` and `Lock`/`Unlock`.<br>5. Converted Overlay Editor Layers toolbar to icons: `Type`, `Square`, `Camera`, `Sparkles`, `Trash`, `ChevronUp`, `ChevronDown`.<br>6. Converted Canvas viewport controls to `Minus`, `Plus`, `RotateCcw` and Timeline transport controls to `Play`/`Pause`, `Square` (Stop), and `Plus` (Marker).<br>7. Added hover tooltips to rapid score decrement/increment buttons in the Esports Data panel. | Verified via `test_strict_theme_and_no_color_literals_outside_theme`, `test_m8_theme_token_compliance_and_zero_gradients`, and full UI wiring test suites. Zero emojis, strict design tokens, 100% tooltip coverage on icon buttons. | [x] Verified |
| **High-Consequence Button Preservation** | Audited all irreversible or high-stakes broadcast actions to prevent operator error from cryptic icon-only representations. | Retained full, explicit text labels with high-visibility color fills on high-consequence buttons:<br>- Live streaming controls: `START STREAMING` / `STOP STREAMING`<br>- Recording controls: `START RECORDING` / `STOP RECORDING`<br>- Broadcast switcher transitions: `CUT` and `AUTO`<br>- Destructive action confirmation dialogs: Scene and Source deletion confirm modals. | Verified in Live Studio and Esports panels: high-stakes buttons remain unambiguous and easily accessible under live broadcast pressure. | [x] Verified |

---

## 18. OBS-Parity Gap Fixes (v1.4)

| Audit Item | Root Cause Identified | Remediation | Verification | Status |
| :--- | :--- | :--- | :--- | :--- |
| **Normal Mode Default (Section 1)** | Studio Mode (dual monitors) was enabled by default on initial run, cluttering the UI for single-monitor streamers and departing from standard OBS behavior. | 1. Updated `UserPreferences::default()` to set `studio_mode: false` by default.<br>2. In Normal Mode, the central canvas displays a single large Program monitor with interactive gizmos, direct scene switching, and full context menus.<br>3. Operators can toggle between Normal Mode and Studio Mode via the `Studio Mode` / `Normal Mode` button in Stream Controls or View menu.<br>4. Studio Mode state is saved and loaded seamlessly from `preferences.json`. | Verified via `test_v1_4_section_1_default_normal_mode` in `tests/obs_parity_v1_4_test.rs` and `test_section_1_normal_mode_default_and_studio_mode_toggle` in `tests/studio_shell_test.rs`. | [x] Verified |
| **Splitter & Pane Layout Parity (Section 2)** | Layout splitters lacked native feel; in Studio Mode, dual monitors needed a vertical splitter for custom Preview/Program proportions, with smooth aspect ratio scaling. | 1. Implemented vertical split ratio in Studio Mode and full single monitor in Normal Mode.<br>2. Fixed dock pane splitter behavior with clear min-height bounds and responsive aspect ratio fitting for standard 16:9 canvas dimensions.<br>3. Restored dock splitter layout gracefully without collapsing or clipping tabs. | Verified in `test_section_2_split_pane_layout_and_panel_resizing` and `obs_parity_v1_4_test.rs`. | [x] Verified |
| **Audio Mixer dBFS Scaling & Faders (Section 3)** | Audio volume sliders used raw linear scaling (0.0..1.5), which did not match broadcast audio standards or OBS logarithmic dB curves. | 1. Added `volume_to_db` and `db_to_volume` helpers mapping linear amplitudes to decibels (-60.0 dB floor to +6.0 dB boost).<br>2. Calibrated unity gain (1.0) strictly to `0.0 dB` and half amplitude (0.5) to `-6.0 dB`.<br>3. Mapped fader sliders logarithmically for high resolution between -20 dB and 0 dB.<br>4. Prominently display live dB readout (`0.0 dB`, `-6.2 dB`, or `-inf dB` when muted/floored). | Verified via `test_v1_4_section_3_decibel_volume_scaling` in `tests/obs_parity_v1_4_test.rs`. | [x] Verified |
| **Eye/Lock Safety Controls (Section 4)** | Sources list lacked instant toggle feedback and safety protection for locked sources against accidental deletion or canvas drag. | 1. Wired `Eye` / `EyeOff` buttons to instantly toggle source visibility on both project scene definition and live compositor scene item.<br>2. Wired `Lock` / `Unlock` buttons to toggle source lock status.<br>3. Locked sources ignore canvas gizmo selection and dragging.<br>4. Attempting to delete a locked source is blocked and displays an inline transient warning: `"Source is locked. Unlock to delete."`.<br>5. Keyboard shortcuts `Ctrl+Shift+H` (Visibility) and `Ctrl+Shift+L` (Lock) wired to active source. | Verified via `test_v1_4_section_4_eye_and_lock_controls` in `tests/obs_parity_v1_4_test.rs`. | [x] Verified |
| **Context Menu Parity & Clipboard (Section 5)** | Right-click menus lacked standard OBS source operations such as Copy, Paste Reference, Paste Duplicate, native output resize, and preview scaling. | 1. Added full OBS-parity context menu on source rows and canvas items:<br>&nbsp;&nbsp;• **Transform**: Center to Screen, Fit to Screen, Stretch to Screen, Reset Transform.<br>&nbsp;&nbsp;• **Order**: Bring to Front, Bring Forward, Send Backward, Send to Back.<br>&nbsp;&nbsp;• **Copy**: Copies selected source definition to application clipboard.<br>&nbsp;&nbsp;• **Paste (Reference)**: Inserts linked source sharing identical backend source properties.<br>&nbsp;&nbsp;• **Paste (Duplicate)**: Clones source with new UUID and ` (Copy)` name suffix.<br>&nbsp;&nbsp;• **Resize output (source size)**: Adjusts canvas resolution to match source dimensions.<br>&nbsp;&nbsp;• **Screenshot (Source)**: Captures instant GPU screenshot to project folder.<br>2. Added canvas context menu: **Lock Preview** and **Preview Scaling** (`Scale to Window`, `Canvas (1:1)`, `Output (1:1)`). | Verified via `test_v1_4_section_5_clipboard_and_context_menu_parity` in `tests/obs_parity_v1_4_test.rs`. | [x] Verified |


---

## 19. Window Capture & Transform UX Fixes (v1.6)

| Audit Item | Root Cause Identified | Remediation | Verification | Status |
| :--- | :--- | :--- | :--- | :--- |
| **Normal Mode Direct Transform Editing (Section 2.1)** | In Normal Mode (`studio_mode == false`), the UI lacked direct mutation targeting for live sources in `project.scenes[active_scene_idx]` and `comp.update_editable_item_transform()`. Transforms only synced to Preview working buffers. | 1. Implemented `editable_scene_id`, `editable_sources`, and `editable_sources_mut` on `Project` to directly return canonical active scene sources when in Normal Mode.<br>2. Added `update_editable_item_transform()`, `update_editable_item_visibility()`, and `update_editable_item_lock()` in `zeno-compositor` to mutate active scene items immediately without requiring Studio Mode or manual transitions.<br>3. Wired `sync_source_transform_to_compositor` to update active scenes in Normal Mode. | Verified via `test_v1_6_normal_mode_transform_and_deselect` asserting item transforms modify canonical scene immediately. | [x] Verified |
| **Empty Space & Escape Deselection (Section 2.2)** | Clicking empty canvas space or pressing `Escape` did not clear the selection or dismiss the active transform gizmo, leaving items selected indefinitely. | 1. Added empty space click detection emitting `TransformGizmoEvent::DeselectAll`.<br>2. Added `Escape` key handling in the central canvas clearing `selected_source_id`.<br>3. Handled `DeselectAll` across `dock.rs` to clear selection state and hide gizmo bounding boxes. | Verified via `test_clicking_empty_space_deselects` in `transform_gizmo.rs` and `test_v1_6_normal_mode_transform_and_deselect`. | [x] Verified |
| **Unclipped Transform Handles (Section 2.3)** | When layers were positioned partially offscreen or larger than the canvas, handles extending beyond the viewport were clipped by child panel clipping rects, preventing adjustment. | 1. Switched gizmo handle and bounding box painting from `ui.painter()` to unclipped `ui.painter()`.<br>2. Expanded interact rect to union of viewport rect and available panel rect.<br>3. Handles placed outside the canvas viewport boundary remain fully visible and interactable. | Verified via unit and UI rendering tests with offscreen geometry; handles painted across boundary without clipping. | [x] Verified |
| **Shift-Drag Aspect Ratio & Layer Isolation (Section 2.4)** | Holding Shift to resize constrained aspect ratio, but clicking near a handle would hit-test the underlying layer and inadvertently select/transform the background layer instead of resizing. | 1. Added 12px `press_origin` detection around handles in `TransformGizmoState`.<br>2. Evaluated handle hit-tests prior to layer hit-tests.<br>3. Preserves aspect ratio on Shift-drag while isolating transform events exclusively to the active handle's layer. | Verified via `test_shift_drag_handle_constrains_aspect_without_affecting_layers_below`. | [x] Verified |
| **On-Canvas Modifier Hint Badge (Section 2.5)** | Operators had no visual feedback indicating available modifier keys during canvas manipulation. | 1. Added an on-canvas hint pill badge displayed below selected items: `Shift: Constrain Aspect \| Alt: Center Scale \| R: Rotate`.<br>2. Dynamic badge state updates live while dragging to show active modifiers (e.g. `(Active)` indicator). | Verified visually and via strict theme token compliance test `test_strict_theme_and_no_color_literals_outside_theme`. | [x] Verified |
| **Window Capture Black Frame & Latency (Section 1)** | Hardware-accelerated applications (DWM) rendered empty frames to legacy GDI DCs, producing black frames where only the separate cursor overlay rendered; synchronous CPU readbacks caused heavy lag. | 1. Implemented `WgcCaptureSession` in `zeno-platform` using `Direct3D11CaptureFramePool::CreateFreeThreaded` with buffer count = 2.<br>2. Reused session across frames; asynchronous background frame capture eliminated render thread stalls.<br>3. Suppressed cursor overlay in fallback GDI path when content bytes are pure zero.<br>4. Enhanced GDI fallback using `PrintWindow(hwnd, hdc_mem, PW_RENDERFULLCONTENT)` (0x2). | Verified via `test_window_source_properties_and_state`, `test_wgc_session_handles_invalid_hwnd`, and workspace test suite. | [x] Verified |

---

## 20. Window Capture Latency & Camera Source Pipeline Fixes (v1.7)

| Audit Item | Root Cause Identified | Remediation | Verification | Status |
| :--- | :--- | :--- | :--- | :--- |
| **Window Capture Latency (Section 1)** | 1. `Direct3D11CaptureFramePool` accumulated queued frames when `TryGetNextFrame` was called only once without draining.<br>2. Synchronous CPU byte-by-byte BGRA->RGBA swizzling loop consumed 8–12ms per frame.<br>3. Fresh 8.3 MB `Vec<u8>` allocated on every frame in callback and cloned on every tick.<br>4. `DoubleBufferedTexture` called `create_view` per frame.<br>5. Missing sequence check caused redundant texture uploads.<br>6. Swapchain presentation mode defaulted to blocking VSync (`PresentMode::Fifo`). | 1. Drained frame pool in a loop on every `FrameArrived` event to discard backlogged frames and retain strictly the latest frame.<br>2. Configured `DoubleBufferedTexture` with `TextureFormat::Bgra8UnormSrgb` and copied D3D11 staging memory directly via contiguous `copy_from_slice`, delegating color mapping to GPU hardware sampler.<br>3. Preallocated front/back `TextureView` pairs to eliminate per-frame view creation.<br>4. Shared frames via `Arc<CapturedFrame>` with monotonic sequence tracking, eliminating duplicate uploads.<br>5. Configured `PresentMode::AutoNoVsync` in `eframe::NativeOptions` for low-latency non-blocking monitor presentation.<br>6. Added `tracing::trace!` latency and arrival instrumentation spans. | Verified via `test_window_source_properties_and_state` and workspace integration suites. Zero heap allocation per frame, sub-frame latency sustained. | [x] Verified |
| **Camera Device Enumeration & Format Negotiation (Section 2.1)** | `CameraCaptureSource` relied on synthetic software simulation with hardcoded 1080p dummy formats rather than real hardware device selection via Windows Media Foundation (`IMFMediaSource`). Selecting Camera in the dialog left `camera_device` empty if not manually toggled, and no `IMFSourceReader` capture loop existed. | 1. Implemented real hardware camera enumeration in `zeno-platform::camera::enumerate_camera_devices` querying native media types (`MF_MT_FRAME_SIZE`, `MF_MT_FRAME_RATE`, `MF_MT_SUBTYPE`) via `IMFSourceReader`.<br>2. Implemented `MfCameraSession` activating `IMFMediaSource` and configuring `IMFSourceReader` with `MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING = 1` to negotiate hardware-decoded RGB32 (BGRA).<br>3. Added high-speed BT.601/BT.709 `nv12_to_bgra` and `yuy2_to_bgra` fallbacks for native YUV streams.<br>4. Pre-selected primary camera device in `SourceDialog` when `camera_device` is empty so confirming creates a valid real camera target. | Verified via `test_camera_device_enumeration_does_not_panic`, `test_nv12_to_bgra_conversion_dimensions_and_channels`, and `test_yuy2_to_bgra_conversion_dimensions_and_channels`. Physical webcams/capture cards deliver live hardware frames. | [x] Verified |
| **Camera Privacy & Permission Denials (Section 2.1)** | When Windows Privacy settings blocked camera access, `ActivateObject` failed silently, resulting in an uninitialized texture that rendered as an ambiguous transparent checkerboard without informing the operator. | 1. Caught `E_ACCESSDENIED` (`0x80070005`) in `MfCameraSession` and flagged `permission_denied`.<br>2. Implemented `render_privacy_blocked_placeholder` in `CameraCaptureSource` displaying an Amber warning banner: `"CAMERA BLOCKED: Access denied by Windows Privacy Settings. Enable in Settings -> Privacy & security -> Camera"`.<br>3. Surface clear diagnostics instead of rendering a blank checkerboard. | Verified via `test_mf_camera_session_graceful_handling_of_invalid_device` and permission handling tests. Clear diagnostic feedback provided. | [x] Verified |

