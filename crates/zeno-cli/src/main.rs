//! # Zenocast Headless CLI
//!
//! Command-line utility for offline rendering, asset conversion, and validation of `.arx` packages.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use pollster::FutureExt;
use std::path::PathBuf;
use zeno_core::{Color, Easing, Keyframe, LayerId, Property, Vec2, Vec3};
use zeno_engine::{
    Composition, Fill, GpuContext, Layer, LayerContent, ShapeContent, ShapeGeometry, TextContent,
    ZenoRenderer,
};
use zeno_format::{ArxPackage, Manifest};

#[derive(Parser)]
#[command(
    name = "zeno-cli",
    version = "0.1.0",
    about = "Zenocast Headless CLI utility"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate an .arx container package
    Validate {
        /// Path to the .arx file to validate
        file: PathBuf,
    },
    /// Generate a sample .arx template for verification
    CreateSample {
        /// Output path for the generated .arx file
        #[arg(default_value = "sample.arx")]
        output: PathBuf,
    },
    /// Render a frame of an .arx package or default animated template at a specified timestamp
    Render {
        /// Optional path to an .arx package file. If omitted, uses built-in animated template.
        #[arg(short, long)]
        file: Option<PathBuf>,
        /// Timestamp in seconds to render (e.g. 1.5)
        #[arg(short, long, default_value_t = 0.0)]
        time: f64,
        /// Output image path (PNG format, e.g. frame.png)
        #[arg(short, long, default_value = "frame.png")]
        out: PathBuf,
        /// Optional canvas width override
        #[arg(long)]
        width: Option<u32>,
        /// Optional canvas height override
        #[arg(long)]
        height: Option<u32>,
    },
}

/// Builds a standard 1080p animated esports lower third with In, Loop, and Out keyframes.
pub fn build_sample_animated_composition() -> Composition {
    let mut comp = Composition::new(1920, 1080, 60.0, 5.0);
    comp.states = vec![
        zeno_engine::AnimationState::new("in", 0.0, 1.0, zeno_engine::StatePlayback::Once),
        zeno_engine::AnimationState::new("loop", 1.0, 4.0, zeno_engine::StatePlayback::Loop),
        zeno_engine::AnimationState::new("out", 4.0, 5.0, zeno_engine::StatePlayback::Once),
    ];

    // Layer 1: Background banner box
    let banner_id = LayerId::new();
    let mut banner_layer = Layer::new(
        banner_id,
        "Lower Third Banner",
        LayerContent::Shape(ShapeContent {
            geometry: ShapeGeometry::Rectangle {
                size: Property::constant(Vec2::new(760.0, 130.0)),
            },
            fill: Some(Fill::Solid(Property::constant(
                Color::from_hex("#15171AFF").unwrap(),
            ))),
            stroke: None,
        }),
    );

    // Sliding In from X = -800 at t=0 to X = 120 at t=1, hold until t=4, slide out to X = -800 at t=5
    banner_layer.transform.position = Property::keyframes(vec![
        Keyframe::new(0.0, Vec3::new(-800.0, 860.0, 0.0), Easing::ease_out()),
        Keyframe::linear(1.0, Vec3::new(120.0, 860.0, 0.0)),
        Keyframe::new(4.0, Vec3::new(120.0, 860.0, 0.0), Easing::ease_in()),
        Keyframe::linear(5.0, Vec3::new(-800.0, 860.0, 0.0)),
    ]);

    // Opacity: fades in 0% -> 100% over [0.0..0.8] and fades out 100% -> 0% over [4.2..5.0]
    banner_layer.transform.opacity_percent = Property::keyframes(vec![
        Keyframe::linear(0.0, 0.0),
        Keyframe::linear(0.8, 100.0),
        Keyframe::linear(4.2, 100.0),
        Keyframe::linear(5.0, 0.0),
    ]);

    comp.layers.push(banner_layer);

    // Layer 2: Red accent bar (child of banner)
    let accent_id = LayerId::new();
    let mut accent_layer = Layer::new(
        accent_id,
        "Accent Strip",
        LayerContent::Shape(ShapeContent {
            geometry: ShapeGeometry::Rectangle {
                size: Property::constant(Vec2::new(8.0, 130.0)),
            },
            fill: Some(Fill::Solid(Property::constant(
                Color::from_hex("#E5484DFF").unwrap(),
            ))),
            stroke: None,
        }),
    );
    accent_layer.parent = Some(banner_id);
    accent_layer.transform.position = Property::constant(Vec3::new(0.0, 0.0, 0.0));
    comp.layers.push(accent_layer);

    // Layer 3: Title text (child of banner)
    let title_id = LayerId::new();
    let mut title_layer = Layer::new(
        title_id,
        "Player Name",
        LayerContent::Text(TextContent {
            text: Property::constant("ZENOCAST // LIVE STREAM".to_string()),
            font_family: "Inter".to_string(),
            font_size: Property::constant(32.0),
            font_weight: 700,
            color: Property::constant(Color::WHITE),
            ..Default::default()
        }),
    );
    title_layer.parent = Some(banner_id);
    title_layer.transform.position = Property::constant(Vec3::new(32.0, 24.0, 0.0));
    comp.layers.push(title_layer);

    // Layer 4: Subtitle text (child of banner)
    let subtitle_id = LayerId::new();
    let mut subtitle_layer = Layer::new(
        subtitle_id,
        "Match Stats",
        LayerContent::Text(TextContent {
            text: Property::constant("CHAMPIONSHIP SERIES - MATCH 1".to_string()),
            font_family: "JetBrains Mono".to_string(),
            font_size: Property::constant(18.0),
            font_weight: 400,
            color: Property::constant(Color::from_hex("#4C8DFFFF").unwrap()),
            ..Default::default()
        }),
    );
    subtitle_layer.parent = Some(banner_id);
    subtitle_layer.transform.position = Property::constant(Vec3::new(32.0, 78.0, 0.0));
    comp.layers.push(subtitle_layer);

    comp
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate { file } => {
            println!("Validating .arx package: {}", file.display());
            if !file.exists() {
                bail!("File not found: {}", file.display());
            }

            match ArxPackage::validate_file(&file) {
                Ok(report) => {
                    println!("Package Name:        {}", report.manifest.name);
                    println!("Author:              {}", report.manifest.author);
                    println!("Format Version:      {}", report.manifest.format_version);
                    println!(
                        "Canvas:              {}x{} @ {} fps",
                        report.manifest.width, report.manifest.height, report.manifest.fps
                    );
                    println!("Layers:              {}", report.layer_count);
                    println!("Variables:           {}", report.variable_count);
                    println!("Assets:              {}", report.asset_count);

                    for warning in &report.warnings {
                        eprintln!("[WARN] {}", warning);
                    }

                    if report.is_valid {
                        println!("\nSUCCESS: Package '{}' is valid.", file.display());
                        Ok(())
                    } else {
                        for error in &report.errors {
                            eprintln!("[ERROR] {}", error);
                        }
                        bail!("Validation failed with {} error(s)", report.errors.len());
                    }
                }
                Err(err) => {
                    eprintln!("[ERROR] Failed to open .arx package: {}", err);
                    bail!(err);
                }
            }
        }
        Commands::CreateSample { output } => {
            println!("Generating sample .arx package at: {}", output.display());
            let manifest =
                Manifest::new("Esports Lower Third", "Zenocast System", 1920, 1080, 60.0);
            let comp = build_sample_animated_composition();

            let pkg = ArxPackage::new(manifest, comp);
            pkg.save(&output).with_context(|| {
                format!("Failed to write sample package to {}", output.display())
            })?;

            println!("Created {}", output.display());
            Ok(())
        }
        Commands::Render {
            file,
            time,
            out,
            width,
            height,
        } => {
            let (comp, render_w, render_h) = if let Some(path) = file {
                println!("Loading package from {}", path.display());
                let pkg = ArxPackage::open(&path).with_context(|| {
                    format!("Failed to read .arx package at {}", path.display())
                })?;
                let w = width.unwrap_or(pkg.manifest.width);
                let h = height.unwrap_or(pkg.manifest.height);
                (pkg.scene, w, h)
            } else {
                let comp = build_sample_animated_composition();
                let w = width.unwrap_or(comp.width);
                let h = height.unwrap_or(comp.height);
                (comp, w, h)
            };

            println!(
                "Rendering frame at t={:.3}s ({}x{}) to {}",
                time,
                render_w,
                render_h,
                out.display()
            );

            let gpu = GpuContext::init_headless()
                .block_on()
                .map_err(|e| anyhow::anyhow!("Failed to initialize headless GPU device: {}", e))?;

            let mut renderer =
                ZenoRenderer::new(&gpu.device, &gpu.queue, wgpu::TextureFormat::Rgba8UnormSrgb);

            let rendered_image =
                renderer.render_to_image(&gpu.device, &gpu.queue, &comp, time, render_w, render_h);

            rendered_image
                .save(&out)
                .with_context(|| format!("Failed to save rendered frame to {}", out.display()))?;

            println!("Successfully saved frame to {}", out.display());
            Ok(())
        }
    }
}
