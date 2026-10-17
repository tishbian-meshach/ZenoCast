//! # zeno-compositor
//!
//! Live GPU scene compositing: scene tree, source transforms, preview/program buses,
//! GPU transition shaders (cut, fade, dip, slide, wipe, zoom, stinger, overlay), and T-bar mixing.

pub mod bus;
pub mod compositor;
pub mod error;
pub mod esports_preset;
pub mod overlay_source;
pub mod pipeline;
pub mod scene;
pub mod transition;

// Re-exports
pub use bus::{BusManager, StudioMode};
pub use compositor::SceneCompositor;
pub use error::CompositorError;
pub use esports_preset::{TournamentPresetGenerator, TournamentSceneSet};
pub use overlay_source::ArxOverlaySource;
pub use pipeline::{CompositorGpuPipeline, ItemUniforms, TransitionUniforms};
pub use scene::{
    Crop, ItemFilter, ItemTransform, OverlayTiming, Scene, SceneItem, SceneItemSource,
};
pub use transition::{
    ActiveTransition, SlideDirection, TransitionAudio, TransitionDefinition, TransitionKind,
    TransitionRule, WipeDirection,
};

// Re-export core IDs for convenience
pub use zeno_core::{SceneId, SourceId, TransitionId};

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;
    use zeno_core::{Color, SceneId, SourceId};
    use zeno_engine::BlendMode;

    #[test]
    fn test_scene_and_item_transform_and_crop() {
        let mut scene = Scene::new("Gameplay Scene");
        assert_eq!(scene.name, "Gameplay Scene");
        assert!(scene.items.is_empty());

        let source_id = SourceId::new();
        let item = SceneItem::new("Game Capture", SceneItemSource::Source(source_id))
            .with_position(960.0, 540.0)
            .with_scale(1.0, 1.0)
            .with_crop(10.0, 20.0, 10.0, 20.0)
            .with_blend_mode(BlendMode::Normal);

        let item_id = item.id;
        scene.add_item(item);
        assert_eq!(scene.items.len(), 1);

        let retrieved = scene.get_item(item_id).expect("Item found by id");
        assert_eq!(retrieved.name, "Game Capture");
        assert_eq!(retrieved.transform.position, Vec2::new(960.0, 540.0));
        assert_eq!(retrieved.crop.left, 10.0);
        assert_eq!(retrieved.crop.top, 20.0);

        // Serialization roundtrip
        let json = serde_json::to_string(&scene).expect("Serialize scene");
        let deserialized: Scene = serde_json::from_str(&json).expect("Deserialize scene");
        assert_eq!(deserialized.id, scene.id);
        assert_eq!(deserialized.items.len(), 1);
        assert_eq!(deserialized.items[0].name, "Game Capture");
    }

    #[test]
    fn test_transition_rule_resolution_hierarchy() {
        use std::sync::Arc;
        use zeno_engine::GpuContext;

        let gpu = pollster::block_on(GpuContext::init_headless()).expect("Headless GPU");
        let mut compositor =
            SceneCompositor::new(Arc::new(gpu.device), Arc::new(gpu.queue), 1920, 1080, 60.0);

        let scene_a = Scene::new("Scene A");
        let id_a = scene_a.id;
        let mut scene_b = Scene::new("Scene B");
        let id_b = scene_b.id;
        let scene_c = Scene::new("Scene C");
        let id_c = scene_c.id;

        // Custom destination default transition for Scene B (Wipe)
        let wipe_trans = TransitionDefinition::wipe(WipeDirection::Horizontal, 0.1, 0.5);
        let wipe_id = wipe_trans.id;
        scene_b.default_transition = Some(wipe_id);

        // Custom pair rule from Scene A to Scene C (DipToColor)
        let dip_trans = TransitionDefinition::dip_to_color(Color::BLACK, 0.8);
        let dip_id = dip_trans.id;

        compositor.register_transition(wipe_trans.clone());
        compositor.register_transition(dip_trans.clone());

        compositor.add_scene(scene_a);
        compositor.add_scene(scene_b);
        compositor.add_scene(scene_c);

        compositor.add_transition_rule(TransitionRule {
            from_scene: Some(id_a),
            to_scene: id_c,
            transition: dip_id,
        });

        // 1. Exact pair: Scene A -> Scene C should resolve to DipToColor (Rule priority 1)
        let resolved_ac = compositor.resolve_transition(id_a, id_c);
        assert_eq!(resolved_ac.id, dip_id);

        // 2. Destination default: Scene A -> Scene B should resolve to Wipe (Rule priority 2)
        let resolved_ab = compositor.resolve_transition(id_a, id_b);
        assert_eq!(resolved_ab.id, wipe_id);

        // 3. Global default: Scene B -> Scene A should resolve to Global Default (Fade) (Rule priority 3)
        let resolved_ba = compositor.resolve_transition(id_b, id_a);
        assert_eq!(resolved_ba.name, "Fade");
    }

    #[test]
    fn test_bus_manager_modes_and_t_bar() {
        let scene1 = SceneId::new();
        let scene2 = SceneId::new();

        let mut bus = BusManager::new(scene1);
        assert_eq!(bus.program_scene, scene1);
        assert_eq!(bus.preview_scene, scene1);
        assert_eq!(bus.mode, StudioMode::Studio);

        bus.set_preview(scene2);
        assert_eq!(bus.preview_scene, scene2);
        assert_eq!(bus.program_scene, scene1);

        bus.swap_buses();
        assert_eq!(bus.program_scene, scene2);
        assert_eq!(bus.preview_scene, scene1);

        bus.set_t_bar(0.65);
        assert_eq!(bus.t_bar_position, 0.65);
    }

    #[test]
    fn test_active_transition_lifecycle_and_easing() {
        let from = SceneId::new();
        let to = SceneId::new();
        let trans = TransitionDefinition::fade(1.0);

        let mut active = ActiveTransition::new(from, to, trans);
        assert_eq!(active.linear_progress(), 0.0);
        assert!(!active.is_finished());

        active.tick(0.5);
        assert!((active.linear_progress() - 0.5).abs() < 1e-4);
        assert!(!active.is_finished());

        active.tick(0.5);
        assert!((active.linear_progress() - 1.0).abs() < 1e-4);
        assert!(active.is_finished());
    }
}
