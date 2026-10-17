//! # zeno-license
//!
//! Licensing and anti-tamper subsystem: Ed25519 offline license verification,
//! periodic heartbeat client, hardware fingerprinting, and binary integrity self-checks.

pub mod anti_tamper;
pub mod base32;
pub mod certificate;
pub mod ed25519;
pub mod fingerprint;
pub mod server_client;

// Re-exports
pub use anti_tamper::AntiTamperGuard;
pub use base32::{decode_grouped, encode_grouped};
pub use certificate::{
    features, Edition, LicenseCertificate, LicenseKey, LicensePayload, LicenseState,
    RestrictedReason,
};
pub use ed25519::{verify_signature, KeyPair};
pub use fingerprint::{
    clear_mock_device_fingerprint, get_device_fingerprint, set_mock_device_fingerprint,
};
pub use server_client::{LicenseClient, LicenseServer};
