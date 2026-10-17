//! Integration tests for Milestone 12: PSD and Lottie importers,
//! data-binding auto-detection, and import report auditing.

use zeno_core::Vec3;
use zeno_engine::{BlendMode, LayerContent};
use zeno_import::{ImportReport, LottieImporter, PsdImporter};

/// Helper to generate a minimal valid binary PSD stream with specified layers.
fn create_test_psd_bytes() -> Vec<u8> {
    let mut bytes = Vec::new();

    // 1. Header (26 bytes)
    bytes.extend_from_slice(b"8BPS"); // Signature
    bytes.extend_from_slice(&1u16.to_be_bytes()); // Version 1
    bytes.extend_from_slice(&[0u8; 6]); // Reserved
    bytes.extend_from_slice(&3u16.to_be_bytes()); // Channels (3)
    bytes.extend_from_slice(&1080u32.to_be_bytes()); // Height
    bytes.extend_from_slice(&1920u32.to_be_bytes()); // Width
    bytes.extend_from_slice(&8u16.to_be_bytes()); // Depth (8-bit)
    bytes.extend_from_slice(&3u16.to_be_bytes()); // ColorMode (3 = RGB)

    // 2. Color Mode Data Section (Length = 0)
    bytes.extend_from_slice(&0u32.to_be_bytes());

    // 3. Image Resources Section (Length = 0)
    bytes.extend_from_slice(&0u32.to_be_bytes());

    // 4. Layer and Mask Information Section
    let mut layer_info = Vec::new();
    let layer_count: i16 = 3;
    layer_info.extend_from_slice(&layer_count.to_be_bytes());

    // Helper to build a layer record
    let make_layer_record = |name: &str,
                             top: i32,
                             left: i32,
                             bottom: i32,
                             right: i32,
                             blend_key: &[u8; 4],
                             opacity: u8,
                             has_mask: bool,
                             is_text: bool|
     -> Vec<u8> {
        let mut r = Vec::new();
        // Rect bounds
        r.extend_from_slice(&top.to_be_bytes());
        r.extend_from_slice(&left.to_be_bytes());
        r.extend_from_slice(&bottom.to_be_bytes());
        r.extend_from_slice(&right.to_be_bytes());

        // Channels count: 3 (R, G, B)
        r.extend_from_slice(&3u16.to_be_bytes());
        for ch_id in 0..3i16 {
            r.extend_from_slice(&ch_id.to_be_bytes());
            r.extend_from_slice(&2u32.to_be_bytes()); // length
        }

        // Blend mode sig
        r.extend_from_slice(b"8BIM");
        r.extend_from_slice(blend_key);
        r.push(opacity); // opacity
        r.push(0); // clipping
        r.push(0); // flags (0 = visible)
        r.push(0); // filler

        // Extra data section
        let mut extra = Vec::new();

        // Mask data
        if has_mask {
            let mut mask_block = Vec::new();
            mask_block.extend_from_slice(&10i32.to_be_bytes()); // top
            mask_block.extend_from_slice(&10i32.to_be_bytes()); // left
            mask_block.extend_from_slice(&100i32.to_be_bytes()); // bottom
            mask_block.extend_from_slice(&100i32.to_be_bytes()); // right
            mask_block.push(0); // default color (0 = alpha)
            mask_block.push(0); // flags
            mask_block.extend_from_slice(&[0u8; 2]); // padding to 20 bytes

            extra.extend_from_slice(&(mask_block.len() as u32).to_be_bytes());
            extra.extend_from_slice(&mask_block);
        } else {
            extra.extend_from_slice(&0u32.to_be_bytes());
        }

        // Blending ranges (0 len)
        extra.extend_from_slice(&0u32.to_be_bytes());

        // Layer Name (Pascal string padded to 4 bytes)
        let name_bytes = name.as_bytes();
        extra.push(name_bytes.len() as u8);
        extra.extend_from_slice(name_bytes);
        let rem = (name_bytes.len() + 1) % 4;
        if rem != 0 {
            extra.extend_from_slice(&vec![0u8; 4 - rem]);
        }

        // Additional tagged blocks
        if is_text {
            extra.extend_from_slice(b"8BIM");
            extra.extend_from_slice(b"TySh");
            let tysh_data = vec![0u8; 8];
            extra.extend_from_slice(&(tysh_data.len() as u32).to_be_bytes());
            extra.extend_from_slice(&tysh_data);
        }

        r.extend_from_slice(&(extra.len() as u32).to_be_bytes());
        r.extend_from_slice(&extra);
        r
    };

    // Layer 0: Background
    layer_info.extend_from_slice(&make_layer_record(
        "Background",
        0,
        0,
        1080,
        1920,
        b"norm",
        255,
        false,
        false,
    ));
    // Layer 1: Bound Text
    layer_info.extend_from_slice(&make_layer_record(
        "bind:match.team1_name",
        100,
        200,
        160,
        600,
        b"norm",
        255,
        false,
        true,
    ));
    // Layer 2: Multiply Masked Layer
    layer_info.extend_from_slice(&make_layer_record(
        "Masked Layer",
        0,
        0,
        500,
        500,
        b"mul ",
        128,
        true,
        false,
    ));

    // Channel image raw dummy bytes for each layer channel
    for _ in 0..9 {
        layer_info.extend_from_slice(&0u16.to_be_bytes()); // Compression: Raw
    }

    let mut layer_section = Vec::new();
    layer_section.extend_from_slice(&(layer_info.len() as u32).to_be_bytes());
    layer_section.extend_from_slice(&layer_info);

    bytes.extend_from_slice(&(layer_section.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&layer_section);

    bytes
}

#[test]
fn test_psd_importer_hierarchy_masks_and_text() {
    let psd_bytes = create_test_psd_bytes();
    let (comp, report) =
        PsdImporter::import_bytes(&psd_bytes, None).expect("Failed to import test PSD");

    assert_eq!(comp.width, 1920);
    assert_eq!(comp.height, 1080);
    assert_eq!(report.imported_layers, 3);
    assert!(report.success);

    // PSD stores bottom-to-top, reverse gives top-to-bottom:
    // Index 0: Masked Layer
    // Index 1: bind:match.team1_name
    // Index 2: Background
    assert_eq!(comp.layers.len(), 3);

    // Verify Masked Layer
    let masked_layer = &comp.layers[0];
    assert_eq!(masked_layer.name, "Masked Layer");
    assert_eq!(masked_layer.blend_mode, BlendMode::Multiply);
    assert!(
        masked_layer.mask.is_some(),
        "Layer should have mask attached"
    );

    // Verify Text Layer & Auto Data Binding
    let text_layer = &comp.layers[1];
    assert_eq!(text_layer.name, "bind:match.team1_name");
    match &text_layer.content {
        LayerContent::Text(txt) => {
            assert_eq!(txt.text.evaluate_at(0.0), "{match.team1_name}");
        }
        other => panic!("Expected Text content, got {:?}", other),
    }

    // Verify Report recorded the binding
    assert_eq!(report.bindings_created.len(), 1);
    assert_eq!(
        report.bindings_created[0],
        (
            "bind:match.team1_name".to_string(),
            "match.team1_name".to_string()
        )
    );
}

#[test]
fn test_lottie_importer_keyframes_and_transforms() {
    let lottie_json = r#"{
        "v": "5.7.4",
        "fr": 60,
        "ip": 0,
        "op": 180,
        "w": 1920,
        "h": 1080,
        "nm": "Esports Scoreboard Intro",
        "ddd": 0,
        "assets": [],
        "layers": [
            {
                "ddd": 0,
                "ind": 1,
                "ty": 4,
                "nm": "CardShape",
                "sr": 1,
                "ks": {
                    "o": { "a": 0, "k": 100 },
                    "r": { "a": 0, "k": 0 },
                    "p": {
                        "a": 1,
                        "k": [
                            {
                                "t": 0,
                                "s": [100, 200, 0],
                                "i": { "x": [0.25], "y": [1.0] },
                                "o": { "x": [0.25], "y": [0.0] }
                            },
                            {
                                "t": 60,
                                "s": [500, 400, 0],
                                "i": { "x": [0.25], "y": [1.0] },
                                "o": { "x": [0.25], "y": [0.0] }
                            },
                            {
                                "t": 120,
                                "s": [900, 600, 0]
                            }
                        ]
                    },
                    "a": { "a": 0, "k": [0, 0, 0] },
                    "s": { "a": 0, "k": [100, 100, 100] }
                },
                "shapes": [
                    {
                        "ty": "rc",
                        "s": { "k": [400, 250] }
                    },
                    {
                        "ty": "fl",
                        "c": { "k": [0.2, 0.4, 0.8, 1.0] }
                    }
                ],
                "ip": 0,
                "op": 180
            },
            {
                "ddd": 0,
                "ind": 2,
                "ty": 5,
                "nm": "bind:scoreboard.team1_score",
                "sr": 1,
                "ks": {
                    "o": { "a": 0, "k": 100 },
                    "r": { "a": 0, "k": 0 },
                    "p": { "a": 0, "k": [960, 540, 0] },
                    "a": { "a": 0, "k": [0, 0, 0] },
                    "s": { "a": 0, "k": [100, 100, 100] }
                },
                "t": {
                    "d": {
                        "k": [
                            {
                                "s": { "t": "0" }
                            }
                        ]
                    }
                },
                "ip": 0,
                "op": 180
            }
        ]
    }"#;

    let (comp, report) =
        LottieImporter::import_str(lottie_json).expect("Failed to import test Lottie JSON");

    assert_eq!(comp.width, 1920);
    assert_eq!(comp.height, 1080);
    assert_eq!(comp.frames_per_second, 60.0);
    assert_eq!(comp.duration_seconds, 3.0); // 180 frames / 60 fps = 3.0 seconds
    assert_eq!(comp.layers.len(), 2);

    // Verify Shape Layer animated position keyframes
    let shape_layer = &comp.layers[0];
    assert_eq!(shape_layer.name, "CardShape");

    // Evaluate position over time
    let p_0 = shape_layer.transform.position.evaluate_at(0.0);
    assert_eq!(p_0, Vec3::new(100.0, 200.0, 0.0));

    let p_1 = shape_layer.transform.position.evaluate_at(1.0); // frame 60 / 60 = 1.0s
    assert_eq!(p_1, Vec3::new(500.0, 400.0, 0.0));

    let p_2 = shape_layer.transform.position.evaluate_at(2.0); // frame 120 / 60 = 2.0s
    assert_eq!(p_2, Vec3::new(900.0, 600.0, 0.0));

    // Midpoint between 0.0s and 1.0s should evaluate smoothly via cubic bezier
    let p_mid = shape_layer.transform.position.evaluate_at(0.5);
    assert!(p_mid.x > 100.0 && p_mid.x < 500.0);
    assert!(p_mid.y > 200.0 && p_mid.y < 400.0);

    // Verify Text Layer & Auto Binding
    let text_layer = &comp.layers[1];
    assert_eq!(text_layer.name, "bind:scoreboard.team1_score");
    match &text_layer.content {
        LayerContent::Text(txt) => {
            assert_eq!(txt.text.evaluate_at(0.0), "{scoreboard.team1_score}");
        }
        other => panic!("Expected Text content for bound layer, got {:?}", other),
    }

    assert_eq!(report.bindings_created.len(), 1);
    assert_eq!(
        report.bindings_created[0],
        (
            "bind:scoreboard.team1_score".to_string(),
            "scoreboard.team1_score".to_string()
        )
    );
}

#[test]
fn test_import_report_auditing() {
    let mut report = ImportReport::new("complex_banner.psd", "PSD");
    assert!(!report.has_issues());

    report.add_warning("Text kerning table rasterized");
    assert!(report.has_issues());

    report.add_unsupported("3D Extrusion effect replaced with flat shape");
    assert_eq!(report.unsupported_features.len(), 1);

    report.add_binding("bind:match.timer", "match.timer");
    assert_eq!(report.bindings_created.len(), 1);
}
