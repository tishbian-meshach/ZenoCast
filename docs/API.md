# Zenocast Remote Control API

Zenocast exposes a high-performance HTTP and WebSocket control server on `127.0.0.1:4466` for remote control, tally indicators, Bitfocus Companion integrations, Stream Deck profiles, and automated scoreboards.

---

## 1. Authentication

Authentication is optional and can be enabled by setting a bearer token in studio settings or environment variables (`ZENOCAST_API_TOKEN`).

When enabled, provide the token either as an HTTP Authorization header:
```http
Authorization: Bearer <your_token>
```
or as a URL query parameter:
```
http://127.0.0.1:4466/api/v1/status?token=<your_token>
```

---

## 2. HTTP REST Endpoints

### 2.1 Studio Health and Status
- **Method:** `GET`
- **Route:** `/api/v1/status` (or `/status`)
- **Response:**
  ```json
  {
    "status": "ok",
    "version": "0.1.0",
    "current_scene": "Live Match",
    "is_streaming": true,
    "is_recording": false,
    "store_keys": 42
  }
  ```

### 2.2 Scene Listing
- **Method:** `GET`
- **Route:** `/api/v1/scenes` (or `/scenes`)
- **Response:**
  ```json
  {
    "scenes": [
      "Starting Soon",
      "Live Match",
      "Caster Desk",
      "Break",
      "Ending"
    ],
    "active_scene": "Live Match"
  }
  ```

### 2.3 Switch Scene
- **Method:** `POST`
- **Route:** `/api/v1/scene/switch` (or `/scene/switch`)
- **Payload:**
  ```json
  {
    "scene_name": "Caster Desk",
    "transition": "fade"
  }
  ```
- **Example cURL:**
  ```bash
  curl -X POST http://127.0.0.1:4466/api/v1/scene/switch \
    -H "Content-Type: application/json" \
    -d "{\"scene_name\": \"Caster Desk\"}"
  ```

### 2.4 Update Live Data Store
- **Method:** `POST`
- **Route:** `/api/v1/data/set` (or `/data/set`)
- **Payload:**
  ```json
  {
    "path": "scoreboard.team1.score",
    "value": 15
  }
  ```
- **Example cURL:**
  ```bash
  curl -X POST http://127.0.0.1:4466/api/v1/data/set \
    -H "Content-Type: application/json" \
    -d "{\"path\": \"scoreboard.team1.score\", \"value\": 15}"
  ```
- *Note:* Any on-air motion-graphics overlay bound to `{scoreboard.team1.score}` immediately updates within the next render frame.

### 2.5 Query Data Value
- **Method:** `GET`
- **Route:** `/api/v1/data/get?path=scoreboard.team1.score` (or `/data/get?path=...`)
- **Response:**
  ```json
  {
    "status": "ok",
    "path": "scoreboard.team1.score",
    "value": 15
  }
  ```

### 2.6 Trigger Overlay Animation
- **Method:** `POST`
- **Route:** `/api/v1/overlay/trigger` (or `/overlay/trigger`)
- **Payload:**
  ```json
  {
    "overlay_id": "lower_third",
    "action": "play_in"
  }
  ```
- Actions: `play_in`, `play_out`, `toggle`.

### 2.7 Stream and Record Control
- `POST /stream/start` – Initiates RTMP live broadcast.
- `POST /stream/stop` – Concludes RTMP broadcast.
- `POST /record/start` – Begins MKV local capture.
- `POST /record/stop` – Stops local recording and remuxes to MP4.

---

## 3. WebSocket Real-Time Push Events

Connect to:
```
ws://127.0.0.1:4466/ws
```

Zenocast streams JSON events to all connected clients:

1. **Data Change:**
   ```json
   {
     "event": "data_change",
     "data": {
       "path": "scoreboard.team1.score",
       "value": 15
     }
   }
   ```
2. **Scene Change:**
   ```json
   {
     "event": "scene_change",
     "data": {
       "scene": "Caster Desk"
     }
   }
   ```
3. **Stream Status:**
   ```json
   {
     "event": "stream_status",
     "data": {
       "streaming": true
     }
   }
   ```

---

## 4. Bitfocus Companion & Stream Deck Setup

1. In Bitfocus Companion, add an **HTTP: Generic** connection targeting `http://127.0.0.1:4466`.
2. Configure button actions:
   - **Score +1:** `POST /api/v1/data/set` with `{"path": "scoreboard.team1.score", "value": ...}`
   - **Switch to Casters:** `POST /api/v1/scene/switch` with `{"scene_name": "Caster Desk"}`
   - **Toggle Lower Third:** `POST /api/v1/overlay/trigger` with `{"overlay_id": "lower_third", "action": "toggle"}`
