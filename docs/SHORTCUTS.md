# Zenocast Keyboard Shortcuts Reference

Complete keyboard shortcut reference for Zenocast Studio, modeled on OBS Studio defaults with Zenocast-specific motion-graphics and live editing capabilities. All shortcuts are fully rebindable via **Settings → Hotkeys** in Zenocast Studio.

> [!TIP]
> Shortcuts respect focus context: when editing text fields or renaming labels, app-level shortcuts do not fire. Global shortcuts (marked with ✓) work even when Zenocast is running in the background when enabled via the "Make global" checkbox.

## 1. Streaming, Recording, and Output

| Action | Default Shortcut | Global Eligible | Description |
| --- | --- | :---: | --- |
| Start Streaming | *(unbound by default)* | ✓ | Starts RTMP/SRT broadcast to configured destinations. |
| Stop Streaming | *(unbound by default)* | ✓ | Halts live broadcast stream cleanly. |
| Start Recording | *(unbound by default)* | ✓ | Begins recording master program output to disk. |
| Stop Recording | *(unbound by default)* | ✓ | Stops active recording and remuxes output if enabled. |
| Pause Recording | *(unbound by default)* | ✓ | Pauses active recording without closing the file. |
| Start/Stop Replay Buffer | *(unbound by default)* | ✓ | Toggles circular instant replay recording buffer. |
| Save Replay Buffer | *(unbound by default)* | ✓ | Saves buffered replay frames to an MP4/MKV clip. |
| Enable/Disable Preview | *(unbound by default)* | ✓ | Toggles Studio Mode Preview display. |

## 2. Scene and Transition Control

| Action | Default Shortcut | Global Eligible | Description |
| --- | --- | :---: | --- |
| Cut (Preview → Program immediately) | *(unbound by default)* | ✓ | Immediately cuts Preview to Program without transition animation. |
| Trigger Transition (Auto) | *(unbound by default)* | ✓ | Executes configured transition (Fade/Slide/Wipe) from Preview to Program. |
| Next Scene | *(unbound by default)* | ✓ | Steps forward through the scenes list. |
| Previous Scene | *(unbound by default)* | ✓ | Steps backward through the scenes list. |
| Toggle Studio Mode | *(unbound by default)* | ✓ | Toggles between Direct Mode and Dual-Bus Studio Mode. |

## 3. Live Studio Editing

| Action | Default Shortcut | Global Eligible | Description |
| --- | --- | :---: | --- |
| Undo | `Ctrl+Z` | — | Reverts the last studio or overlay editor operation. |
| Redo | `Ctrl+Shift+Z` (or `Ctrl+Y`) | — | Reapplies the last undone operation. |
| Delete selected source/layer | `Delete` | — | Removes currently selected source or overlay layer. |
| Duplicate selected source/layer | `Ctrl+D` | — | Duplicates selected source or overlay layer with unique ID. |
| Copy | `Ctrl+C` | — | Copies selected source or layer to internal clipboard. |
| Paste | `Ctrl+V` | — | Pastes copied item with offset position. |
| Paste in place (same position) | `Ctrl+Shift+V` | — | Pastes copied item at its exact original coordinates. |
| Select All (sources in current scene) | `Ctrl+A` | — | Selects all sources in active Preview scene. |
| Deselect All | `Escape` | — | Clears current source or layer selection. |
| Bring to Front | `Ctrl+Shift+]` | — | Moves selected item to top of rendering order. |
| Bring Forward | `Ctrl+]` | — | Moves selected item up one level in stack. |
| Send Backward | `Ctrl+[` | — | Moves selected item down one level in stack. |
| Send to Back | `Ctrl+Shift+[` | — | Moves selected item to bottom of stack. |
| Toggle visibility of selected source | `Ctrl+Shift+H` | ✓ | Hides or shows the currently selected source. |
| Toggle lock of selected source | `Ctrl+Shift+L` | — | Locks or unlocks canvas transforms for selected source. |
| Mute/Unmute selected audio source | `Ctrl+Shift+M` | ✓ | Toggles mute status of selected audio source. |
| Open Properties of selected source | `Alt+Enter` | — | Opens configuration modal for selected source. |
| Rename selected scene/source | `F2` | — | Activates inline rename buffer for selected item. |

## 4. Overlay Editor

| Action | Default Shortcut | Global Eligible | Description |
| --- | --- | :---: | --- |
| Add Text Layer | `T` | — | Inserts a new vector text layer in overlay composition. |
| Add Shape Layer | `R` | — | Inserts a new vector rectangle/shape layer. |
| Add Camera Layer | `Shift+C` | — | Inserts 2.5D perspective camera layer. |
| Add Particles Layer | `Shift+P` | — | Inserts GPU particle emitter layer. |
| Delete selected layer | `Delete` | — | Deletes selected layer from timeline and canvas. |
| Group selected layers | `Ctrl+G` | — | Groups selected layers into a container group. |
| Ungroup | `Ctrl+Shift+G` | — | Unwraps group contents back to parent composition. |
| Add Keyframe on selected property | `Alt+K` | — | Inserts an animated keyframe at current playhead time. |
| Delete Keyframe at playhead | `Alt+Shift+K` | — | Removes keyframe at playhead for selected property. |
| Play/Pause (timeline) | `Space` | — | Toggles playback animation loop. |
| Stop (return to start) | `Shift+Space` | — | Stops playback and returns playhead to start (0.0s). |
| Go to Start | `Home` | — | Moves playhead to time 0.0s. |
| Go to End | `End` | — | Moves playhead to duration boundary. |
| Step one frame back | `,` | — | Steps playhead backward by 1 frame (1/60s). |
| Step one frame forward | `.` | — | Steps playhead forward by 1 frame (1/60s). |
| Set In-point / Loop-start marker at playhead | `I` | — | Sets lifecycle intro/loop start boundary at playhead. |
| Set Loop-end / Out-start marker at playhead | `O` | — | Sets lifecycle loop end/out start boundary at playhead. |
| Add generic Marker at playhead | `M` | — | Places a named cue point on the time ruler. |
| Zoom timeline in | `=` | — | Increases timeline pixel zoom resolution. |
| Zoom timeline out | `-` | — | Decreases timeline pixel zoom resolution. |
| Fit timeline to window | `Shift+F` | — | Scales timeline zoom to fit composition duration. |
| Zoom canvas in | `Ctrl+=` | — | Zooms in canvas viewport center. |
| Zoom canvas out | `Ctrl+-` | — | Zooms out canvas viewport center. |
| Reset canvas zoom to 100% | `Ctrl+0` | — | Resets canvas zoom factor to 1.0 (100%). |
| Fit canvas to window | `Shift+1` | — | Centers and scales composition to fit viewport. |
| Toggle Dope Sheet / Curve Editor view | `Shift+F3` | — | Switches between Dope Sheet and Bezier Curve Editor. |
| Switch to Move/Select tool | `V` | — | Activates selection and transform gizmo tool. |
| Switch to Pan (hand) tool | `H` | — | Activates canvas hand pan tool. |

## 5. Window, Panels, and Modes

| Action | Default Shortcut | Global Eligible | Description |
| --- | --- | :---: | --- |
| Switch to Live Studio mode | `Ctrl+1` | — | Switches application to broadcast Live Studio switcher. |
| Switch to Edit Overlay mode | `Ctrl+2` | — | Switches application to motion graphics Overlay Editor. |
| Switch to Data Control mode | `Ctrl+3` | — | Switches application to Esports Live Data Control panel. |
| Open Settings | `Ctrl+,` | — | Opens Studio Settings modal. |
| New Project | `Ctrl+N` | — | Creates a new blank broadcast project. |
| Open Project | `Ctrl+O` | — | Prompts to open an existing .arxproj project. |
| Save Project | `Ctrl+S` | — | Saves current project state to disk. |
| Save Project As | `Ctrl+Shift+S` | — | Prompts for destination file path to save project. |
| Toggle fullscreen Preview/Program | `F11` | — | Toggles borderless fullscreen monitor display. |
| Close current dialog/modal | `Escape` | — | Closes open modal or dismisses current dialog. |
| Confirm current dialog (primary action) | `Enter` | — | Confirms primary button action in active modal dialog. |

## 6. Per-Scene and Per-Source Hotkeys

Zenocast provides full OBS parity for assigning dedicated hotkeys to individual scenes and sources:

- **Per-Scene Hotkeys**: Right-click any scene in the Scenes panel or select **Hotkeys** in Scene Properties to assign:
  - **Switch to Scene**: Instantly switches to this scene when pressed.
  - **Switch Back to Previous Scene**: Reverts to the previously active scene.
- **Per-Source Hotkeys**: Right-click any source in the Sources panel or open Source Properties to assign:
  - **Mute / Unmute**: Toggles audio mute state for this source.
  - **Push-to-Mute**: Temporarily mutes the source only while the assigned key is held down.
  - **Push-to-Talk**: Unmutes the source only while the assigned key is held down.
  - **Show / Hide**: Toggles on-air visual visibility for this source.

All per-item hotkeys also appear in **Settings → Hotkeys** grouped under their respective scene and source headers, complete with conflict detection and "Make global" support.
