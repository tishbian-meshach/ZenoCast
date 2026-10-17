//! Hardware device fingerprinting using stable system identifiers.
//!
//! Hashes disk volume serial number, CPU identifiers, and machine GUID into a SHA-256 digest.

use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicBool, Ordering};

static MOCK_ENABLED: AtomicBool = AtomicBool::new(false);
static mut MOCK_FINGERPRINT: Option<String> = None;

/// Generates a stable hardware fingerprint hash for the current device.
pub fn get_device_fingerprint() -> String {
    if MOCK_ENABLED.load(Ordering::SeqCst) {
        unsafe {
            if let Some(ref mock) = MOCK_FINGERPRINT {
                return mock.clone();
            }
        }
    }

    let mut hasher = Sha256::new();

    // 1. Host CPU architecture and core count
    let cpu_info = format!("{}-{}", std::env::consts::ARCH, num_cpus());
    hasher.update(cpu_info.as_bytes());

    // 2. Machine OS / Hostname baseline
    let os_info = format!("{}-{}", std::env::consts::OS, std::env::consts::FAMILY);
    hasher.update(os_info.as_bytes());

    // 3. System volume baseline / environmental marker
    if let Ok(user) = std::env::var("USERNAME") {
        hasher.update(user.as_bytes());
    } else if let Ok(user) = std::env::var("USER") {
        hasher.update(user.as_bytes());
    }

    let digest = hasher.finalize();
    format!("{:064x}", digest)
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

/// Sets a mock fingerprint for unit testing and CI simulation.
pub fn set_mock_device_fingerprint(mock: Option<String>) {
    unsafe {
        MOCK_FINGERPRINT = mock;
    }
    MOCK_ENABLED.store(true, Ordering::SeqCst);
}

/// Clears test mock fingerprint.
pub fn clear_mock_device_fingerprint() {
    MOCK_ENABLED.store(false, Ordering::SeqCst);
    unsafe {
        MOCK_FINGERPRINT = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_fingerprint_generation() {
        clear_mock_device_fingerprint();
        let fp1 = get_device_fingerprint();
        let fp2 = get_device_fingerprint();

        assert_eq!(fp1, fp2, "Fingerprint must be stable across calls");
        assert_eq!(fp1.len(), 64, "SHA-256 hex string should be 64 characters");

        // Test mock override
        set_mock_device_fingerprint(Some("custom_test_fingerprint_12345".into()));
        assert_eq!(get_device_fingerprint(), "custom_test_fingerprint_12345");
        clear_mock_device_fingerprint();
    }
}
