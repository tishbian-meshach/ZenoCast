# Zenocast Esports Broadcast Templates & Scene Presets

Zenocast ships with a production-ready suite of 14 native `.arx` esports templates located in `templates/esports/`. Designed specifically for live tournament operations (Valorant, CS2, League of Legends, Rocket League), these templates feature dedicated In, Loop, and Out animations and automatic bindings to the live reactive `DataStore`.

---

## 1. Included Template Catalog

| File | Category | Description | DataStore Bindings |
| :--- | :--- | :--- | :--- |
| `starting_soon.arx` | Pre-Show | 1080p60 countdown timer with match title and animated sponsor carousel. | `match.title`, `timer.countdown` |
| `be_right_back.arx` | Intermission | Ambient animated background with particle embers and upcoming schedule. | `schedule.next_match` |
| `stream_ending.arx` | Post-Show | Match recap, final series score, MVP spotlight, and social credits. | `match.mvp`, `match.winner` |
| `scoreboard_bo1.arx` | Match HUD | Single-game scoreboard header with team logos, names, and scores. | `team1.name`, `team1.score`, `team2.name`, `team2.score` |
| `scoreboard_bo3.arx` | Match HUD | Best-of-3 HUD featuring map score pips and active map name. | `match.bo_format`, `team1.maps`, `team2.maps`, `match.map_name` |
| `scoreboard_bo5.arx` | Match HUD | Grand Finals Best-of-5 HUD with multi-round score indicators. | `team1.maps`, `team2.maps`, `team1.round_score`, `team2.round_score` |
| `lower_third.arx` | Talent | Clean broadcast lower-third for caster and player introductions. | `talent.name`, `talent.title`, `talent.handle` |
| `caster_desk_2.arx` | Desk Setup | Dual-caster picture-in-picture layout with lower nameplates. | `caster1.name`, `caster2.name` |
| `caster_desk_3.arx` | Desk Setup | 3-analyst desk layout with integrated lower third. | `desk.analyst1`, `desk.analyst2`, `desk.analyst3` |
| `caster_desk_4.arx` | Desk Setup | 4-person broadcast desk layout with individual name tags. | `desk.host`, `desk.analyst1`, `desk.analyst2`, `desk.analyst3` |
| `player_cam_frames.arx` | In-Game | Camera overlay frames with dynamic team color borders. | `team1.color`, `team2.color` |
| `map_veto.arx` | Pre-Match | Pick and ban map veto screen with dynamic status tags (Picked / Banned / Decider). | `veto.map1..7`, `veto.status1..7` |
| `kill_feed.arx` | In-Game | Dynamic match event banner and round winner notification. | `event.headline`, `event.subtitle` |
| `tournament_bracket.arx`| Playoffs | 8-team single-elimination tournament bracket graphic with live advancement. | `bracket.quarterfinals`, `bracket.semifinals`, `bracket.finals` |

---

## 2. In / Loop / Out Lifecycle Markers

Every Zenocast broadcast template defines three distinct animation phases in `scene.json`:

1. **In (Intro Animation)**:
   - Evaluates from $t = 0.0$ to $t = \text{in\_duration}$ (e.g. 0.5s to 1.0s).
   - Slides, fades, or expands geometry into active broadcast position.
2. **Loop (Sustained State)**:
   - When the intro completes, the template enters a seamless continuous loop.
   - Ideal for subtle glass reflections, particle glows, sponsor ticker rotations, and live score updates.
3. **Out (Exit Animation)**:
   - Triggered when the operator cuts or transitions away from the scene or turns off the overlay source.
   - Smoothly clears the graphic from air without harsh cuts.

---

## 3. One-Click Tournament Scene Generator

Zenocast provides the `TournamentPresetGenerator` (`zeno_compositor::TournamentPresetGenerator`) to scaffold an entire broadcast production setup in seconds:

1. **Countdown / Starting Soon**: Loads `starting_soon.arx` over an ambient animated particle background.
2. **Talent Desk**: Configures camera capture devices into `caster_desk_2.arx` with lower third automation.
3. **Map Veto Desk**: Loads `map_veto.arx` linked to spreadsheet data.
4. **Live Gameplay**: Composes gameplay screen capture, player webcams (`player_cam_frames.arx`), and `scoreboard_bo3.arx`.
5. **Replay & Highlight**: Fullscreen game feed with animated "REPLAY" stinger wipe transition.
6. **Post-Match Desk**: Transitions to final bracket (`tournament_bracket.arx`) and `stream_ending.arx`.

### Using in Code or Control API
```rust
use zeno_compositor::TournamentPresetGenerator;

// Generates 6 broadcast-ready scenes with configured transition rules
let scenes = TournamentPresetGenerator::generate_standard_preset("VCT Grand Final");
```
Or via HTTP API:
```bash
POST /api/v1/scenes/preset
Content-Type: application/json

{ "preset": "tournament_standard", "title": "VCT Grand Final" }
```
