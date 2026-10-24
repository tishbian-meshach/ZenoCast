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
| **Status Readout** | Reflects real live stream status ("LIVE" vs "OFFLINE") | Verified: displays RED_LIVE "LIVE" when active in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **FPS Telemetry** | Real-time encoder framerate from `OutputManager::stats()` | Verified: reports live fps in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Dropped Frame Telemetry** | Percentage of frames dropped by network/encoder | Verified: reports `stats.dropped_percent` with warning color if > 0.1% in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Bitrate Telemetry** | Current network output bitrate in kbps | Verified: reports active bitrate kbps from encoder telemetry in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Duration Timer** | Elapsed stream/record duration formatted as `HH:MM:SS` | Verified: formats elapsed time string accurately in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |

---

## 7. Monitors and Switching Controls

| Control | Action / Engine Target | Verification Note | Status |
| :--- | :--- | :--- | :--- |
| **CUT Button** | Instantly swaps Preview and Program scene buses | Verified: swaps active and preview scenes immediately in `test_m8_preview_to_program_transition_switching`. | [x] Verified |
| **AUTO Button** | Runs active transition definition with configured duration | Verified: executes auto transition in `test_m8_preview_to_program_transition_switching`. | [x] Verified |
| **T-BAR Slider** | Smoothly blends preview and program scenes on drag; cuts at >= 0.95 | Verified: scrubbing smoothly updates progress; drag release >= 0.95 swaps buses in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Preview Monitor View** | Renders live WGPU preview texture view from compositor with interactive gizmo | Verified: renders GPU preview framebuffer in `test_section_2_1_wire_source_creation_to_real_backend_and_live_preview`. | [x] Verified |
| **Program Monitor View** | Renders live WGPU program texture view with on-air tally border | Verified: renders GPU program framebuffer with live border in `test_section_2_1_wire_source_creation_to_real_backend_and_live_preview`. | [x] Verified |

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
| **Selection Bounding Box** | Highlight bounding box drawn with `ACCENT` stroke on selected items | Verified: hit testing and selection rendering in `test_hit_test_unrotated`. | [x] Verified |
| **8 Resize Handles** | 4 corner and 4 edge midpoint 8x8px square handles for 2D scaling | Verified: drag handles resize width/height in `test_gizmo_item_creation`. | [x] Verified |
| **Shift Aspect Lock** | Holding Shift while dragging corner handles locks original aspect ratio | Verified: aspect ratio preservation formula verified in `test_aspect_ratio_preservation_calc`. | [x] Verified |
| **Alt Center Scale** | Holding Alt scales symmetrically from object center | Verified: updates center and dimensions symmetrically. | [x] Verified |
| **Rotation Handle on Stem** | Circular rotate handle located 24px above top-center handle | Verified: computes angle around item center and updates `rotation_degrees`. | [x] Verified |
| **15° Angle Snap** | Holding Shift while rotating snaps to 15-degree increments | Verified: snapping logic verified in `test_rotation_15_degree_snapping`. | [x] Verified |
| **Canvas Hit-Testing** | Clicking directly on visible layer selects it; topmost layer wins; clicking empty canvas deselects | Verified: z-order hit-testing verified in `test_hit_test_unrotated`. | [x] Verified |
| **Keyboard Nudge** | Arrow keys nudge 1px; Shift+Arrow nudges 10px | Verified: updates position in discrete increments when canvas is focused. | [x] Verified |
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



