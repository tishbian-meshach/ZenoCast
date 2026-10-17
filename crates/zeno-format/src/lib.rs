//! # zeno-format
//!
//! Serialization, packaging, validation, and schema migration for the `.arx` container format.

pub mod error;
pub mod manifest;
pub mod migration;
pub mod package;

// Re-exports
pub use error::FormatError;
pub use manifest::{Manifest, CURRENT_ENGINE_VERSION, CURRENT_FORMAT_VERSION};
pub use migration::migrate_scene;
pub use package::{ArxPackage, ValidationReport};

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use zeno_core::{Color, LayerId, Property};
    use zeno_engine::{Composition, Layer, LayerContent, TextContent};

    fn make_test_composition(name: &str, text: &str) -> (Manifest, Composition) {
        let manifest = Manifest::new(name, "Zenocast Test", 1920, 1080, 60.0);
        let mut comp = Composition::new(1920, 1080, 60.0, 5.0);

        let layer = Layer::new(
            LayerId::new(),
            "Title",
            LayerContent::Text(TextContent {
                text: Property::constant(text.to_string()),
                color: Property::constant(Color::WHITE),
                ..Default::default()
            }),
        );
        comp.layers.push(layer);
        (manifest, comp)
    }

    #[test]
    fn test_arx_roundtrip_in_memory() {
        let (manifest, comp) = make_test_composition("Test Overlay", "Score: 1 - 0");
        let mut pkg = ArxPackage::new(manifest, comp);
        pkg.assets
            .insert("assets/logo.png".to_string(), vec![0x89, 0x50, 0x4E, 0x47]);
        pkg.thumbnail = Some(vec![1, 2, 3, 4]);

        let bytes = pkg.to_bytes().expect("serialize arx package to bytes");
        let reloaded = ArxPackage::from_bytes(&bytes).expect("deserialize arx package from bytes");

        assert_eq!(pkg.manifest, reloaded.manifest);
        assert_eq!(pkg.scene, reloaded.scene);
        assert_eq!(pkg.assets, reloaded.assets);
        assert_eq!(pkg.thumbnail, reloaded.thumbnail);
    }

    #[test]
    fn test_invalid_archive_handling() {
        let bad_bytes = b"This is not a zip archive";
        let err = ArxPackage::from_bytes(bad_bytes).unwrap_err();
        assert!(matches!(err, FormatError::Zip(_)));
    }

    #[test]
    fn test_future_version_rejected() {
        let manifest = Manifest {
            format_version: 9999,
            name: "Future Overlay".to_string(),
            author: "Time Traveler".to_string(),
            width: 1920,
            height: 1080,
            fps: 60.0,
            engine_min_version: "99.0.0".to_string(),
        };
        let comp = Composition::default();
        let pkg = ArxPackage::new(manifest, comp);

        let bytes = pkg.to_bytes().expect("pack");
        let err = ArxPackage::from_bytes(&bytes).unwrap_err();
        assert!(matches!(
            err,
            FormatError::UnsupportedVersion { found: 9999, .. }
        ));
    }

    proptest! {
        #[test]
        fn prop_test_roundtrip(name in "\\PC{1,20}", text in "\\PC{1,30}") {
            let (manifest, comp) = make_test_composition(&name, &text);
            let pkg = ArxPackage::new(manifest, comp);

            let bytes = pkg.to_bytes().expect("pack to bytes");
            let reloaded = ArxPackage::from_bytes(&bytes).expect("unpack from bytes");

            prop_assert_eq!(&pkg.manifest.name, &reloaded.manifest.name);
            prop_assert_eq!(pkg.scene.layers.len(), reloaded.scene.layers.len());
            prop_assert_eq!(&pkg.scene.layers[0].name, &reloaded.scene.layers[0].name);
        }
    }
}
