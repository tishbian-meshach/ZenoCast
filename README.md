# Zenocast

An esports broadcast studio: OBS-style live production plus a built-in motion-graphics editor with its own file format (`.arx`). Everything is edited and streamed from one app, with no export step. What you edit is exactly what goes on air.

## Features

- **Live Production**: Display/window capture, camera feeds, scene compositor, configurable transitions, 6-track audio mixer, hardware video encoding, RTMP/SRT streaming, and direct YouTube integration.
- **Overlay & Screen Editor**: Motion-graphics authoring with `.arx` container format, keyframes, bezier curves, separate In/Loop/Out lifecycles, shape vectors, text shaping, GPU particles, and Rhai expressions.
- **Esports Control Layer**: Live data store, scoreboard controllers, automated roster and match data bindings, spreadsheet synchronization (Excel, CSV, Google Sheets), and remote control HTTP/WebSocket API.
- **Licensing & Platform Security**: Offline-first Ed25519 signature verification, seat management, and binary integrity protections.

## Architecture

Zenocast is built as a modular multi-crate Rust workspace targeting Windows 10/11 x64:

- `zeno-core`: Primitive types, math, time, and `Property<T>`.
- `zeno-engine`: Pure scene graph, evaluation pipeline, and render list generation.
- `zeno-format`: `.arx` zip container pack/unpack and migrations.
- `zeno-import`: PSD and Lottie file importers.
- `zeno-script`: Sandboxed Rhai expression runtime.
- `zeno-data`: Live reactive data store and spreadsheet bindings.
- `zeno-media`: Video capture devices, screen capture, media playback.
- `zeno-audio`: WASAPI audio engine, filters, and mixer.
- `zeno-compositor`: Real-time GPU scene compositor and screen transition system.
- `zeno-output`: Hardware encoders (NVENC, AMF, QSV, x264) and RTMP/SRT muxer.
- `zeno-control`: HTTP/WebSocket API and global hotkey control.
- `zeno-platform`: Windows API bindings and hardware abstractions.
- `zeno-license`: License validation, integrity self-checks, and device fingerprinting.
- `zeno-ui`: Egui broadcast control room and motion-graphics editor.
- `zeno-app`: Live studio desktop executable.
- `zeno-cli`: Headless rendering and validation command-line tool.

## Building and Setup

Refer to [SETUP.md](docs/SETUP.md) for toolchain prerequisites and build instructions.
