//! License token models, cryptographic certificate verification, and licensing state machine.
//!
//! Adheres strictly to Section 16 specification:
//! - Ed25519 offline verification against embedded server public key.
//! - Grace period offline operation (14 days default).
//! - Clean degradation to Restricted mode without data destruction.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::base32::{decode_grouped, encode_grouped};
use crate::ed25519::verify_signature;

/// Commercial edition tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Edition {
    Solo,
    Studio,
    Team,
    Trial,
}

impl Edition {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Solo => "Solo Edition (1 Seat)",
            Self::Studio => "Studio Edition (3 Seats)",
            Self::Team => "Team / Enterprise Edition (10 Seats)",
            Self::Trial => "Trial Edition (14-Day Evaluation)",
        }
    }

    pub fn default_seat_count(&self) -> u32 {
        match self {
            Self::Solo => 1,
            Self::Studio => 3,
            Self::Team => 10,
            Self::Trial => 1,
        }
    }
}

/// Bitmask feature flags for edition gating.
pub mod features {
    pub const FEATURE_1080P60: u64 = 1 << 0;
    pub const FEATURE_4K: u64 = 1 << 1;
    pub const FEATURE_MULTI_STREAM: u64 = 1 << 2;
    pub const FEATURE_SRT: u64 = 1 << 3;
    pub const FEATURE_ESPORTS: u64 = 1 << 4;
    pub const FEATURE_CONTROL_API: u64 = 1 << 5;
    pub const FEATURE_UNLIMITED: u64 = 0xFFFF_FFFF_FFFF_FFFF;
}

/// Signed license payload distributed to customers as a typed key.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LicensePayload {
    pub license_id: Uuid,
    pub customer_id: String,
    pub edition: Edition,
    pub seat_count: u32,
    pub features: u64,
    pub issued_at: u64,
    pub expires_at: Option<u64>,
}

/// Offline-verifiable typed license key token (grouped Base32).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LicenseKey {
    pub payload: LicensePayload,
    #[serde(with = "signature_serde")]
    pub signature: [u8; 64],
}

impl LicenseKey {
    /// Encodes license key into canonical grouped string: `XXXXX-XXXXX-XXXXX-...`
    pub fn to_key_string(&self) -> Result<String, String> {
        let json_bytes = serde_json::to_vec(self)
            .map_err(|e| format!("Failed to serialize license key: {}", e))?;
        Ok(encode_grouped(&json_bytes))
    }

    /// Parses and decodes grouped license key string.
    pub fn from_key_string(s: &str) -> Result<Self, String> {
        let raw_bytes = decode_grouped(s)?;
        serde_json::from_slice(&raw_bytes).map_err(|e| format!("Invalid license key format: {}", e))
    }

    /// Verifies the key signature against the server public key.
    pub fn verify(&self, server_pubkey: &[u8; 32]) -> bool {
        let Ok(payload_bytes) = serde_json::to_vec(&self.payload) else {
            return false;
        };
        verify_signature(server_pubkey, &payload_bytes, &self.signature)
    }
}

/// Signed activation certificate bound to a specific hardware device fingerprint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LicenseCertificate {
    pub license_id: Uuid,
    pub customer_id: String,
    pub edition: Edition,
    pub features: u64,
    pub expires_at: Option<u64>,
    pub device_fingerprint: String,
    pub issued_at: u64,
    #[serde(with = "signature_serde")]
    pub server_signature: [u8; 64],
}

mod signature_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(sig: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        sig.to_vec().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec: Vec<u8> = Vec::deserialize(deserializer)?;
        let mut arr = [0u8; 64];
        if vec.len() == 64 {
            arr.copy_from_slice(&vec);
            Ok(arr)
        } else {
            Err(serde::de::Error::custom("Expected 64-byte signature"))
        }
    }
}

/// Specific reason why licensing dropped into Restricted mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RestrictedReason {
    NoActivation,
    Expired,
    TamperDetected,
    SeatRevoked,
    GracePeriodExpired,
    InvalidSignature,
    DeviceMismatch,
}

impl RestrictedReason {
    pub fn message(&self) -> &'static str {
        match self {
            Self::NoActivation => "Zenocast has not been activated on this machine.",
            Self::Expired => "Your license subscription has expired.",
            Self::TamperDetected => "Security integrity anomaly detected. Output pipeline locked.",
            Self::SeatRevoked => "This machine seat was deactivated from the portal.",
            Self::GracePeriodExpired => {
                "Offline grace period (14 days) exceeded. Please reconnect."
            }
            Self::InvalidSignature => "License signature could not be verified.",
            Self::DeviceMismatch => "License certificate belongs to another device.",
        }
    }
}

/// Current active license state of the broadcast application.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LicenseState {
    Valid {
        certificate: LicenseCertificate,
    },
    GracePeriod {
        certificate: LicenseCertificate,
        offline_since_sec: u64,
    },
    Restricted {
        reason: RestrictedReason,
    },
}

impl LicenseState {
    /// Checks whether the output pipeline (encoding, RTMP/SRT streaming, recording) is permitted.
    pub fn is_output_allowed(&self) -> bool {
        match self {
            Self::Valid { .. } | Self::GracePeriod { .. } => true,
            Self::Restricted { .. } => false,
        }
    }

    /// Whether this license state requires displaying an on-air watermark banner.
    pub fn requires_watermark(&self) -> bool {
        match self {
            Self::Valid { certificate } => certificate.edition == Edition::Trial,
            Self::GracePeriod { certificate, .. } => certificate.edition == Edition::Trial,
            Self::Restricted { .. } => true,
        }
    }

    /// Evaluates certificate validity against local time and device fingerprint.
    pub fn evaluate(
        cert: &LicenseCertificate,
        server_pubkey: &[u8; 32],
        current_time_sec: u64,
        host_fingerprint: &str,
        last_online_heartbeat_sec: Option<u64>,
    ) -> Self {
        // 1. Verify device fingerprint binding
        if cert.device_fingerprint != host_fingerprint {
            return Self::Restricted {
                reason: RestrictedReason::DeviceMismatch,
            };
        }

        // 2. Verify cryptographic signature
        let Ok(data_to_verify) = serde_json::to_vec(&(
            &cert.license_id,
            &cert.customer_id,
            &cert.edition,
            &cert.features,
            &cert.expires_at,
            &cert.device_fingerprint,
            &cert.issued_at,
        )) else {
            return Self::Restricted {
                reason: RestrictedReason::InvalidSignature,
            };
        };

        if !verify_signature(server_pubkey, &data_to_verify, &cert.server_signature) {
            return Self::Restricted {
                reason: RestrictedReason::InvalidSignature,
            };
        }

        // 3. Verify expiration
        if let Some(exp) = cert.expires_at {
            if current_time_sec > exp {
                return Self::Restricted {
                    reason: RestrictedReason::Expired,
                };
            }
        }

        // 4. Verify offline grace period (14 days = 1,209,600 seconds)
        const GRACE_PERIOD_SECS: u64 = 14 * 24 * 3600;
        if let Some(last_seen) = last_online_heartbeat_sec {
            let offline_duration = current_time_sec.saturating_sub(last_seen);
            if offline_duration > GRACE_PERIOD_SECS {
                return Self::Restricted {
                    reason: RestrictedReason::GracePeriodExpired,
                };
            } else if offline_duration > (72 * 3600) {
                // Heartbeat expected every 72 hours; offline longer enters Grace Period countdown
                return Self::GracePeriod {
                    certificate: cert.clone(),
                    offline_since_sec: last_seen,
                };
            }
        }

        Self::Valid {
            certificate: cert.clone(),
        }
    }
}
