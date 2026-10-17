# Zenocast Licensing Server API Specification

This document defines the HTTP REST API implemented by the `arena-license-server` authority. It manages Ed25519 offline license verification, device activations, heartbeat renewals, multi-seat allocations, and administrative revocations for Zenocast per Section 16 of the Zenocast specification.

---

## 1. Cryptographic Architecture

- **Algorithm**: Ed25519 (RFC 8032) digital signatures over JSON payloads.
- **Key Distribution**: Zenocast clients embed the server's 32-byte Ed25519 public key. Only the license authority server retains the private signing key.
- **Key Format**: Grouped Base32 tokens formatted as `XXXXX-XXXXX-XXXXX-XXXXX-...` containing serialized `LicensePayload` and its Ed25519 signature.
- **Offline First**: The initial key is verifiable offline immediately without network contact. Once activated online, a signed `LicenseCertificate` bound to the client's SHA-256 hardware device fingerprint is stored locally.
- **Heartbeat & Grace Period**: The client re-contacts the server every 24 to 72 hours. If unreachable, the client operates in an offline grace period for up to 14 days before degrading into Restricted mode.

---

## 2. API Endpoints

### 2.1 Online Device Activation

Transfers a purchased license key and machine hardware fingerprint to activate a seat.

- **Endpoint**: `POST /activate`
- **Headers**:
  - `Content-Type: application/json`

#### Request Payload
```json
{
  "key": "A7X9K-MN2PQ-9W8YT-4XZL1-BR7VK-3MDQW",
  "device_fingerprint": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "client_version": "0.1.0"
}
```

#### Response (200 OK)
Returns a signed device-bound certificate:
```json
{
  "license_id": "3b2e5910-8b1b-4f92-95e2-63b51f04cfd1",
  "customer_id": "VCT_EMEA_Production",
  "edition": "Studio",
  "features": 18446744073709551615,
  "expires_at": 1735689600,
  "device_fingerprint": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "issued_at": 1700000000,
  "server_signature": [158, 5, 55, 132, 239, 100, 211, 123, ...]
}
```

#### Error Responses
- `400 Bad Request`: Invalid key format or cryptographic signature failed.
- `403 Forbidden`: License subscription has expired or key was revoked.
- `409 Conflict`: Maximum seat count reached (e.g. 1 seat for Solo, 3 seats for Studio, 10 seats for Team). An existing seat must be deactivated before this machine can activate.

---

### 2.2 Periodic Heartbeat

Silently refreshes activation certificates and checks for revoked seats or chargebacks.

- **Endpoint**: `POST /heartbeat`
- **Headers**:
  - `Content-Type: application/json`

#### Request Payload
```json
{
  "license_id": "3b2e5910-8b1b-4f92-95e2-63b51f04cfd1",
  "device_fingerprint": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "client_time": 1700259200
}
```

#### Response (200 OK)
Returns a refreshed certificate with updated `issued_at` timestamp:
```json
{
  "license_id": "3b2e5910-8b1b-4f92-95e2-63b51f04cfd1",
  "customer_id": "VCT_EMEA_Production",
  "edition": "Studio",
  "features": 18446744073709551615,
  "expires_at": 1735689600,
  "device_fingerprint": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "issued_at": 1700259200,
  "server_signature": [112, 44, 98, ...]
}
```

#### Error Responses
- `403 Forbidden`: `{"error": "License revoked"}` or `{"error": "Device seat was revoked"}`.
- `404 Not Found`: License ID does not exist.

---

### 2.3 Self-Service Device Deactivation

Releases a seat assignment, allowing the customer to migrate Zenocast to a new broadcast workstation.

- **Endpoint**: `POST /deactivate`
- **Headers**:
  - `Content-Type: application/json`

#### Request Payload
```json
{
  "license_id": "3b2e5910-8b1b-4f92-95e2-63b51f04cfd1",
  "device_fingerprint": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
}
```

#### Response (200 OK)
```json
{
  "status": "deactivated",
  "seats_available": 1
}
```

#### Error Responses
- `404 Not Found`: Device fingerprint was not registered under this license ID.

---

### 2.4 Administrative Revocation (Admin Only)

Blocks a compromised or chargebacked license key globally.

- **Endpoint**: `POST /revoke`
- **Headers**:
  - `Content-Type: application/json`
  - `Authorization: Bearer <ADMIN_SECRET_KEY>`

#### Request Payload
```json
{
  "license_id": "3b2e5910-8b1b-4f92-95e2-63b51f04cfd1",
  "reason": "Suspected key leak on public discord channel"
}
```

#### Response (200 OK)
```json
{
  "status": "revoked",
  "revoked_at": 1700500000
}
```

---

## 3. License Editions & Feature Matrix

| Edition | Seat Count | Output Resolution | Multi-Stream | Esports Presets | Control API | Watermark |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Trial** | 1 | 1080p60 | No | Yes | Yes | `"ZENOCAST TRIAL - FOR EVALUATION ONLY"` |
| **Solo** | 1 | 1080p60 | No | Yes | Yes | None |
| **Studio** | 3 | 4K60 | Yes | Yes | Yes | None |
| **Team** | 10 | 4K60 | Yes | Yes | Yes | None |

---

## 4. Client Hardening & Enforcement Policy

1. **Restricted Mode**: When a license is invalid, expired beyond 14 days, or tampered with:
   - The UI and motion-graphics editor remain fully functional so customers never lose access to project files.
   - The broadcast output pipeline (hardware encoder, RTMP/SRT streaming, and disk recording) is gated and disabled.
   - An on-screen warning banner is rendered over preview buses.
2. **Anti-Tamper & Anti-Debug**:
   - Win32 `IsDebuggerPresent` check and code segment memory hash self-checks execute at random session intervals.
   - Any detected modification immediately locks output pipeline and displays `"ZENOCAST RESTRICTED MODE - TAMPER DETECTED"`.
