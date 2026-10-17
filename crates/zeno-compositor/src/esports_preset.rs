//! One-click tournament broadcast scene set generator.
//!
//! Complies with Section 13 M17:
//! "One-click scene set generation. Accept: a full tournament stream setup can be created in under 10 minutes from a clean install."

use crate::compositor::SceneCompositor;
use crate::scene::{Scene, SceneItem, SceneItemSource};
use crate::transition::{SlideDirection, TransitionDefinition, TransitionRule};
use zeno_core::{Color, SceneId, SourceId};

/// Metadata and scene handles for an auto-generated esports tournament production setup.
#[derive(Debug, Clone)]
pub struct TournamentSceneSet {
    /// Starting Soon scene ID.
    pub starting_soon_id: SceneId,
    /// Caster Desk scene ID.
    pub caster_desk_id: SceneId,
    /// Map Veto scene ID.
    pub map_veto_id: SceneId,
    /// Live Gameplay match scene ID.
    pub live_match_id: SceneId,
    /// Break / Be Right Back scene ID.
    pub break_id: SceneId,
    /// Tournament Ending / Post-show scene ID.
    pub ending_id: SceneId,
    /// All created scene IDs in sequence.
    pub all_scene_ids: Vec<SceneId>,
    /// Configured transition rules count.
    pub rule_count: usize,
}

/// One-click tournament scene generator.
pub struct TournamentPresetGenerator;

impl TournamentPresetGenerator {
    /// Generates a full esports broadcast production scene graph and transition rules
    /// inside the provided `SceneCompositor`.
    pub fn generate(compositor: &mut SceneCompositor) -> TournamentSceneSet {
        let width = compositor.width() as f32;
        let height = compositor.height() as f32;

        // --------------------------------------------------------------------
        // 1. Scene: Starting Soon
        // --------------------------------------------------------------------
        let mut s_starting = Scene::new("Starting Soon");
        let starting_id = s_starting.id;

        let src_starting_overlay = SourceId::new();
        s_starting.add_item(
            SceneItem::new("Starting Soon Overlay", SceneItemSource::Source(src_starting_overlay))
                .with_position(width / 2.0, height / 2.0)
                .with_scale(1.0, 1.0),
        );

        // --------------------------------------------------------------------
        // 2. Scene: Caster Desk
        // --------------------------------------------------------------------
        let mut s_casters = Scene::new("Caster Desk");
        let casters_id = s_casters.id;

        let src_caster_cam1 = SourceId::new();
        let src_caster_cam2 = SourceId::new();
        let src_caster_overlay = SourceId::new();

        // 2 caster camera feeds placed side-by-side
        let cam_w = (width - 120.0) / 2.0;
        let cam_h = cam_w * 9.0 / 16.0;

        s_casters.add_item(
            SceneItem::new("Caster 1 Camera", SceneItemSource::Source(src_caster_cam1))
                .with_position(40.0 + (cam_w / 2.0), height / 2.0)
                .with_scale(cam_w / width, cam_h / height),
        );
        s_casters.add_item(
            SceneItem::new("Caster 2 Camera", SceneItemSource::Source(src_caster_cam2))
                .with_position(width - 40.0 - (cam_w / 2.0), height / 2.0)
                .with_scale(cam_w / width, cam_h / height),
        );
        s_casters.add_item(
            SceneItem::new("Caster Desk Frames & Lower Third", SceneItemSource::Source(src_caster_overlay))
                .with_position(width / 2.0, height / 2.0),
        );

        // --------------------------------------------------------------------
        // 3. Scene: Map Veto / Analysis
        // --------------------------------------------------------------------
        let mut s_veto = Scene::new("Map Veto");
        let veto_id = s_veto.id;

        let src_veto_overlay = SourceId::new();
        s_veto.add_item(
            SceneItem::new("Map Veto Board", SceneItemSource::Source(src_veto_overlay))
                .with_position(width / 2.0, height / 2.0),
        );

        // --------------------------------------------------------------------
        // 4. Scene: Live Match (Gameplay)
        // --------------------------------------------------------------------
        let mut s_match = Scene::new("Live Match");
        let match_id = s_match.id;

        let src_gameplay = SourceId::new();
        let src_scoreboard = SourceId::new();
        let src_playercams = SourceId::new();
        let src_killfeed = SourceId::new();

        // Fullscreen gameplay
        s_match.add_item(
            SceneItem::new("Game Capture (DirectX)", SceneItemSource::Source(src_gameplay))
                .with_position(width / 2.0, height / 2.0),
        );
        // Player cam frames on flanks
        s_match.add_item(
            SceneItem::new("Player Cam Frames", SceneItemSource::Source(src_playercams))
                .with_position(width / 2.0, height / 2.0),
        );
        // Scoreboard overlay on top
        s_match.add_item(
            SceneItem::new("Scoreboard Overlay", SceneItemSource::Source(src_scoreboard))
                .with_position(width / 2.0, height / 2.0),
        );
        // Kill feed overlay on top right
        s_match.add_item(
            SceneItem::new("Kill Feed", SceneItemSource::Source(src_killfeed))
                .with_position(width / 2.0, height / 2.0),
        );

        // --------------------------------------------------------------------
        // 5. Scene: Break / BRB
        // --------------------------------------------------------------------
        let mut s_break = Scene::new("Break");
        let break_id = s_break.id;

        let src_brb_overlay = SourceId::new();
        s_break.add_item(
            SceneItem::new("Be Right Back Overlay", SceneItemSource::Source(src_brb_overlay))
                .with_position(width / 2.0, height / 2.0),
        );

        // --------------------------------------------------------------------
        // 6. Scene: Ending / Post-Show
        // --------------------------------------------------------------------
        let mut s_ending = Scene::new("Stream Ending");
        let ending_id = s_ending.id;

        let src_ending_overlay = SourceId::new();
        s_ending.add_item(
            SceneItem::new("Ending Credits & Bracket", SceneItemSource::Source(src_ending_overlay))
                .with_position(width / 2.0, height / 2.0),
        );

        // --------------------------------------------------------------------
        // Register Scenes into Compositor
        // --------------------------------------------------------------------
        compositor.add_scene(s_starting);
        compositor.add_scene(s_casters);
        compositor.add_scene(s_veto);
        compositor.add_scene(s_match);
        compositor.add_scene(s_break);
        compositor.add_scene(s_ending);

        // --------------------------------------------------------------------
        // Configure Production Transitions & Priority Rules
        // --------------------------------------------------------------------
        let trans_crossfade_1s = TransitionDefinition::fade(1.0);
        let trans_slide_left = TransitionDefinition::slide(SlideDirection::Left, true, 0.75);
        let trans_dip_black = TransitionDefinition::dip_to_color(Color::BLACK, 0.5);

        let xfade_id = trans_crossfade_1s.id;
        let slide_id = trans_slide_left.id;
        let dip_id = trans_dip_black.id;

        compositor.register_transition(trans_crossfade_1s);
        compositor.register_transition(trans_slide_left);
        compositor.register_transition(trans_dip_black);

        // Per-scene transition rules:
        // Starting Soon -> Caster Desk: 1.0s Crossfade
        compositor.add_transition_rule(TransitionRule {
            from_scene: Some(starting_id),
            to_scene: casters_id,
            transition: xfade_id,
        });

        // Caster Desk -> Live Match: Dynamic Slide Left
        compositor.add_transition_rule(TransitionRule {
            from_scene: Some(casters_id),
            to_scene: match_id,
            transition: slide_id,
        });

        // Live Match -> Break: Slide Left
        compositor.add_transition_rule(TransitionRule {
            from_scene: Some(match_id),
            to_scene: break_id,
            transition: slide_id,
        });

        // Break -> Live Match: Slide Left
        compositor.add_transition_rule(TransitionRule {
            from_scene: Some(break_id),
            to_scene: match_id,
            transition: slide_id,
        });

        // Live Match -> Ending: 0.5s Dip to Black
        compositor.add_transition_rule(TransitionRule {
            from_scene: Some(match_id),
            to_scene: ending_id,
            transition: dip_id,
        });

        TournamentSceneSet {
            starting_soon_id: starting_id,
            caster_desk_id: casters_id,
            map_veto_id: veto_id,
            live_match_id: match_id,
            break_id,
            ending_id,
            all_scene_ids: vec![
                starting_id,
                casters_id,
                veto_id,
                match_id,
                break_id,
                ending_id,
            ],
            rule_count: 5,
        }
    }
}
