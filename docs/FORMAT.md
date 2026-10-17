# Zenocast `.arx` Container Format Specification

The `.arx` (Arena eXchange) format is Zenocast's native self-contained motion-graphics and overlay container. It bundles vector shapes, text elements, animation timelines, keyframe curves, Rhai expression scripts, 3D glTF models, particle emitters, and embedded media assets into a single distributable file.

---

## 1. Physical Layout (ZIP Archive)

An `.arx` package is a standard ZIP archive using Deflated compression with UTF-8 filename encoding:

```
package.arx
├── manifest.json       # Required: Package metadata, dimensions, framerate, versioning
├── scene.json          # Required: Composition hierarchy, layers, timelines, keyframes
├── thumb.png           # Optional: 256x144 preview thumbnail for asset browsers
└── assets/             # Optional: Embedded binary assets referenced by layers
    ├── team_logo.png
    ├── sponsor_crest.glb
    └── custom_font.ttf
```

---

## 2. Specification of Key Files

### 2.1 `manifest.json`

Metadata describing the package author, canvas geometry, and minimum engine compatibility:

```json
{
  "format_version": 1,
  "name": "VCT Champions Scoreboard",
  "author": "Zenocast Graphics Team",
  "width": 1920,
  "height": 1080,
  "fps": 60.0,
  "engine_min_version": "0.1.0"
}
```

- `format_version`: Integer format version (currently `1`). Used for backward-compatible schema migrations.
- `name`: Human-readable title displayed in the template selector.
- `width` / `height`: Composition canvas dimensions in logical pixels.
- `fps`: Target animation evaluation rate.
- `engine_min_version`: Semver string specifying the minimum compatible Zenocast engine.

---

### 2.2 `scene.json`

The complete serialized `zeno_engine::Composition` scene graph:

```json
{
  "id": "comp_scoreboard_vct",
  "name": "Scoreboard Lower Third",
  "width": 1920.0,
  "height": 1080.0,
  "fps": 60.0,
  "duration_seconds": 10.0,
  "in_out_points": {
    "in_duration": 1.0,
    "out_duration": 1.0,
    "loop_enabled": true
  },
  "variables": [
    { "name": "team1_name", "type": "String", "default": "Sentinels" },
    { "name": "team1_score", "type": "Number", "default": 2 },
    { "name": "team2_name", "type": "String", "default": "Fnatic" },
    { "name": "team2_score", "type": "Number", "default": 1 }
  ],
  "layers": [
    {
      "id": "layer_background",
      "name": "Glass Surface",
      "kind": "Shape",
      "transform": {
        "position": { "x": 960.0, "y": 980.0 },
        "scale": { "x": 1.0, "y": 1.0 },
        "rotation": 0.0,
        "opacity": 0.95
      },
      "shape": {
        "type": "RoundedRect",
        "width": 640.0,
        "height": 72.0,
        "corner_radius": 4.0,
        "fill": [0.082, 0.090, 0.102, 0.95],
        "stroke": [0.298, 0.553, 1.0, 1.0],
        "stroke_width": 1.0
      }
    },
    {
      "id": "layer_team1_text",
      "name": "Team 1 Name",
      "kind": "Text",
      "text": {
        "content": "$team1_name",
        "font_family": "Inter",
        "font_size": 24.0,
        "color": [0.906, 0.914, 0.925, 1.0]
      }
    }
  ]
}
```

---

### 2.3 Layer Kinds Supported

- `Shape`: Vector rectangles, rounded boxes, circles, polygons, path strokes.
- `Text`: Rich text with font selection, weight, tracking, alignment, and data store binding.
- `Image`: Embedded PNG / JPEG textures.
- `Model3D`: Binary glTF 2.0 (`.glb`) with metallic/roughness PBR materials, analytical normals, and continuous rotation.
- `Particle`: GPU-accelerated particle emitters with configurable birth rate, speed, spread, lifetime, and gravity.
- `Browser`: Off-screen web page or HTML5 animated overlay.

---

## 3. Schema Versioning & Migrations

When an `.arx` container with `format_version < CURRENT_FORMAT_VERSION` is opened:
1. `zeno_format::migration::migrate_scene` executes sequential step migrations.
2. Older keyframe curve definitions are converted to normalized cubic Bézier parameters.
3. Missing properties are populated with sensible defaults.
4. When saved, the package is upgraded and written with the latest format version.
