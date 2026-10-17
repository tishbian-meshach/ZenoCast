//! M5 Acceptance Test: Scene Compositor at 1080p60 holding frame budget during transitions.

use pollster::FutureExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use zeno_compositor::{
    Scene, SceneCompositor, SceneItem, SceneItemSource, StudioMode, TransitionDefinition,
};
use zeno_core::{Color, SourceId};
use zeno_engine::GpuContext;
use zeno_media::DoubleBufferedTexture;

#[test]
fn test_m5_scene_compositor_switching_and_frame_budget() {
    let gpu = GpuContext::init_headless()
        .block_on()
        .expect("Headless GPU context");

    let device = Arc::new(gpu.device);
    let queue = Arc::new(gpu.queue);

    let mut compositor = SceneCompositor::new(device.clone(), queue.clone(), 1920, 1080, 60.0);

    // Create mock input video sources
    let source_a_id = SourceId::new();
    let source_b_id = SourceId::new();

    let mut tex_buffer_a = DoubleBufferedTexture::new();
    let mut tex_buffer_b = DoubleBufferedTexture::new();

    // 1920x1080 solid frames for sources
    let dummy_pixels_a = [76u8, 141, 255, 255].repeat(1920 * 1080); // Accent blue
    let dummy_pixels_b = [229u8, 72, 77, 255].repeat(1920 * 1080); // Live red

    tex_buffer_a.upload(&device, &queue, 1920, 1080, &dummy_pixels_a, 0.0);
    tex_buffer_b.upload(&device, &queue, 1920, 1080, &dummy_pixels_b, 0.0);

    let frame_a = tex_buffer_a.current_frame().expect("Frame A");
    let frame_b = tex_buffer_b.current_frame().expect("Frame B");

    let mut sources = HashMap::new();
    sources.insert(source_a_id, frame_a.texture_view.clone());
    sources.insert(source_b_id, frame_b.texture_view.clone());

    // 1. Build Scene A ("Starting Soon")
    let mut scene_a = Scene::new("Starting Soon");
    scene_a.background = Color::from_hex("#0E0F11FF").unwrap();
    let item_a = SceneItem::new("Background Feed", SceneItemSource::Source(source_a_id))
        .with_position(960.0, 540.0)
        .with_scale(1.0, 1.0);
    scene_a.add_item(item_a);
    let id_a = scene_a.id;

    // 2. Build Scene B ("Main Match")
    let mut scene_b = Scene::new("Main Match");
    scene_b.background = Color::from_hex("#15171AFF").unwrap();
    let item_b = SceneItem::new("Gameplay", SceneItemSource::Source(source_b_id))
        .with_position(960.0, 540.0)
        .with_scale(1.0, 1.0)
        .with_crop(40.0, 40.0, 40.0, 40.0); // 40px crop
    scene_b.add_item(item_b);
    let id_b = scene_b.id;

    // 3. Build Scene C ("Analyst Desk")
    let mut scene_c = Scene::new("Analyst Desk");
    scene_c.background = Color::from_hex("#1D2024FF").unwrap();
    let id_c = scene_c.id;

    compositor.add_scene(scene_a);
    compositor.add_scene(scene_b);
    compositor.add_scene(scene_c);

    // Initial bus state
    compositor.bus_mut().set_mode(StudioMode::Studio);
    compositor.bus_mut().set_program(id_a);
    compositor.bus_mut().set_preview(id_b);

    assert_eq!(compositor.bus().program_scene, id_a);
    assert_eq!(compositor.bus().preview_scene, id_b);

    // Register a 300 ms fade transition (0.3 s = 18 frames at 60 fps)
    let fade_trans = TransitionDefinition::fade(0.3);
    compositor.register_transition(fade_trans.clone());

    // Trigger Auto transition from Scene A to Scene B
    compositor.auto_transition(Some(fade_trans));
    assert!(compositor.active_transition().is_some());

    // Run 60 frames (1.0 second) of live compositing at 1080p60
    let total_frames = 60;
    let dt = 1.0 / 60.0;
    let mut total_render_micros = 0u128;
    let mut max_frame_micros = 0u128;

    for frame_idx in 0..total_frames {
        let frame_start = Instant::now();

        compositor
            .render_frame(&sources, dt)
            .expect("Render frame failed");

        let frame_elapsed = frame_start.elapsed().as_micros();
        total_render_micros += frame_elapsed;
        if frame_elapsed > max_frame_micros {
            max_frame_micros = frame_elapsed;
        }

        // Verify transition completion after ~18 frames (0.3 seconds)
        if frame_idx >= 20 {
            assert!(
                compositor.active_transition().is_none(),
                "Transition should have completed by frame {}",
                frame_idx
            );
            assert_eq!(
                compositor.bus().program_scene,
                id_b,
                "Program scene must be Scene B after transition"
            );
        }
    }

    let avg_micros = total_render_micros / total_frames as u128;
    println!(
        "[M5 Acceptance] 1080p60 Scene Compositor: 60 frames rendered during fade transition. Avg render: {} µs ({:.2} ms), Max: {} µs ({:.2} ms)",
        avg_micros,
        avg_micros as f64 / 1000.0,
        max_frame_micros,
        max_frame_micros as f64 / 1000.0
    );

    // Frame budget requirement: 16.6 ms (16,666 µs) for 60 fps. Target under 4 ms for GPU compositor.
    assert!(
        avg_micros < 10_000,
        "Compositor frame time exceeded budget: avg was {} µs",
        avg_micros
    );

    // 4. Test Mid-Transition Interruption
    // Trigger transition from B back to A
    let return_fade = TransitionDefinition::fade(0.5);
    compositor.bus_mut().set_preview(id_a);
    compositor.auto_transition(Some(return_fade));
    assert!(compositor.active_transition().is_some());

    // Advance 5 frames (mid-transition)
    for _ in 0..5 {
        compositor.render_frame(&sources, dt).unwrap();
    }
    assert!(compositor.active_transition().is_some());

    // Switch mid-transition to Scene C: cues C and begins transition
    compositor.bus_mut().set_preview(id_c);
    compositor.auto_transition(Some(TransitionDefinition::fade(0.3)));
    assert!(compositor.active_transition().is_some());

    // Complete transition to C
    for _ in 0..30 {
        compositor.render_frame(&sources, dt).unwrap();
    }
    assert_eq!(compositor.bus().program_scene, id_c);

    // 5. Test T-Bar manual transition from Scene C to Scene A
    compositor.bus_mut().set_preview(id_a);
    compositor.set_t_bar(0.5);
    assert_eq!(compositor.bus().t_bar_position, 0.5);
    assert!(compositor.active_transition().is_some());

    // Set T-bar to 1.0 (completion)
    compositor.set_t_bar(1.0);
    assert!(compositor.active_transition().is_none());
    assert_eq!(compositor.bus().program_scene, id_a);
}
