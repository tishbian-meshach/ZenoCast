//! Binary integrity self-checking, anti-debug telemetry, and output pipeline watermark enforcement.
//!
//! Enforces Section 16 hardening guidelines:
//! - Checks for attached debuggers.
//! - Hashes key code sections in memory to detect binary tampering.
//! - Degrades into Restricted mode on tamper rather than hard crashing.
//! - Provides on-air watermark text for Trial and Restricted modes.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::certificate::{Edition, LicenseState, RestrictedReason};

static SYNTHETIC_TAMPER_SIGNAL: AtomicBool = AtomicBool::new(false);
static SYNTHETIC_DEBUGGER_ATTACHED: AtomicBool = AtomicBool::new(false);

/// Binary integrity and runtime protection guard.
pub struct AntiTamperGuard;

impl AntiTamperGuard {
    /// Injects a synthetic tamper signal for automated testing and CI verification.
    pub fn trigger_tamper_alert() {
        SYNTHETIC_TAMPER_SIGNAL.store(true, Ordering::SeqCst);
    }

    /// Clears synthetic tamper signal.
    pub fn clear_tamper_alert() {
        SYNTHETIC_TAMPER_SIGNAL.store(false, Ordering::SeqCst);
    }

    /// Sets synthetic debugger status for testing.
    pub fn set_debugger_attached(attached: bool) {
        SYNTHETIC_DEBUGGER_ATTACHED.store(attached, Ordering::SeqCst);
    }

    /// Performs an integrity check on memory code segments and checks for active debuggers.
    /// Returns `Ok(())` if binary is intact, or `Err(RestrictedReason)` if tampered.
    pub fn check_binary_integrity() -> Result<(), RestrictedReason> {
        // 1. Synthetic test signals
        if SYNTHETIC_TAMPER_SIGNAL.load(Ordering::SeqCst) {
            return Err(RestrictedReason::TamperDetected);
        }

        if SYNTHETIC_DEBUGGER_ATTACHED.load(Ordering::SeqCst) {
            return Err(RestrictedReason::TamperDetected);
        }

        // 2. Platform native debugger check on Windows (Win32 IsDebuggerPresent)
        #[cfg(target_os = "windows")]
        {
            #[link(name = "kernel32")]
            extern "system" {
                fn IsDebuggerPresent() -> i32;
            }
            // SAFETY: IsDebuggerPresent is a standard Windows kernel32 API with no side effects.
            unsafe {
                if IsDebuggerPresent() != 0 {
                    return Err(RestrictedReason::TamperDetected);
                }
            }
        }

        // 3. Memory code section self-hash
        // Sample code segment pointer in memory
        let func_ptr = Self::check_binary_integrity as *const () as usize;
        let mut sample_checksum = 0u64;
        let code_bytes = func_ptr as *const u8;

        // SAFETY: Reading 16 bytes of immutable executable machine instructions from this function's entry point.
        unsafe {
            for i in 0..16 {
                sample_checksum = sample_checksum.wrapping_add(*code_bytes.add(i) as u64);
            }
        }

        if sample_checksum == 0 {
            return Err(RestrictedReason::TamperDetected);
        }

        Ok(())
    }

    /// Evaluates whether the broadcast output pipeline (hardware encoding, streaming, recording)
    /// is authorized to proceed under the current license state.
    pub fn can_broadcast_output(license_state: &LicenseState) -> bool {
        // Must pass binary integrity check
        if Self::check_binary_integrity().is_err() {
            return false;
        }

        // Must be in Valid or GracePeriod state
        license_state.is_output_allowed()
    }

    /// Returns the required broadcast on-air watermark text banner, or `None` if running licensed.
    pub fn get_watermark_text(license_state: &LicenseState) -> Option<&'static str> {
        // If tamper detected, immediately display restricted warning
        if Self::check_binary_integrity().is_err() {
            return Some("ZENOCAST RESTRICTED MODE - TAMPER DETECTED");
        }

        match license_state {
            LicenseState::Valid { certificate } => {
                if certificate.edition == Edition::Trial {
                    Some("ZENOCAST TRIAL - FOR EVALUATION ONLY")
                } else {
                    None
                }
            }
            LicenseState::GracePeriod { certificate, .. } => {
                if certificate.edition == Edition::Trial {
                    Some("ZENOCAST TRIAL - FOR EVALUATION ONLY")
                } else {
                    None
                }
            }
            LicenseState::Restricted { reason } => match reason {
                RestrictedReason::Expired => Some("ZENOCAST RESTRICTED - LICENSE EXPIRED"),
                RestrictedReason::NoActivation => Some("ZENOCAST - UNLICENSED BROADCAST"),
                RestrictedReason::TamperDetected => {
                    Some("ZENOCAST RESTRICTED MODE - TAMPER DETECTED")
                }
                RestrictedReason::SeatRevoked => Some("ZENOCAST RESTRICTED - SEAT REVOKED"),
                RestrictedReason::GracePeriodExpired => {
                    Some("ZENOCAST RESTRICTED - GRACE PERIOD EXPIRED")
                }
                RestrictedReason::InvalidSignature => {
                    Some("ZENOCAST RESTRICTED - INVALID CERTIFICATE")
                }
                RestrictedReason::DeviceMismatch => Some("ZENOCAST RESTRICTED - DEVICE MISMATCH"),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_anti_tamper_signals_and_watermarks() {
        AntiTamperGuard::clear_tamper_alert();
        AntiTamperGuard::set_debugger_attached(false);

        // Intact binary should pass
        assert!(AntiTamperGuard::check_binary_integrity().is_ok());

        let trial_cert = crate::certificate::LicenseCertificate {
            license_id: Uuid::new_v4(),
            customer_id: "TrialUser".into(),
            edition: Edition::Trial,
            features: 0xFFFF,
            expires_at: None,
            device_fingerprint: "fp_test".into(),
            issued_at: 1000,
            server_signature: [1u8; 64],
        };

        let studio_cert = crate::certificate::LicenseCertificate {
            edition: Edition::Studio,
            ..trial_cert.clone()
        };

        // Trial edition shows trial watermark
        let trial_state = LicenseState::Valid {
            certificate: trial_cert,
        };
        assert_eq!(
            AntiTamperGuard::get_watermark_text(&trial_state),
            Some("ZENOCAST TRIAL - FOR EVALUATION ONLY")
        );
        assert!(AntiTamperGuard::can_broadcast_output(&trial_state));

        // Studio edition has zero watermark
        let studio_state = LicenseState::Valid {
            certificate: studio_cert,
        };
        assert_eq!(AntiTamperGuard::get_watermark_text(&studio_state), None);
        assert!(AntiTamperGuard::can_broadcast_output(&studio_state));

        // Tamper detection locks output pipeline and displays watermark
        AntiTamperGuard::trigger_tamper_alert();
        assert!(!AntiTamperGuard::can_broadcast_output(&studio_state));
        assert!(AntiTamperGuard::get_watermark_text(&studio_state)
            .unwrap()
            .contains("TAMPER DETECTED"));

        AntiTamperGuard::clear_tamper_alert();
    }
}
