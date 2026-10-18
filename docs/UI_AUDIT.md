# Zenocast UI Interactivity Audit

Every interactive control in the Zenocast esports broadcast suite is audited below. Each control is verified against real application, engine, compositor, and output pipeline state—never mock or isolated visual dummy state.

---

## 1. Top Menu Bar

| Control | Action / Engine Target | Verification Note | Status |
| :--- | :--- | :--- | :--- |
| **File > New Project** | Clears `project`, unregisters sources in `SourceRegistry`, reinitializes default compositor scenes | Verified: `app.reset_to_new_project()` resets scenes count to 2, clears registry count to 0 in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **File > Open Project...** | Opens `.arxproj` modal, deserializes JSON, synchronizes compositor scenes & items | Verified: loads serialized `.arxproj` and restores scene hierarchy in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **File > Save Project** | Serializes current `Project` state to existing project path | Verified: writes valid JSON schema to disk in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **File > Save Project As...** | Prompts for target path and serializes `Project` to disk | Verified: file created at destination and verified non-empty in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
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

---

## 3. Sources Panel

| Control | Action / Engine Target | Verification Note | Status |
| :--- | :--- | :--- | :--- |
| **+ Add Source** | Opens `SourceDialog` for 10 broadcast source types | Verified: adds concrete `Source` trait objects across all 10 types in `test_section_2_2_all_ten_source_types_and_resource_lifecycle`. | [x] Verified |
| **- Remove** | Deletes source from project, compositor, and stops in `SourceRegistry` | Verified: unregisters handle, frees GPU resources, leaves 0 leaked handles in `test_section_2_2_all_ten_source_types_and_resource_lifecycle`. | [x] Verified |
| **Properties** | Re-opens dialog with source properties to update configuration | Verified: updates parameters live in `SourceRegistry` and compositor in `test_section_2_2_all_ten_source_types_and_resource_lifecycle`. | [x] Verified |
| **Visibility (👁)** | Toggles `visible` flag on `SourceDefinition` and compositor `SceneItem` | Verified: flips `item.visible` in compositor scene in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Lock (🔒)** | Toggles `locked` flag on `SourceDefinition` to prevent unintended canvas manipulation | Verified: flips `src_def.locked` in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |

---

## 4. Audio Mixer Panel

| Control | Action / Engine Target | Verification Note | Status |
| :--- | :--- | :--- | :--- |
| **Master Volume Slider** | Multiplies master audio gain (0.0..=1.5) across all active audio tracks | Verified: adjusts `master_volume` and syncs effective gain into `SourceRegistry` in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Master Mute Button** | Silences master output bus and sets dB readout to `-inf dB` | Verified: mutes all channels and displays `-inf dB` in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Channel Volume Sliders** | Adjusts individual source volume and propagates to WASAPI/FFmpeg decoder | Verified: updates `SourceProperties::AudioInput/Output` in `SourceRegistry` in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **Channel Mute Buttons** | Silences individual source and propagates to audio pipeline | Verified: updates source mute state in `SourceProperties` in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
| **dB Readouts** | Displays calculated logarithmic decibel level `20.0 * log10(gain)` | Verified: 0.5 gain produces ~ -6.0 dB, 0.0 or muted produces `-inf dB` in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |

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
| **Preview Monitor View** | Renders live WGPU preview texture view from compositor | Verified: renders GPU preview framebuffer in `test_section_2_1_wire_source_creation_to_real_backend_and_live_preview`. | [x] Verified |
| **Program Monitor View** | Renders live WGPU program texture view with on-air tally border | Verified: renders GPU program framebuffer with live border in `test_section_2_1_wire_source_creation_to_real_backend_and_live_preview`. | [x] Verified |

---

## 8. Window Chrome & Lifecycle

| Control | Action / Engine Target | Verification Note | Status |
| :--- | :--- | :--- | :--- |
| **Window Minimize/Maximize** | Standard OS desktop window control via `eframe` / `winit` | Verified: handled by native window event loop in `zeno-app`. | [x] Verified |
| **Window Close / Clean Shutdown** | Invokes `eframe::App::on_exit()` for leak-free shutdown | Verified: stops streaming, finalizes recording, and cleans `SourceRegistry` in `test_section_3_ui_interactivity_audit_wiring`. | [x] Verified |
