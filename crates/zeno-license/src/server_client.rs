//! Activation server logic and local licensing client manager.
//!
//! Implements Section 16 client-server protocol:
//! - Online activation with device fingerprint binding and seat count enforcement.
//! - Periodic heartbeat client refreshing certificates.
//! - Self-service deactivation allowing seat migration to a new machine.
//! - Administrative seat revocation.

use parking_lot::RwLock;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::certificate::{
    Edition, LicenseCertificate, LicenseKey, LicensePayload, LicenseState, RestrictedReason,
};
use crate::ed25519::KeyPair;
use crate::fingerprint::get_device_fingerprint;

/// Internal record of an active license seat on the activation server.
#[derive(Debug, Clone)]
struct ServerLicenseRecord {
    payload: LicensePayload,
    active_devices: HashSet<String>,
    revoked: bool,
}

/// Standalone activation and license authority server (`arena-license-server` specification).
pub struct LicenseServer {
    keypair: KeyPair,
    records: RwLock<HashMap<Uuid, ServerLicenseRecord>>,
}

impl LicenseServer {
    /// Initializes server with a dedicated Ed25519 keypair seed.
    pub fn new(seed: [u8; 32]) -> Self {
        let keypair = KeyPair::from_seed(&seed);
        Self {
            keypair,
            records: RwLock::new(HashMap::new()),
        }
    }

    /// Returns the public Ed25519 verification key to be embedded into the client.
    pub fn public_key(&self) -> [u8; 32] {
        self.keypair.public_key
    }

    /// Issues a new signed license key for a customer.
    pub fn issue_license(
        &self,
        customer_id: &str,
        edition: Edition,
        seat_count: u32,
        features: u64,
        duration_secs: Option<u64>,
        now_sec: u64,
    ) -> LicenseKey {
        let license_id = Uuid::new_v4();
        let expires_at = duration_secs.map(|d| now_sec + d);

        let payload = LicensePayload {
            license_id,
            customer_id: customer_id.to_string(),
            edition,
            seat_count,
            features,
            issued_at: now_sec,
            expires_at,
        };

        let payload_bytes = serde_json::to_vec(&payload).unwrap();
        let signature = self.keypair.sign(&payload_bytes);

        let record = ServerLicenseRecord {
            payload: payload.clone(),
            active_devices: HashSet::new(),
            revoked: false,
        };
        self.records.write().insert(license_id, record);

        LicenseKey { payload, signature }
    }

    /// Handles client activation request (`POST /activate`).
    pub fn activate(
        &self,
        key_str: &str,
        device_fingerprint: &str,
        now_sec: u64,
    ) -> Result<LicenseCertificate, String> {
        let key = LicenseKey::from_key_string(key_str)?;

        // Verify key signature
        if !key.verify(&self.keypair.public_key) {
            return Err("Cryptographic signature on license key is invalid".into());
        }

        let mut records = self.records.write();
        let record = records
            .get_mut(&key.payload.license_id)
            .ok_or_else(|| "License record not found on server".to_string())?;

        if record.revoked {
            return Err("License key has been administratively revoked".into());
        }

        if let Some(exp) = record.payload.expires_at {
            if now_sec > exp {
                return Err("License subscription has expired".into());
            }
        }

        // Check if device is already registered or seats available
        if !record.active_devices.contains(device_fingerprint) {
            if record.active_devices.len() as u32 >= record.payload.seat_count {
                return Err(format!(
                    "Maximum seat count ({} seats) reached for this license. Deactivate an existing device first.",
                    record.payload.seat_count
                ));
            }
            record.active_devices.insert(device_fingerprint.to_string());
        }

        // Issue signed device certificate
        let cert_data = (
            &record.payload.license_id,
            &record.payload.customer_id,
            &record.payload.edition,
            &record.payload.features,
            &record.payload.expires_at,
            device_fingerprint,
            &now_sec,
        );
        let cert_bytes = serde_json::to_vec(&cert_data).unwrap();
        let cert_sig = self.keypair.sign(&cert_bytes);

        Ok(LicenseCertificate {
            license_id: record.payload.license_id,
            customer_id: record.payload.customer_id.clone(),
            edition: record.payload.edition,
            features: record.payload.features,
            expires_at: record.payload.expires_at,
            device_fingerprint: device_fingerprint.to_string(),
            issued_at: now_sec,
            server_signature: cert_sig,
        })
    }

    /// Handles client periodic heartbeat request (`POST /heartbeat`).
    pub fn heartbeat(
        &self,
        license_id: Uuid,
        device_fingerprint: &str,
        now_sec: u64,
    ) -> Result<LicenseCertificate, String> {
        let records = self.records.read();
        let record = records
            .get(&license_id)
            .ok_or_else(|| "License not found".to_string())?;

        if record.revoked {
            return Err("License revoked".into());
        }

        if !record.active_devices.contains(device_fingerprint) {
            return Err("Seat for this device has been released or revoked".into());
        }

        // Return refreshed certificate
        let cert_data = (
            &record.payload.license_id,
            &record.payload.customer_id,
            &record.payload.edition,
            &record.payload.features,
            &record.payload.expires_at,
            device_fingerprint,
            &now_sec,
        );
        let cert_bytes = serde_json::to_vec(&cert_data).unwrap();
        let cert_sig = self.keypair.sign(&cert_bytes);

        Ok(LicenseCertificate {
            license_id: record.payload.license_id,
            customer_id: record.payload.customer_id.clone(),
            edition: record.payload.edition,
            features: record.payload.features,
            expires_at: record.payload.expires_at,
            device_fingerprint: device_fingerprint.to_string(),
            issued_at: now_sec,
            server_signature: cert_sig,
        })
    }

    /// Handles self-service device deactivation (`POST /deactivate`).
    pub fn deactivate(&self, license_id: Uuid, device_fingerprint: &str) -> Result<(), String> {
        let mut records = self.records.write();
        let record = records
            .get_mut(&license_id)
            .ok_or_else(|| "License not found".to_string())?;

        if record.active_devices.remove(device_fingerprint) {
            Ok(())
        } else {
            Err("Device was not registered under this license".into())
        }
    }

    /// Administratively revokes a license key (e.g. chargeback / leak).
    pub fn revoke(&self, license_id: Uuid) {
        if let Some(record) = self.records.write().get_mut(&license_id) {
            record.revoked = true;
            record.active_devices.clear();
        }
    }
}

/// Client-side licensing manager running on the user's workstation.
pub struct LicenseClient {
    server_public_key: [u8; 32],
    active_certificate: RwLock<Option<LicenseCertificate>>,
    last_heartbeat_timestamp: RwLock<Option<u64>>,
}

impl LicenseClient {
    /// Creates a new license client with embedded server public key.
    pub fn new(server_public_key: [u8; 32]) -> Self {
        Self {
            server_public_key,
            active_certificate: RwLock::new(None),
            last_heartbeat_timestamp: RwLock::new(None),
        }
    }

    /// Creates a mock valid client with an activated Studio license for testing and offline development.
    pub fn new_mock_valid() -> Self {
        let server = LicenseServer::new([42u8; 32]);
        let now_sec = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let key = server.issue_license(
            "operator_dev",
            Edition::Studio,
            3,
            0xFFFF_FFFF,
            Some(365 * 86400),
            now_sec,
        );
        let key_str = key.to_key_string().unwrap();
        let fp = get_device_fingerprint();
        let cert = server.activate(&key_str, &fp, now_sec).unwrap();
        let client = Self::new(server.public_key());
        client.store_certificate(cert, now_sec);
        client
    }

    /// Installs a new activation certificate.
    pub fn store_certificate(&self, cert: LicenseCertificate, now_sec: u64) {
        *self.active_certificate.write() = Some(cert);
        *self.last_heartbeat_timestamp.write() = Some(now_sec);
    }

    /// Clears certificate (e.g. after deactivation).
    pub fn clear_certificate(&self) {
        *self.active_certificate.write() = None;
        *self.last_heartbeat_timestamp.write() = None;
    }

    /// Evaluates current license state against local hardware and system clock.
    pub fn get_state(&self, now_sec: u64) -> LicenseState {
        let cert_guard = self.active_certificate.read();
        let Some(ref cert) = *cert_guard else {
            return LicenseState::Restricted {
                reason: RestrictedReason::NoActivation,
            };
        };

        let current_fp = get_device_fingerprint();
        let last_hb = *self.last_heartbeat_timestamp.read();

        LicenseState::evaluate(cert, &self.server_public_key, now_sec, &current_fp, last_hb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_mock_valid() {
        let client = LicenseClient::new_mock_valid();
        let now_sec = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let state = client.get_state(now_sec);
        assert!(state.is_output_allowed());
    }
}
