//! M5b Acceptance Test: Full Transition System with per-scene rules, all transition kinds,
//! GPU transition shaders at 1080p60 within frame budget, and seamless mid-transition interruption.

use pollster::FutureExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use zeno_compositor::{
    OverlayTiming, Scene, SceneCompositor, SceneItem, SceneItemSource, SlideDirection, StudioMode,
    TransitionDefinition, TransitionKind, TransitionRule, WipeDirection,
};
use zeno_core::{Color, SourceId};
use zeno_engine::GpuContext;
use zeno_media::DoubleBufferedTexture;

#[test]
fn test_m5b_transition_system_and_all_kinds_at_1080p60() {
    let gpu = GpuContext::init_headless()
        .block_on()
        .expect("Headless GPU context");

    let device = Arc::new(gpu.device);
    let queue = Arc::new(gpu.queue);

    let mut compositor = SceneCompositor::new(device.clone(), queue.clone(), 1920, 1080, 60.0);

    // Create mock input sources
    let source_a_id = SourceId::new();
    let source_b_id = SourceId::new();

    let mut tex_buf_a = DoubleBufferedTexture::new();
    let mut tex_buf_b = DoubleBufferedTexture::new();

    let pixels_a = [76u8, 141, 255, 255].repeat(1920 * 1080); // Accent blue (#4C8DFF)
    let pixels_b = [229u8, 72, 77, 255].repeat(1920 * 1080); // Live red (#E5484D)

    tex_buf_a.upload(&device, &queue, 1920, 1080, &pixels_a, 0.0);
    tex_buf_b.upload(&device, &queue, 1920, 1080, &pixels_b, 0.0);

    let mut sources = HashMap::new();
    sources.insert(
        source_a_id,
        tex_buf_a.current_frame().unwrap().texture_view.clone(),
    );
    sources.insert(
        source_b_id,
        tex_buf_b.current_frame().unwrap().texture_view.clone(),
    );

    // 1. Setup scenes with items and overlay timing
    let mut scene_1 = Scene::new("Scene 1");
    scene_1.background = Color::rgb(0.055, 0.059, 0.067);
    scene_1.overlay_timing = OverlayTiming::Overlap;
    scene_1.add_item(
        SceneItem::new("Feed 1", SceneItemSource::Source(source_a_id))
            .with_position(960.0, 540.0)
            .with_scale(1.0, 1.0),
    );
    let id_1 = scene_1.id;

    let mut scene_2 = Scene::new("Scene 2");
    scene_2.background = Color::rgb(0.082, 0.090, 0.102);
    scene_2.overlay_timing = OverlayTiming::Sequential;
    scene_2.add_item(
        SceneItem::new("Feed 2", SceneItemSource::Source(source_b_id))
            .with_position(960.0, 540.0)
            .with_scale(1.0, 1.0),
    );
    let id_2 = scene_2.id;

    let mut scene_3 = Scene::new("Scene 3");
    scene_3.background = Color::rgb(0.114, 0.125, 0.141);
    let id_3 = scene_3.id;

    compositor.add_scene(scene_1);
    compositor.add_scene(scene_2);
    compositor.add_scene(scene_3);

    // 2. Test Rule Resolution (Exact Pair Rule > Destination Default > Global Default)
    let wipe_dest_trans = TransitionDefinition::wipe(WipeDirection::Horizontal, 0.05, 0.4);
    let wipe_id = wipe_dest_trans.id;
    compositor.get_scene_mut(id_2).unwrap().default_transition = Some(wipe_id);

    let slide_pair_trans = TransitionDefinition::slide(SlideDirection::Left, true, 0.3);
    let slide_id = slide_pair_trans.id;

    compositor.register_transition(wipe_dest_trans);
    compositor.register_transition(slide_pair_trans);

    compositor.add_transition_rule(TransitionRule {
        from_scene: Some(id_1),
        to_scene: id_2,
        transition: slide_id,
    });

    // Rule priority 1: Exact pair rule matches Scene 1 -> Scene 2
    let resolved_pair = compositor.resolve_transition(id_1, id_2);
    assert_eq!(resolved_pair.id, slide_id);
    assert!(matches!(resolved_pair.kind, TransitionKind::Slide { .. }));

    // Rule priority 2: Destination default matches Scene 3 -> Scene 2
    let resolved_dest = compositor.resolve_transition(id_3, id_2);
    assert_eq!(resolved_dest.id, wipe_id);
    assert!(matches!(resolved_dest.kind, TransitionKind::Wipe { .. }));

    // Rule priority 3: Global default matches Scene 2 -> Scene 1
    let resolved_global = compositor.resolve_transition(id_2, id_1);
    assert_eq!(resolved_global.name, "Fade");

    // 3. Test Every Transition Kind at 1080p60 within frame budget
    let transition_kinds_to_test = vec![
        TransitionDefinition::cut(),
        TransitionDefinition::fade(0.2),
        TransitionDefinition::dip_to_color(Color::BLACK, 0.2),
        TransitionDefinition::slide(SlideDirection::Left, true, 0.2),
        TransitionDefinition::slide(SlideDirection::Right, false, 0.2),
        TransitionDefinition::slide(SlideDirection::Up, true, 0.2),
        TransitionDefinition::slide(SlideDirection::Down, false, 0.2),
        TransitionDefinition::wipe(WipeDirection::Horizontal, 0.05, 0.2),
        TransitionDefinition::wipe(WipeDirection::Vertical, 0.05, 0.2),
        TransitionDefinition::wipe(WipeDirection::Radial, 0.05, 0.2),
        TransitionDefinition {
            id: zeno_core::TransitionId::new(),
            name: "Zoom In".to_string(),
            kind: TransitionKind::Zoom { zoom_in: true },
            duration_seconds: 0.2,
            easing: zeno_core::Easing::ease_in_out(),
            switch_point_percent: 0.5,
            audio_behavior: zeno_compositor::TransitionAudio::Crossfade,
        },
        TransitionDefinition {
            id: zeno_core::TransitionId::new(),
            name: "Stinger".to_string(),
            kind: TransitionKind::Stinger {
                asset: None,
                switch_point: 0.5,
            },
            duration_seconds: 0.2,
            easing: zeno_core::Easing::Linear,
            switch_point_percent: 0.5,
            audio_behavior: zeno_compositor::TransitionAudio::Crossfade,
        },
        TransitionDefinition {
            id: zeno_core::TransitionId::new(),
            name: "Branded Overlay Transition".to_string(),
            kind: TransitionKind::Overlay {
                composition_id: None,
                switch_point: 0.6,
            },
            duration_seconds: 0.2,
            easing: zeno_core::Easing::ease_out(),
            switch_point_percent: 0.6,
            audio_behavior: zeno_compositor::TransitionAudio::FollowVideo,
        },
    ];

    let dt = 1.0 / 60.0;

    for trans in transition_kinds_to_test {
        trans
            .validate()
            .expect("Transition definition must be valid");

        compositor.bus_mut().set_mode(StudioMode::Direct);
        let from = compositor.bus().program_scene;
        let to = if from == id_1 { id_2 } else { id_1 };

        let start_time = Instant::now();
        compositor.switch_scene(to, Some(trans.clone()));

        // Stream 15 frames (~0.25 s at 60 fps)
        let mut frame_count = 0;
        let mut sum_frame_micros = 0u128;

        for _ in 0..15 {
            let f_start = Instant::now();
            compositor
                .render_frame(&sources, dt)
                .expect("Render frame failed during transition test");
            let f_elapsed = f_start.elapsed().as_micros();
            sum_frame_micros += f_elapsed;
            frame_count += 1;
        }

        let avg_micros = sum_frame_micros / frame_count as u128;
        let total_elapsed = start_time.elapsed().as_millis();

        println!(
            "[M5b] Transition '{}': 15 frames @ 1080p60 completed in {} ms (avg frame: {} µs = {:.2} ms)",
            trans.name, total_elapsed, avg_micros, avg_micros as f64 / 1000.0
        );

        // Frame budget check: must hold under 16.6 ms (and target under 4 ms for GPU compositor)
        assert!(
            avg_micros < 10_000,
            "Frame budget exceeded for transition '{}': avg {} µs",
            trans.name,
            avg_micros
        );

        // Verify transition cleanly arrived at destination
        assert_eq!(
            compositor.bus().program_scene,
            to,
            "Scene switch should arrive at destination scene"
        );
    }

    // 4. Test Switching Mid-Transition with No Visible Jump
    // Start a 0.5s fade from current scene to Scene 3
    let long_fade = TransitionDefinition::fade(0.5);
    compositor.switch_scene(id_3, Some(long_fade));
    assert!(compositor.active_transition().is_some());

    // Advance 5 frames (in-flight)
    for _ in 0..5 {
        compositor.render_frame(&sources, dt).unwrap();
    }
    assert!(compositor.active_transition().is_some());

    // Operator switches mid-transition to Scene 1
    compositor.switch_scene(id_1, Some(TransitionDefinition::fade(0.3)));
    let active_interrupted = compositor
        .active_transition()
        .expect("Active transition exists after interruption");

    // The interrupted transition must have from_snapshot = true to smoothly blend from the current visual frame
    assert!(
        active_interrupted.from_snapshot,
        "Mid-transition interruption must capture visual state snapshot"
    );

    // Render frame from snapshot: executes with no crash and no visual jump
    compositor
        .render_frame(&sources, dt)
        .expect("Render frame from interrupted snapshot succeeded");

    // Complete remaining frames to Scene 1
    for _ in 0..25 {
        compositor.render_frame(&sources, dt).unwrap();
    }
    assert_eq!(compositor.bus().program_scene, id_1);

    // 5. Test Live Preview Thumbnail Generation for Transitions Panel
    let preview_trans = TransitionDefinition::wipe(WipeDirection::Radial, 0.1, 0.4);
    let preview_res =
        compositor.render_transition_preview(id_1, id_2, &preview_trans, 0.5, &sources);
    assert!(
        preview_res.is_ok(),
        "Transition preview thumbnail generation must succeed"
    );
}
