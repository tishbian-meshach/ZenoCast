//! Milestone M21 Acceptance Tests: License Verification & Anti-Tamper Hardening.
//!
//! Acceptance criteria (Section 13 & 16):
//! - "a build with a tampered checksum or a detached debugger refuses to start or degrades per Section 16's policy;
//! - an invalid or expired key blocks output;
//! - a valid key works fully offline for the grace period;
//! - the activation server handles a reissue after a seat is deactivated."
//!
//! Verifies:
//! 1. Ed25519 key verification and canonical grouped Base32 token format (`XXXXX-XXXXX-...`).
//! 2. Online activation and device fingerprint binding.
//! 3. Multi-seat enforcement on server (Solo 1 seat vs Studio 3 seats) and self-service reissue.
//! 4. 14-day offline grace period transition and expired subscription handling.
//! 5. Anti-tamper signals, debugger detection, output gating, and on-air watermark banners.
//! 6. Administrative license revocation.

use zeno_license::anti_tamper::AntiTamperGuard;
use zeno_license::certificate::{features, Edition, LicenseKey, LicenseState, RestrictedReason};
use zeno_license::fingerprint::get_device_fingerprint;
use zeno_license::server_client::{LicenseClient, LicenseServer};

#[test]
fn test_m21_ed25519_key_issuance_and_grouped_base32_format() {
    let server = LicenseServer::new([101u8; 32]);
    let now_sec = 1_700_000_000u64;

    // Issue Solo, Studio, Team, and Trial keys
    let solo_key = server.issue_license(
        "solocaster_vct",
        Edition::Solo,
        Edition::Solo.default_seat_count(),
        features::FEATURE_1080P60 | features::FEATURE_CONTROL_API,
        None, // Perpetual
        now_sec,
    );
    assert_eq!(solo_key.payload.seat_count, 1);
    assert_eq!(solo_key.payload.expires_at, None);

    // Verify canonical grouped string format: 5-character groups separated by dashes
    let key_str = solo_key.to_key_string().expect("Serialization succeeded");
    assert!(!key_str.is_empty());
    let parts: Vec<&str> = key_str.split('-').collect();
    assert!(parts.len() >= 5);
    for part in &parts[..parts.len() - 1] {
        assert_eq!(
            part.len(),
            5,
            "Each group except possibly last must be 5 chars"
        );
    }

    // Decode and verify Ed25519 cryptographic signature offline
    let decoded = LicenseKey::from_key_string(&key_str).expect("Decode valid key string");
    assert_eq!(decoded, solo_key);
    assert!(
        decoded.verify(&server.public_key()),
        "Signature must verify against server public key"
    );

    // Tampering test: modify a single character in the key
    let mut tampered_str = key_str.clone();
    let original_char = tampered_str.chars().next().unwrap();
    let replacement = if original_char == 'A' { 'B' } else { 'A' };
    tampered_str.replace_range(0..1, &replacement.to_string());

    let tampered_result = LicenseKey::from_key_string(&tampered_str);
    if let Ok(tampered_key) = tampered_result {
        assert!(
            !tampered_key.verify(&server.public_key()),
            "Tampered payload must fail cryptographic signature verification"
        );
    }
}

#[test]
fn test_m21_online_activation_and_device_fingerprint_binding() {
    let server = LicenseServer::new([102u8; 32]);
    let now_sec = 1_700_000_000u64;

    let key = server.issue_license(
        "riot_games_arena",
        Edition::Studio,
        3,
        features::FEATURE_UNLIMITED,
        Some(365 * 86400),
        now_sec,
    );
    let key_str = key.to_key_string().unwrap();

    let local_fp = get_device_fingerprint();
    assert!(!local_fp.is_empty());

    // Activate on local machine
    let cert = server
        .activate(&key_str, &local_fp, now_sec)
        .expect("Activation must succeed");

    assert_eq!(cert.device_fingerprint, local_fp);
    assert_eq!(cert.edition, Edition::Studio);

    // Install certificate into local client
    let client = LicenseClient::new(server.public_key());
    client.store_certificate(cert, now_sec);

    // Verify valid license state
    let state = client.get_state(now_sec);
    assert!(matches!(state, LicenseState::Valid { .. }));
    assert!(state.is_output_allowed());

    // Hardware change test: simulate certificate being copied to another device
    let foreign_device_fp = "FOREIGN-HARDWARE-SHA256-UUID-123456789";
    let simulated_state = LicenseState::evaluate(
        &server.activate(&key_str, &local_fp, now_sec).unwrap(),
        &server.public_key(),
        now_sec,
        foreign_device_fp,
        Some(now_sec),
    );
    assert_eq!(
        simulated_state,
        LicenseState::Restricted {
            reason: RestrictedReason::DeviceMismatch
        }
    );
    assert!(!simulated_state.is_output_allowed());
}

#[test]
fn test_m21_seat_count_enforcement_and_reissue_after_deactivation() {
    let server = LicenseServer::new([103u8; 32]);
    let now_sec = 1_700_000_000u64;

    // 1. Solo Edition: exactly 1 seat
    let solo_key = server.issue_license(
        "solo_creator",
        Edition::Solo,
        1,
        features::FEATURE_1080P60,
        None,
        now_sec,
    );
    let solo_key_str = solo_key.to_key_string().unwrap();

    let dev_1 = "DEV_WORKSTATION_A";
    let dev_2 = "DEV_LAPTOP_B";

    // First device activates successfully
    assert!(server.activate(&solo_key_str, dev_1, now_sec).is_ok());

    // Second device attempts activation -> rejected because seat limit is reached
    let err = server.activate(&solo_key_str, dev_2, now_sec).unwrap_err();
    assert!(
        err.contains("Maximum seat count (1 seats) reached"),
        "Error must explain seat count limitation: {}",
        err
    );

    // Operator deactivates first device via self-service portal
    server
        .deactivate(solo_key.payload.license_id, dev_1)
        .expect("Deactivation must succeed");

    // Second device can now activate successfully (reissue on new seat)
    assert!(
        server.activate(&solo_key_str, dev_2, now_sec).is_ok(),
        "Second device must activate after seat is freed"
    );

    // 2. Studio Edition: exactly 3 seats
    let studio_key = server.issue_license(
        "production_truck",
        Edition::Studio,
        3,
        features::FEATURE_UNLIMITED,
        None,
        now_sec,
    );
    let studio_key_str = studio_key.to_key_string().unwrap();

    assert!(server.activate(&studio_key_str, "RIG_1", now_sec).is_ok());
    assert!(server.activate(&studio_key_str, "RIG_2", now_sec).is_ok());
    assert!(server.activate(&studio_key_str, "RIG_3", now_sec).is_ok());
    assert!(server.activate(&studio_key_str, "RIG_4", now_sec).is_err());
}

#[test]
fn test_m21_offline_grace_period_and_expired_behavior() {
    let server = LicenseServer::new([104u8; 32]);
    let start_time = 1_700_000_000u64;

    // 1-year subscription
    let key = server.issue_license(
        "traveling_caster",
        Edition::Studio,
        3,
        features::FEATURE_UNLIMITED,
        Some(365 * 86400),
        start_time,
    );
    let key_str = key.to_key_string().unwrap();
    let fp = get_device_fingerprint();

    let cert = server.activate(&key_str, &fp, start_time).unwrap();
    let client = LicenseClient::new(server.public_key());
    client.store_certificate(cert, start_time);

    // Day 0: Valid
    assert!(matches!(
        client.get_state(start_time),
        LicenseState::Valid { .. }
    ));

    // Day 2 (within 72h heartbeat window): still Valid
    let day_2 = start_time + (2 * 86400);
    assert!(matches!(
        client.get_state(day_2),
        LicenseState::Valid { .. }
    ));

    // Day 5 (offline, >72h but <14 days grace period): GracePeriod state, output STILL allowed!
    let day_5 = start_time + (5 * 86400);
    let state_day_5 = client.get_state(day_5);
    assert!(
        matches!(state_day_5, LicenseState::GracePeriod { .. }),
        "Must be in GracePeriod: {:?}",
        state_day_5
    );
    assert!(
        state_day_5.is_output_allowed(),
        "Broadcast output must be permitted during 14-day grace period"
    );

    // Day 15 (offline >14 days): Grace period expired -> Restricted mode, output BLOCKED
    let day_15 = start_time + (15 * 86400);
    let state_day_15 = client.get_state(day_15);
    assert_eq!(
        state_day_15,
        LicenseState::Restricted {
            reason: RestrictedReason::GracePeriodExpired
        }
    );
    assert!(
        !state_day_15.is_output_allowed(),
        "Output must be blocked once grace period expires"
    );

    // Heartbeat reconnects at Day 15: refreshed certificate restores Valid state
    let refreshed_cert = server
        .heartbeat(key.payload.license_id, &fp, day_15)
        .expect("Heartbeat succeeds");
    client.store_certificate(refreshed_cert, day_15);
    assert!(matches!(
        client.get_state(day_15),
        LicenseState::Valid { .. }
    ));

    // Expired subscription test
    let expired_time = start_time + (400 * 86400);
    let state_expired = client.get_state(expired_time);
    assert_eq!(
        state_expired,
        LicenseState::Restricted {
            reason: RestrictedReason::Expired
        }
    );
    assert!(!state_expired.is_output_allowed());
}

#[test]
fn test_m21_anti_tamper_signals_and_watermark_enforcement() {
    // Binary integrity self-check
    assert!(AntiTamperGuard::check_binary_integrity().is_ok());

    let client = LicenseClient::new_mock_valid();
    let now_sec = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let state = client.get_state(now_sec);

    // Clean state: output permitted, no watermark
    assert!(AntiTamperGuard::can_broadcast_output(&state));
    assert_eq!(AntiTamperGuard::get_watermark_text(&state), None);

    // 1. Synthetic Tamper Alert injected (simulates modified binary code section)
    AntiTamperGuard::trigger_tamper_alert();
    assert!(
        !AntiTamperGuard::can_broadcast_output(&state),
        "Output must be immediately blocked on memory checksum mismatch"
    );
    assert_eq!(
        AntiTamperGuard::get_watermark_text(&state),
        Some("ZENOCAST RESTRICTED MODE - TAMPER DETECTED"),
        "Must display restricted tamper warning banner"
    );
    AntiTamperGuard::clear_tamper_alert();
    assert!(AntiTamperGuard::can_broadcast_output(&state));

    // 2. Synthetic Debugger Attached injected (simulates Win32 IsDebuggerPresent)
    AntiTamperGuard::set_debugger_attached(true);
    assert!(
        !AntiTamperGuard::can_broadcast_output(&state),
        "Output must be blocked when debugger is attached"
    );
    AntiTamperGuard::set_debugger_attached(false);
    assert!(AntiTamperGuard::can_broadcast_output(&state));

    // 3. Trial mode watermark
    let trial_state = LicenseState::Valid {
        certificate: zeno_license::certificate::LicenseCertificate {
            license_id: uuid::Uuid::new_v4(),
            customer_id: "trial_evaluator".into(),
            edition: Edition::Trial,
            features: 0xFFFF,
            expires_at: Some(now_sec + 86400),
            device_fingerprint: get_device_fingerprint(),
            issued_at: now_sec,
            server_signature: [1u8; 64],
        },
    };
    assert_eq!(
        AntiTamperGuard::get_watermark_text(&trial_state),
        Some("ZENOCAST TRIAL - FOR EVALUATION ONLY")
    );
}

#[test]
fn test_m21_administrative_revocation_blocks_reissue_and_heartbeats() {
    let server = LicenseServer::new([105u8; 32]);
    let now_sec = 1_700_000_000u64;

    let key = server.issue_license(
        "compromised_org",
        Edition::Team,
        10,
        features::FEATURE_UNLIMITED,
        None,
        now_sec,
    );
    let key_str = key.to_key_string().unwrap();
    let fp = "DEVICE_TEAM_1";

    // Initial activation succeeds
    let cert = server.activate(&key_str, fp, now_sec).unwrap();
    assert_eq!(cert.customer_id, "compromised_org");

    // Administrative revocation due to chargeback / leak
    server.revoke(key.payload.license_id);

    // Heartbeat attempt is rejected
    let hb_err = server
        .heartbeat(key.payload.license_id, fp, now_sec + 3600)
        .unwrap_err();
    assert!(hb_err.contains("License revoked"));

    // Reactivation attempt is rejected
    let act_err = server
        .activate(&key_str, "NEW_DEVICE", now_sec + 3600)
        .unwrap_err();
    assert!(act_err.contains("administratively revoked"));
}
