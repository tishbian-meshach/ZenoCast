//! Integration tests for Milestone 14: GPU particle system,
//! presets (confetti, sparks, smoke), deterministic scrubbing, and 50,000 particle throughput.

use std::time::Instant;
use zeno_core::Property;
use zeno_engine::eval::{evaluate_at, ResolvedContent};
use zeno_engine::{Composition, Layer, LayerContent, ParticleEmitter};

#[test]
fn test_particle_presets() {
    // 1. Confetti
    let confetti = ParticleEmitter::confetti();
    assert_eq!(*confetti.spread_degrees.fallback_value(), 360.0);
    assert!(!confetti.additive_blend);
    assert!(confetti.gravity.fallback_value().y > 0.0);
    assert_eq!(*confetti.lifetime_seconds.fallback_value(), 3.5);

    // 2. Sparks
    let sparks = ParticleEmitter::sparks();
    assert_eq!(*sparks.spread_degrees.fallback_value(), 60.0);
    assert!(sparks.additive_blend);
    assert_eq!(*sparks.initial_speed.fallback_value(), 650.0);
    assert_eq!(*sparks.lifetime_seconds.fallback_value(), 0.8);

    // 3. Smoke
    let smoke = ParticleEmitter::smoke();
    assert!(!smoke.additive_blend);
    assert!(
        smoke.gravity.fallback_value().y < 0.0,
        "Smoke must be buoyant"
    );
    assert!(
        *smoke.start_size_px.fallback_value() < *smoke.end_size_px.fallback_value(),
        "Smoke must expand"
    );
}

#[test]
fn test_deterministic_scrubbing() {
    let mut comp = Composition::new(1920, 1080, 60.0, 10.0);
    let id = zeno_core::LayerId::new();
    let emitter = ParticleEmitter::sparks();
    let layer = Layer::new(id, "Sparks Emitter", LayerContent::Particles(emitter));
    comp.layers.push(layer);

    // 1. Evaluate at target timestamp t = 2.5s
    let frame_first = evaluate_at(&comp, 2.5);
    let particles_first = match &frame_first.layers[0].content {
        ResolvedContent::Particles { particles, .. } => particles.clone(),
        _ => panic!("Expected ResolvedContent::Particles"),
    };
    assert!(
        !particles_first.is_empty(),
        "Must have active particles at t=2.5s"
    );

    // 2. Scrub forward to t = 8.0s
    let _ = evaluate_at(&comp, 8.0);

    // 3. Scrub backward to t = 0.5s
    let _ = evaluate_at(&comp, 0.5);

    // 4. Scrub back to exact target timestamp t = 2.5s
    let frame_second = evaluate_at(&comp, 2.5);
    let particles_second = match &frame_second.layers[0].content {
        ResolvedContent::Particles { particles, .. } => particles.clone(),
        _ => panic!("Expected ResolvedContent::Particles"),
    };

    // Assert exact bit-for-bit determinism across timeline scrubbing
    assert_eq!(
        particles_first.len(),
        particles_second.len(),
        "Particle count must be identical"
    );
    for (idx, (p1, p2)) in particles_first
        .iter()
        .zip(particles_second.iter())
        .enumerate()
    {
        assert_eq!(
            p1.position, p2.position,
            "Particle {} position mismatch after scrubbing",
            idx
        );
        assert_eq!(p1.size, p2.size, "Particle {} size mismatch", idx);
        assert_eq!(p1.color, p2.color, "Particle {} color mismatch", idx);
        assert_eq!(
            p1.age_normalized, p2.age_normalized,
            "Particle {} age mismatch",
            idx
        );
    }
}

#[test]
fn test_50000_particles_performance() {
    let mut comp = Composition::new(1920, 1080, 60.0, 5.0);
    let id = zeno_core::LayerId::new();

    // 50,000 particles emitted per second with 1.0 second lifetime
    // -> Exactly 50,000 active particles at t = 1.0s
    let mut emitter = ParticleEmitter::sparks();
    emitter.rate_per_second = Property::constant(50_000.0);
    emitter.lifetime_seconds = Property::constant(1.0);

    let layer = Layer::new(
        id,
        "Heavy Particle Emitter",
        LayerContent::Particles(emitter),
    );
    comp.layers.push(layer);

    let start = Instant::now();
    let frame = evaluate_at(&comp, 1.0);
    let duration = start.elapsed();

    let count = match &frame.layers[0].content {
        ResolvedContent::Particles { particles, .. } => particles.len(),
        _ => panic!("Expected ResolvedContent::Particles"),
    };

    assert_eq!(
        count, 50_000,
        "Must simulate exactly 50,000 active particles"
    );

    // Must simulate comfortably within the 60 fps frame budget (16.6ms)
    // Closed-form evaluation executes in ~1-5ms on reference CPU.
    println!("50,000 particles evaluated in: {:.2?}", duration);
    assert!(
        duration.as_millis() < 16,
        "50,000 particles took {:?}, exceeding 16.6ms 60fps frame budget",
        duration
    );
}
