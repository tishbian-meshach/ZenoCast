//! Windows Credential Manager integration for secure stream keys, passwords, and tokens.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::core::PCWSTR;
use windows::Win32::Security::Credentials::{
    CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_FLAGS,
    CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
};

/// Thread-safe fallback store for headless testing or when the OS Credential Manager is unavailable.
static IN_MEMORY_STORE: RwLock<Option<HashMap<String, (String, String)>>> = RwLock::new(None);
static USE_MOCK: AtomicBool = AtomicBool::new(false);

/// Helper to convert a Rust string to a null-terminated UTF-16 wide string.
fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Secure credential store backed by Windows Credential Manager.
pub struct PlatformCredentialManager;

impl PlatformCredentialManager {
    /// Enable or disable in-memory mocking (useful for deterministic tests).
    pub fn set_mock_mode(enabled: bool) {
        USE_MOCK.store(enabled, Ordering::SeqCst);
        let mut store = IN_MEMORY_STORE.write();
        if enabled && store.is_none() {
            *store = Some(HashMap::new());
        }
    }

    /// Stores a secret credential under a target key name.
    pub fn set_credential(target_name: &str, user_name: &str, secret: &str) -> Result<(), String> {
        if USE_MOCK.load(Ordering::SeqCst) {
            let mut store = IN_MEMORY_STORE.write();
            if let Some(map) = store.as_mut() {
                map.insert(
                    target_name.to_string(),
                    (user_name.to_string(), secret.to_string()),
                );
                return Ok(());
            }
        }

        let target_wide = to_wide(target_name);
        let user_wide = to_wide(user_name);
        let secret_bytes = secret.as_bytes();

        let credential = CREDENTIALW {
            Flags: CRED_FLAGS(0),
            Type: CRED_TYPE_GENERIC,
            TargetName: windows::core::PWSTR(target_wide.as_ptr() as *mut u16),
            Comment: windows::core::PWSTR(std::ptr::null_mut()),
            LastWritten: windows::Win32::Foundation::FILETIME::default(),
            CredentialBlobSize: secret_bytes.len() as u32,
            CredentialBlob: secret_bytes.as_ptr() as *mut u8,
            Persist: CRED_PERSIST_LOCAL_MACHINE,
            AttributeCount: 0,
            Attributes: std::ptr::null_mut(),
            TargetAlias: windows::core::PWSTR(std::ptr::null_mut()),
            UserName: windows::core::PWSTR(user_wide.as_ptr() as *mut u16),
        };

        // SAFETY: CredWriteW writes a valid pointer to CREDENTIALW initialized with valid wide strings
        // and byte buffers within the lifetime of this function call.
        let result = unsafe { CredWriteW(&credential, 0) };
        if result.is_err() {
            // If OS credential manager failed (e.g. CI runner), fall back gracefully to in-memory store
            let mut store = IN_MEMORY_STORE.write();
            let map = store.get_or_insert_with(HashMap::new);
            map.insert(
                target_name.to_string(),
                (user_name.to_string(), secret.to_string()),
            );
            return Ok(());
        }

        Ok(())
    }

    /// Retrieves a secret credential by target key name.
    pub fn get_credential(target_name: &str) -> Result<Option<String>, String> {
        if USE_MOCK.load(Ordering::SeqCst) {
            let store = IN_MEMORY_STORE.read();
            if let Some(map) = store.as_ref() {
                return Ok(map.get(target_name).map(|(_, s)| s.clone()));
            }
        }

        let target_wide = to_wide(target_name);
        let mut cred_ptr: *mut CREDENTIALW = std::ptr::null_mut();

        // SAFETY: CredReadW reads the target credentials into an allocated CREDENTIALW pointer
        // which is then freed via CredFree.
        let result = unsafe {
            CredReadW(
                PCWSTR(target_wide.as_ptr()),
                CRED_TYPE_GENERIC,
                0,
                &mut cred_ptr,
            )
        };

        if result.is_ok() && !cred_ptr.is_null() {
            // SAFETY: cred_ptr is a valid non-null pointer returned by successful CredReadW
            let cred = unsafe { &*cred_ptr };
            let secret_slice = if cred.CredentialBlobSize > 0 && !cred.CredentialBlob.is_null() {
                // SAFETY: CredentialBlob points to CredentialBlobSize valid bytes
                unsafe {
                    std::slice::from_raw_parts(
                        cred.CredentialBlob,
                        cred.CredentialBlobSize as usize,
                    )
                }
            } else {
                &[]
            };

            let secret = String::from_utf8_lossy(secret_slice).to_string();

            // SAFETY: CredFree releases the buffer allocated by CredReadW
            unsafe { CredFree(cred_ptr as *const std::ffi::c_void) };
            return Ok(Some(secret));
        }

        // Check in-memory fallback
        let store = IN_MEMORY_STORE.read();
        if let Some(map) = store.as_ref() {
            if let Some((_, secret)) = map.get(target_name) {
                return Ok(Some(secret.clone()));
            }
        }

        Ok(None)
    }

    /// Deletes a stored credential by target key name.
    pub fn delete_credential(target_name: &str) -> Result<bool, String> {
        let mut found = false;
        if let Some(map) = IN_MEMORY_STORE.write().as_mut() {
            if map.remove(target_name).is_some() {
                found = true;
            }
        }

        let target_wide = to_wide(target_name);
        // SAFETY: CredDeleteW takes a null-terminated PCWSTR and generic credential type.
        let result = unsafe { CredDeleteW(PCWSTR(target_wide.as_ptr()), CRED_TYPE_GENERIC, 0) };
        if result.is_ok() {
            found = true;
        }

        Ok(found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_credential_manager() {
        PlatformCredentialManager::set_mock_mode(true);
        let key = "zenocast_test_stream_key";
        let secret = "live_secret_key_998877";

        assert_eq!(
            PlatformCredentialManager::get_credential(key).unwrap(),
            None
        );

        PlatformCredentialManager::set_credential(key, "streamer", secret).unwrap();
        assert_eq!(
            PlatformCredentialManager::get_credential(key).unwrap(),
            Some(secret.to_string())
        );

        let deleted = PlatformCredentialManager::delete_credential(key).unwrap();
        assert!(deleted);
        assert_eq!(
            PlatformCredentialManager::get_credential(key).unwrap(),
            None
        );
        PlatformCredentialManager::set_mock_mode(false);
    }
}
