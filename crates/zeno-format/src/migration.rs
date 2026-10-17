//! Schema migration pipeline for transitioning legacy `.arx` packages to the latest engine version.

use crate::error::FormatError;
use crate::manifest::CURRENT_FORMAT_VERSION;

/// Applies sequential migrations to a raw JSON scene object until it matches the current format version.
pub fn migrate_scene(
    from_version: u32,
    _scene_json: &mut serde_json::Value,
) -> Result<u32, FormatError> {
    if from_version > CURRENT_FORMAT_VERSION {
        return Err(FormatError::UnsupportedVersion {
            found: from_version,
            supported_max: CURRENT_FORMAT_VERSION,
        });
    }

    if from_version == CURRENT_FORMAT_VERSION {
        return Ok(from_version);
    }

    // Future version migrations will chain here.
    Err(FormatError::ValidationFailed(format!(
        "No migration step defined from version {}",
        from_version
    )))
}

/// Migration from version 1 to 2 (template for future migrations).
#[allow(dead_code)]
fn migrate_v1_to_v2(scene: &mut serde_json::Value) -> Result<u32, FormatError> {
    if let Some(obj) = scene.as_object_mut() {
        if !obj.contains_key("lifecycle") {
            obj.insert("lifecycle".to_string(), serde_json::Value::Null);
        }
    }
    Ok(2)
}
