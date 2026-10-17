//! Loading, saving, unpacking, and packaging `.arx` zip archives.

use crate::error::FormatError;
use crate::manifest::{Manifest, CURRENT_FORMAT_VERSION};
use crate::migration::migrate_scene;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read, Seek, Write};
use std::path::Path;
use zeno_engine::Composition;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// Validation report summarizing integrity checks performed on an `.arx` container.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationReport {
    /// Whether all checks passed.
    pub is_valid: bool,
    /// Package manifest.
    pub manifest: Manifest,
    /// Layer count found in scene.json.
    pub layer_count: usize,
    /// Declared variables count.
    pub variable_count: usize,
    /// Embedded asset count.
    pub asset_count: usize,
    /// Any non-fatal warnings (e.g. unreferenced assets, zero duration).
    pub warnings: Vec<String>,
    /// Fatal error messages encountered.
    pub errors: Vec<String>,
}

/// In-memory representation of an unpacked `.arx` motion-graphics package.
#[derive(Debug, Clone, PartialEq)]
pub struct ArxPackage {
    /// Package manifest metadata.
    pub manifest: Manifest,
    /// Composition scene graph and timeline.
    pub scene: Composition,
    /// In-memory raw bytes for embedded assets keyed by relative path (e.g. "assets/alpha.png").
    pub assets: HashMap<String, Vec<u8>>,
    /// Optional preview thumbnail PNG bytes.
    pub thumbnail: Option<Vec<u8>>,
}

impl ArxPackage {
    /// Construct a new package from a composition and manifest.
    pub fn new(manifest: Manifest, scene: Composition) -> Self {
        Self {
            manifest,
            scene,
            assets: HashMap::new(),
            thumbnail: None,
        }
    }

    /// Read and unpack an `.arx` package from any seekable reader.
    pub fn read<R: Read + Seek>(reader: R) -> Result<Self, FormatError> {
        let mut archive = ZipArchive::new(reader)?;

        // 1. Read manifest.json
        let manifest_content = {
            let mut file = archive
                .by_name("manifest.json")
                .map_err(|_| FormatError::MissingEntry("manifest.json".to_string()))?;
            let mut buf = String::new();
            file.read_to_string(&mut buf)?;
            buf
        };

        let raw_manifest: serde_json::Value = serde_json::from_str(&manifest_content)?;
        let format_version = raw_manifest
            .get("format_version")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| {
                FormatError::InvalidManifest(
                    "Missing or invalid 'format_version' field".to_string(),
                )
            })? as u32;

        if format_version > CURRENT_FORMAT_VERSION {
            return Err(FormatError::UnsupportedVersion {
                found: format_version,
                supported_max: CURRENT_FORMAT_VERSION,
            });
        }

        let manifest: Manifest = serde_json::from_value(raw_manifest)?;

        // 2. Read scene.json
        let scene_content = {
            let mut file = archive
                .by_name("scene.json")
                .map_err(|_| FormatError::MissingEntry("scene.json".to_string()))?;
            let mut buf = String::new();
            file.read_to_string(&mut buf)?;
            buf
        };

        let mut scene_json: serde_json::Value = serde_json::from_str(&scene_content)?;

        // 3. Migrate if from an older format
        if format_version < CURRENT_FORMAT_VERSION {
            migrate_scene(format_version, &mut scene_json)?;
        }

        let scene: Composition = serde_json::from_value(scene_json)?;

        // 4. Read thumbnail.png if present
        let mut thumbnail = None;
        if let Ok(mut thumb_file) = archive.by_name("thumbnail.png") {
            let mut thumb_bytes = Vec::new();
            thumb_file.read_to_end(&mut thumb_bytes)?;
            thumbnail = Some(thumb_bytes);
        }

        // 5. Read all files under assets/
        let mut assets = HashMap::new();
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            let name = entry.name().to_string();
            if name.starts_with("assets/") && !name.ends_with('/') {
                let mut data = Vec::new();
                entry.read_to_end(&mut data)?;
                assets.insert(name, data);
            }
        }

        Ok(Self {
            manifest,
            scene,
            assets,
            thumbnail,
        })
    }

    /// Open an `.arx` package directly from a file path.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, FormatError> {
        let file = File::open(path)?;
        Self::read(file)
    }

    /// Pack and write this package into any seekable writer.
    pub fn write<W: Write + Seek>(&self, writer: W) -> Result<(), FormatError> {
        let mut zip = ZipWriter::new(writer);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        // 1. Write manifest.json
        zip.start_file("manifest.json", options)?;
        let manifest_bytes = serde_json::to_vec_pretty(&self.manifest)?;
        zip.write_all(&manifest_bytes)?;

        // 2. Write scene.json
        zip.start_file("scene.json", options)?;
        let scene_bytes = serde_json::to_vec_pretty(&self.scene)?;
        zip.write_all(&scene_bytes)?;

        // 3. Write thumbnail.png if present
        if let Some(thumb) = &self.thumbnail {
            zip.start_file("thumbnail.png", options)?;
            zip.write_all(thumb)?;
        }

        // 4. Write assets
        for (rel_path, data) in &self.assets {
            let clean_path = if rel_path.starts_with("assets/") {
                rel_path.clone()
            } else {
                format!("assets/{}", rel_path.trim_start_matches('/'))
            };
            zip.start_file(clean_path, options)?;
            zip.write_all(data)?;
        }

        zip.finish()?;
        Ok(())
    }

    /// Save the package to a local file on disk.
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), FormatError> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = File::create(path)?;
        self.write(file)
    }

    /// Save to in-memory bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, FormatError> {
        let mut buffer = Cursor::new(Vec::new());
        self.write(&mut buffer)?;
        Ok(buffer.into_inner())
    }

    /// Load package directly from memory slice.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, FormatError> {
        Self::read(Cursor::new(bytes))
    }

    /// Perform comprehensive validation on this package.
    pub fn validate(&self) -> ValidationReport {
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        if self.manifest.width == 0 || self.manifest.height == 0 {
            errors.push("Canvas dimensions width and height must be non-zero".to_string());
        }

        if self.manifest.fps <= 0.0 {
            errors.push("Framerate must be strictly positive".to_string());
        }

        if self.scene.duration_seconds <= 0.0 {
            warnings.push("Composition has zero or negative duration".to_string());
        }

        // Verify that all assets referenced in composition exist in package
        for asset_ref in &self.scene.assets {
            let asset_path = if asset_ref.path.starts_with("assets/") {
                asset_ref.path.clone()
            } else {
                format!("assets/{}", asset_ref.path)
            };

            if !self.assets.contains_key(&asset_path) {
                errors.push(format!(
                    "Asset '{}' (ID {}) referenced in scene is missing from container",
                    asset_ref.path, asset_ref.id
                ));
            }
        }

        // Check for reserved animation states
        let has_in = self.scene.find_state("in").is_some();
        let has_loop = self.scene.find_state("loop").is_some();
        let has_out = self.scene.find_state("out").is_some();

        if !has_in || !has_loop || !has_out {
            warnings.push(format!(
                "Composition is missing standard animation state(s): in={}, loop={}, out={}",
                has_in, has_loop, has_out
            ));
        }

        ValidationReport {
            is_valid: errors.is_empty(),
            manifest: self.manifest.clone(),
            layer_count: self.scene.layers.len(),
            variable_count: self.scene.variables.len(),
            asset_count: self.assets.len(),
            warnings,
            errors,
        }
    }

    /// Open and validate an `.arx` file on disk.
    pub fn validate_file<P: AsRef<Path>>(path: P) -> Result<ValidationReport, FormatError> {
        let pkg = Self::open(path)?;
        Ok(pkg.validate())
    }
}
