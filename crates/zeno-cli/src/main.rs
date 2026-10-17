//! # Zenocast Headless CLI
//!
//! Command-line utility for offline rendering, asset conversion, and validation of `.arx` packages.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use zeno_core::{Color, LayerId, Property, Vec3};
use zeno_engine::{Composition, Layer, LayerContent, TextContent};
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
            let mut comp = Composition::new(1920, 1080, 60.0, 5.0);
            comp.states = vec![
                zeno_engine::AnimationState::new("in", 0.0, 1.0, zeno_engine::StatePlayback::Once),
                zeno_engine::AnimationState::new(
                    "loop",
                    1.0,
                    4.0,
                    zeno_engine::StatePlayback::Loop,
                ),
                zeno_engine::AnimationState::new("out", 4.0, 5.0, zeno_engine::StatePlayback::Once),
            ];

            let layer_id = LayerId::new();
            let mut layer = Layer::new(
                layer_id,
                "Lower Third Name",
                LayerContent::Text(TextContent {
                    text: Property::constant("PRO PLAYER // MATCH WINNER".to_string()),
                    font_family: "Inter".to_string(),
                    font_size: Property::constant(36.0),
                    font_weight: 700,
                    color: Property::constant(Color::WHITE),
                    ..Default::default()
                }),
            );
            layer.transform.position = Property::constant(Vec3::new(100.0, 950.0, 0.0));
            comp.layers.push(layer);

            let pkg = ArxPackage::new(manifest, comp);
            pkg.save(&output).with_context(|| {
                format!("Failed to write sample package to {}", output.display())
            })?;

            println!("Created {}", output.display());
            Ok(())
        }
    }
}
