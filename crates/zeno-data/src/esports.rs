//! Esports pack: tournament match models, team/roster manager, and broadcast overlay templates.
//!
//! Complies with Section 13 M17:
//! - Pre-packaged broadcast motion-graphics templates in `.arx` container format.
//! - Team & roster manager with esports team presets and player profiles.
//! - Automatic synchronization into the live reactive `DataStore`.

use crate::store::DataStore;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use zeno_core::{Color, LayerId, Property, Vec2, Vec3};
use zeno_engine::model::{
    Composition, Fill, Layer, LayerContent, ShapeContent, ShapeGeometry, TextContent,
};
use zeno_format::{ArxPackage, FormatError, Manifest};
// use crate::value::DataValue;

/// An esports competitor player profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    /// In-game pseudonym/handle (e.g. "TenZ", "Boaster").
    pub handle: String,
    /// Real name (e.g. "Tyson Ngo", "Jake Howlett").
    pub real_name: String,
    /// Team role (e.g. "Duelist / Initiator", "IGL / Controller").
    pub role: String,
    /// Relative or absolute path to player headshot photo.
    pub photo_path: String,
}

/// An esports team profile with colors, branding, and active roster.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Team {
    /// Unique team slug/identifier (e.g. "sentinels").
    pub id: String,
    /// Full team organization name (e.g. "Sentinels").
    pub name: String,
    /// 3-4 letter uppercase tag (e.g. "SEN").
    pub tag: String,
    /// Primary broadcast theme color.
    pub primary_color: Color,
    /// Secondary accent color.
    pub secondary_color: Color,
    /// Relative or absolute path to high-res team logo.
    pub logo_path: String,
    /// Active 5-player roster plus substitutes.
    pub roster: Vec<Player>,
}

/// An on-air talent caster or host profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Caster {
    /// Display name (e.g. "Lauren Scott").
    pub name: String,
    /// On-air handle (e.g. "Pansy").
    pub handle: String,
    /// Social media username (e.g. "@pansy").
    pub social: String,
}

/// Series format classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeriesFormat {
    /// Best of 1 match.
    BO1,
    /// Best of 3 match.
    BO3,
    /// Best of 5 match (Grand Finals).
    BO5,
}

impl SeriesFormat {
    /// Display label for on-air graphics.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BO1 => "BEST OF 1",
            Self::BO3 => "BEST OF 3",
            Self::BO5 => "BEST OF 5",
        }
    }
}

/// Full configuration for an esports tournament match broadcast.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TournamentMatch {
    pub tournament_name: String,
    pub match_title: String,
    pub series_format: SeriesFormat,
    pub team1: Team,
    pub team2: Team,
    pub team1_score: u32,
    pub team2_score: u32,
    pub casters: Vec<Caster>,
    pub map_pool: Vec<String>,
}

/// Central manager for team rosters, player catalogs, and tournament presets.
pub struct RosterManager {
    teams: Vec<Team>,
    casters: Vec<Caster>,
}

impl RosterManager {
    /// Creates a roster manager initialized with standard esports presets.
    pub fn new() -> Self {
        let mut mgr = Self {
            teams: Vec::new(),
            casters: Vec::new(),
        };
        mgr.load_builtin_presets();
        mgr
    }

    /// Load default esports organizations and talent.
    pub fn load_builtin_presets(&mut self) {
        self.teams = vec![
            Team {
                id: "sentinels".to_string(),
                name: "Sentinels".to_string(),
                tag: "SEN".to_string(),
                primary_color: Color::rgb(0.9, 0.2, 0.2), // Red
                secondary_color: Color::WHITE,
                logo_path: "assets/logos/sen.png".to_string(),
                roster: vec![
                    Player {
                        handle: "TenZ".into(),
                        real_name: "Tyson Ngo".into(),
                        role: "Initiator".into(),
                        photo_path: "".into(),
                    },
                    Player {
                        handle: "zekken".into(),
                        real_name: "Zachary Patrone".into(),
                        role: "Duelist".into(),
                        photo_path: "".into(),
                    },
                    Player {
                        handle: "Sacy".into(),
                        real_name: "Gustavo Rossi".into(),
                        role: "Initiator".into(),
                        photo_path: "".into(),
                    },
                    Player {
                        handle: "johnqt".into(),
                        real_name: "Mohamed Amine Ouarid".into(),
                        role: "IGL / Sentinel".into(),
                        photo_path: "".into(),
                    },
                    Player {
                        handle: "Zellsis".into(),
                        real_name: "Jordan Montemurro".into(),
                        role: "Flex / Controller".into(),
                        photo_path: "".into(),
                    },
                ],
            },
            Team {
                id: "fnatic".to_string(),
                name: "Fnatic".to_string(),
                tag: "FNC".to_string(),
                primary_color: Color::rgb(0.95, 0.6, 0.1), // Orange
                secondary_color: Color::BLACK,
                logo_path: "assets/logos/fnc.png".to_string(),
                roster: vec![
                    Player {
                        handle: "Boaster".into(),
                        real_name: "Jake Howlett".into(),
                        role: "IGL".into(),
                        photo_path: "".into(),
                    },
                    Player {
                        handle: "Derke".into(),
                        real_name: "Nikita Sirmitev".into(),
                        role: "Duelist".into(),
                        photo_path: "".into(),
                    },
                    Player {
                        handle: "Alfajer".into(),
                        real_name: "Emir Ali Beder".into(),
                        role: "Sentinel".into(),
                        photo_path: "".into(),
                    },
                    Player {
                        handle: "Chronicle".into(),
                        real_name: "Timofey Khromov".into(),
                        role: "Initiator".into(),
                        photo_path: "".into(),
                    },
                    Player {
                        handle: "Leo".into(),
                        real_name: "Leo Jannesson".into(),
                        role: "Initiator".into(),
                        photo_path: "".into(),
                    },
                ],
            },
            Team {
                id: "cloud9".to_string(),
                name: "Cloud9".to_string(),
                tag: "C9".to_string(),
                primary_color: Color::rgb(0.3, 0.55, 1.0), // Blue
                secondary_color: Color::WHITE,
                logo_path: "assets/logos/c9.png".to_string(),
                roster: vec![
                    Player {
                        handle: "OXY".into(),
                        real_name: "Francis Hoang".into(),
                        role: "Duelist".into(),
                        photo_path: "".into(),
                    },
                    Player {
                        handle: "vanity".into(),
                        real_name: "Anthony Malaspina".into(),
                        role: "IGL".into(),
                        photo_path: "".into(),
                    },
                    Player {
                        handle: "Xeppaa".into(),
                        real_name: "Erick Bach".into(),
                        role: "Flex".into(),
                        photo_path: "".into(),
                    },
                    Player {
                        handle: "moose".into(),
                        real_name: "Kaleb Jayne".into(),
                        role: "Sentinel".into(),
                        photo_path: "".into(),
                    },
                    Player {
                        handle: "runi".into(),
                        real_name: "Dylan Cade".into(),
                        role: "Initiator".into(),
                        photo_path: "".into(),
                    },
                ],
            },
            Team {
                id: "t1".to_string(),
                name: "T1".to_string(),
                tag: "T1".to_string(),
                primary_color: Color::rgb(0.85, 0.15, 0.2), // Crimson
                secondary_color: Color::BLACK,
                logo_path: "assets/logos/t1.png".to_string(),
                roster: vec![
                    Player {
                        handle: "Sayaplayer".into(),
                        real_name: "Ha Jung-woo".into(),
                        role: "Duelist".into(),
                        photo_path: "".into(),
                    },
                    Player {
                        handle: "Carpe".into(),
                        real_name: "Lee Jae-hyeok".into(),
                        role: "Initiator".into(),
                        photo_path: "".into(),
                    },
                    Player {
                        handle: "xeta".into(),
                        real_name: "Son Seon-ho".into(),
                        role: "Flex".into(),
                        photo_path: "".into(),
                    },
                    Player {
                        handle: "iZu".into(),
                        real_name: "Ham Woo-joo".into(),
                        role: "Controller".into(),
                        photo_path: "".into(),
                    },
                    Player {
                        handle: "Rossy".into(),
                        real_name: "Daniel Abedrabbo".into(),
                        role: "IGL".into(),
                        photo_path: "".into(),
                    },
                ],
            },
        ];

        self.casters = vec![
            Caster {
                name: "Lauren Scott".into(),
                handle: "Pansy".into(),
                social: "@pansy".into(),
            },
            Caster {
                name: "Michael Robins".into(),
                handle: "hypoc".into(),
                social: "@hypoc".into(),
            },
            Caster {
                name: "Josh Hough".into(),
                handle: "Sideshow".into(),
                social: "@SideshowGaming".into(),
            },
            Caster {
                name: "Brennon Hook".into(),
                handle: "Bren".into(),
                social: "@BrenCasts".into(),
            },
        ];
    }

    /// Returns all registered teams.
    pub fn teams(&self) -> &[Team] {
        &self.teams
    }

    /// Finds a team by its ID or tag.
    pub fn find_team(&self, query: &str) -> Option<&Team> {
        let q = query.to_lowercase();
        self.teams
            .iter()
            .find(|t| t.id.to_lowercase() == q || t.tag.to_lowercase() == q)
    }

    /// Default grand finals tournament match preset.
    pub fn default_match(&self) -> TournamentMatch {
        let team1 = self
            .find_team("SEN")
            .cloned()
            .unwrap_or_else(|| self.teams[0].clone());
        let team2 = self
            .find_team("FNC")
            .cloned()
            .unwrap_or_else(|| self.teams[1].clone());

        TournamentMatch {
            tournament_name: "ZENOCAST MASTERS 2026".to_string(),
            match_title: "GRAND FINALS".to_string(),
            series_format: SeriesFormat::BO5,
            team1,
            team2,
            team1_score: 2,
            team2_score: 1,
            casters: self.casters.clone(),
            map_pool: vec![
                "Ascent".into(),
                "Bind".into(),
                "Haven".into(),
                "Sunset".into(),
                "Split".into(),
            ],
        }
    }

    /// Writes all tournament match fields into the live data store.
    pub fn sync_to_data_store(&self, match_info: &TournamentMatch, store: &DataStore) {
        store.set("tournament.name", match_info.tournament_name.clone());
        store.set("match.title", match_info.match_title.clone());
        store.set("match.format", match_info.series_format.as_str());

        // Team 1
        store.set("match.team1.name", match_info.team1.name.clone());
        store.set("match.team1.tag", match_info.team1.tag.clone());
        store.set("match.team1.score", match_info.team1_score as i64);
        store.set("match.team1.color", match_info.team1.primary_color.to_hex());
        for (i, p) in match_info.team1.roster.iter().enumerate() {
            let prefix = format!("match.team1.player_{}", i + 1);
            store.set(format!("{prefix}.handle"), p.handle.clone());
            store.set(format!("{prefix}.role"), p.role.clone());
        }

        // Team 2
        store.set("match.team2.name", match_info.team2.name.clone());
        store.set("match.team2.tag", match_info.team2.tag.clone());
        store.set("match.team2.score", match_info.team2_score as i64);
        store.set("match.team2.color", match_info.team2.primary_color.to_hex());
        for (i, p) in match_info.team2.roster.iter().enumerate() {
            let prefix = format!("match.team2.player_{}", i + 1);
            store.set(format!("{prefix}.handle"), p.handle.clone());
            store.set(format!("{prefix}.role"), p.role.clone());
        }

        // Casters
        for (i, c) in match_info.casters.iter().enumerate() {
            let prefix = format!("casters.caster_{}", i + 1);
            store.set(format!("{prefix}.name"), c.name.clone());
            store.set(format!("{prefix}.handle"), c.handle.clone());
            store.set(format!("{prefix}.social"), c.social.clone());
        }

        // Map pool
        for (i, m) in match_info.map_pool.iter().enumerate() {
            store.set(format!("map_veto.map_{}", i + 1), m.clone());
        }
    }
}

impl Default for RosterManager {
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------------------
// Broadcast Template Generators (.arx Packages)
// ----------------------------------------------------------------------------

/// Helper to generate a text layer with given properties.
fn make_text_layer(name: &str, text: &str, font_size: f32, pos: Vec3, color: Color) -> Layer {
    let mut layer = Layer::new(
        LayerId::new(),
        name,
        LayerContent::Text(TextContent {
            text: Property::constant(text.to_string()),
            font_size: Property::constant(font_size),
            color: Property::constant(color),
            ..Default::default()
        }),
    );
    layer.transform.position = Property::constant(pos);
    layer
}

/// Helper to generate a rectangle box shape layer.
fn make_rect_layer(name: &str, size: Vec2, pos: Vec3, fill_color: Color) -> Layer {
    let mut layer = Layer::new(
        LayerId::new(),
        name,
        LayerContent::Shape(ShapeContent {
            geometry: ShapeGeometry::Rectangle {
                size: Property::constant(size),
            },
            fill: Some(Fill::Solid(Property::constant(fill_color))),
            stroke: None,
        }),
    );
    layer.transform.position = Property::constant(pos);
    layer
}

/// Creates the "Starting Soon" full-screen overlay template.
pub fn create_starting_soon_template() -> ArxPackage {
    let manifest = Manifest::new("Starting Soon", "Zenocast Esports Pack", 1920, 1080, 60.0);
    let mut comp = Composition::new(1920, 1080, 60.0, 30.0);

    // Dark flat background container
    comp.layers.push(make_rect_layer(
        "BackgroundCard",
        Vec2::new(1200.0, 500.0),
        Vec3::new(960.0, 540.0, 0.0),
        Color::rgb(0.08, 0.09, 0.1),
    ));

    // Accent line
    comp.layers.push(make_rect_layer(
        "AccentBar",
        Vec2::new(1200.0, 8.0),
        Vec3::new(960.0, 300.0, 0.0),
        Color::rgb(0.3, 0.55, 1.0),
    ));

    // Tournament title
    comp.layers.push(make_text_layer(
        "TournamentName",
        "{tournament.name}",
        28.0,
        Vec3::new(960.0, 360.0, 0.0),
        Color::rgb(0.55, 0.6, 0.65),
    ));

    // Big starting soon announcement
    comp.layers.push(make_text_layer(
        "StartingSoonTitle",
        "BROADCAST STARTING SOON",
        56.0,
        Vec3::new(960.0, 440.0, 0.0),
        Color::WHITE,
    ));

    // Matchup banner
    comp.layers.push(make_text_layer(
        "MatchBanner",
        "{match.team1.name}  vs  {match.team2.name}",
        36.0,
        Vec3::new(960.0, 540.0, 0.0),
        Color::rgb(0.9, 0.9, 0.95),
    ));

    // Format badge
    comp.layers.push(make_text_layer(
        "FormatBadge",
        "{match.format} • {match.title}",
        24.0,
        Vec3::new(960.0, 620.0, 0.0),
        Color::rgb(0.95, 0.65, 0.15),
    ));

    ArxPackage::new(manifest, comp)
}

/// Creates the "Be Right Back" screen template.
pub fn create_be_right_back_template() -> ArxPackage {
    let manifest = Manifest::new("Be Right Back", "Zenocast Esports Pack", 1920, 1080, 60.0);
    let mut comp = Composition::new(1920, 1080, 60.0, 15.0);

    comp.layers.push(make_rect_layer(
        "Card",
        Vec2::new(1000.0, 400.0),
        Vec3::new(960.0, 540.0, 0.0),
        Color::rgb(0.08, 0.09, 0.1),
    ));

    comp.layers.push(make_text_layer(
        "Title",
        "BE RIGHT BACK",
        52.0,
        Vec3::new(960.0, 460.0, 0.0),
        Color::WHITE,
    ));

    comp.layers.push(make_text_layer(
        "ScoreSummary",
        "{match.team1.tag}  {match.team1.score} - {match.team2.score}  {match.team2.tag}",
        44.0,
        Vec3::new(960.0, 550.0, 0.0),
        Color::rgb(0.3, 0.55, 1.0),
    ));

    ArxPackage::new(manifest, comp)
}

/// Creates the "Stream Ending" screen template.
pub fn create_stream_ending_template() -> ArxPackage {
    let manifest = Manifest::new("Stream Ending", "Zenocast Esports Pack", 1920, 1080, 60.0);
    let mut comp = Composition::new(1920, 1080, 60.0, 20.0);

    comp.layers.push(make_rect_layer(
        "Card",
        Vec2::new(1100.0, 450.0),
        Vec3::new(960.0, 540.0, 0.0),
        Color::rgb(0.08, 0.09, 0.1),
    ));

    comp.layers.push(make_text_layer(
        "Title",
        "THANKS FOR WATCHING",
        48.0,
        Vec3::new(960.0, 440.0, 0.0),
        Color::WHITE,
    ));

    comp.layers.push(make_text_layer(
        "TournamentCredits",
        "{tournament.name}  •  PRODUCED WITH ZENOCAST",
        24.0,
        Vec3::new(960.0, 540.0, 0.0),
        Color::rgb(0.6, 0.65, 0.7),
    ));

    ArxPackage::new(manifest, comp)
}

/// Creates the BO1 Scoreboard overlay template.
pub fn create_scoreboard_bo1_template() -> ArxPackage {
    let manifest = Manifest::new("Scoreboard BO1", "Zenocast Esports Pack", 1920, 1080, 60.0);
    let mut comp = Composition::new(1920, 1080, 60.0, 10.0);

    comp.layers.push(make_rect_layer(
        "Bar",
        Vec2::new(700.0, 64.0),
        Vec3::new(960.0, 60.0, 0.0),
        Color::rgb(0.08, 0.09, 0.1),
    ));

    comp.layers.push(make_text_layer(
        "Team1",
        "{match.team1.tag}  {match.team1.score}",
        28.0,
        Vec3::new(820.0, 60.0, 0.0),
        Color::WHITE,
    ));

    comp.layers.push(make_text_layer(
        "Divider",
        "VS (BO1)",
        20.0,
        Vec3::new(960.0, 60.0, 0.0),
        Color::rgb(0.95, 0.65, 0.15),
    ));

    comp.layers.push(make_text_layer(
        "Team2",
        "{match.team2.score}  {match.team2.tag}",
        28.0,
        Vec3::new(1100.0, 60.0, 0.0),
        Color::WHITE,
    ));

    ArxPackage::new(manifest, comp)
}

/// Creates the BO3 Scoreboard overlay template.
pub fn create_scoreboard_bo3_template() -> ArxPackage {
    let manifest = Manifest::new("Scoreboard BO3", "Zenocast Esports Pack", 1920, 1080, 60.0);
    let mut comp = Composition::new(1920, 1080, 60.0, 10.0);

    comp.layers.push(make_rect_layer(
        "Bar",
        Vec2::new(820.0, 72.0),
        Vec3::new(960.0, 60.0, 0.0),
        Color::rgb(0.08, 0.09, 0.1),
    ));

    comp.layers.push(make_text_layer(
        "Team1",
        "{match.team1.tag}  [{match.team1.score}]",
        30.0,
        Vec3::new(780.0, 60.0, 0.0),
        Color::WHITE,
    ));

    comp.layers.push(make_text_layer(
        "Divider",
        "BO3",
        22.0,
        Vec3::new(960.0, 60.0, 0.0),
        Color::rgb(0.3, 0.55, 1.0),
    ));

    comp.layers.push(make_text_layer(
        "Team2",
        "[{match.team2.score}]  {match.team2.tag}",
        30.0,
        Vec3::new(1140.0, 60.0, 0.0),
        Color::WHITE,
    ));

    ArxPackage::new(manifest, comp)
}

/// Creates the BO5 Scoreboard overlay template (Grand Finals).
pub fn create_scoreboard_bo5_template() -> ArxPackage {
    let manifest = Manifest::new("Scoreboard BO5", "Zenocast Esports Pack", 1920, 1080, 60.0);
    let mut comp = Composition::new(1920, 1080, 60.0, 10.0);

    comp.layers.push(make_rect_layer(
        "Bar",
        Vec2::new(900.0, 76.0),
        Vec3::new(960.0, 60.0, 0.0),
        Color::rgb(0.08, 0.09, 0.1),
    ));

    comp.layers.push(make_text_layer(
        "Team1",
        "{match.team1.tag}  [{match.team1.score}]",
        32.0,
        Vec3::new(760.0, 60.0, 0.0),
        Color::WHITE,
    ));

    comp.layers.push(make_text_layer(
        "Divider",
        "GRAND FINALS (BO5)",
        20.0,
        Vec3::new(960.0, 60.0, 0.0),
        Color::rgb(0.95, 0.65, 0.15),
    ));

    comp.layers.push(make_text_layer(
        "Team2",
        "[{match.team2.score}]  {match.team2.tag}",
        32.0,
        Vec3::new(1160.0, 60.0, 0.0),
        Color::WHITE,
    ));

    ArxPackage::new(manifest, comp)
}

/// Creates the Lower Third broadcast overlay template.
pub fn create_lower_third_template() -> ArxPackage {
    let manifest = Manifest::new("Lower Third", "Zenocast Esports Pack", 1920, 1080, 60.0);
    let mut comp = Composition::new(1920, 1080, 60.0, 8.0);

    // Accent slash
    comp.layers.push(make_rect_layer(
        "Accent",
        Vec2::new(8.0, 90.0),
        Vec3::new(160.0, 900.0, 0.0),
        Color::rgb(0.3, 0.55, 1.0),
    ));

    // Primary lower third banner
    comp.layers.push(make_rect_layer(
        "Bar",
        Vec2::new(560.0, 90.0),
        Vec3::new(448.0, 900.0, 0.0),
        Color::rgb(0.08, 0.09, 0.1),
    ));

    // Primary name
    comp.layers.push(make_text_layer(
        "Name",
        "{casters.caster_1.handle}",
        32.0,
        Vec3::new(400.0, 885.0, 0.0),
        Color::WHITE,
    ));

    // Subtitle / social handle
    comp.layers.push(make_text_layer(
        "Subtitle",
        "{casters.caster_1.name}  •  {casters.caster_1.social}",
        18.0,
        Vec3::new(400.0, 925.0, 0.0),
        Color::rgb(0.6, 0.65, 0.7),
    ));

    ArxPackage::new(manifest, comp)
}

/// Creates Caster Desk camera overlays for 2, 3, or 4 casters.
pub fn create_caster_desk_template(num_casters: usize) -> ArxPackage {
    let title = format!("Caster Desk ({} Casters)", num_casters);
    let manifest = Manifest::new(&title, "Zenocast Esports Pack", 1920, 1080, 60.0);
    let mut comp = Composition::new(1920, 1080, 60.0, 10.0);

    let total_w = 1760.0;
    let slot_w = (total_w / num_casters as f32) - 20.0;
    let slot_h = slot_w * 9.0 / 16.0;

    for i in 0..num_casters {
        let x = 100.0 + (slot_w / 2.0) + (i as f32 * (slot_w + 20.0));
        let y = 500.0;

        // Camera border frame
        comp.layers.push(make_rect_layer(
            &format!("CamFrame_{}", i + 1),
            Vec2::new(slot_w, slot_h),
            Vec3::new(x, y, 0.0),
            Color::rgb(0.12, 0.13, 0.15),
        ));

        // Nameplate beneath
        comp.layers.push(make_rect_layer(
            &format!("Nameplate_{}", i + 1),
            Vec2::new(slot_w, 40.0),
            Vec3::new(x, y + (slot_h / 2.0) + 24.0, 0.0),
            Color::rgb(0.08, 0.09, 0.1),
        ));

        comp.layers.push(make_text_layer(
            &format!("CasterText_{}", i + 1),
            &format!("{{casters.caster_{}.handle}}", i + 1),
            20.0,
            Vec3::new(x, y + (slot_h / 2.0) + 24.0, 0.0),
            Color::WHITE,
        ));
    }

    ArxPackage::new(manifest, comp)
}

/// Creates Player Cam Frames overlay (5v5 box frames).
pub fn create_player_cam_frames_template() -> ArxPackage {
    let manifest = Manifest::new(
        "Player Cam Frames",
        "Zenocast Esports Pack",
        1920,
        1080,
        60.0,
    );
    let mut comp = Composition::new(1920, 1080, 60.0, 10.0);

    // Left team: 5 player mini boxes stacked along the left margin
    let box_w = 160.0;
    let box_h = 100.0;

    for i in 0..5 {
        let y = 200.0 + (i as f32 * 140.0);
        comp.layers.push(make_rect_layer(
            &format!("T1_Cam_{}", i + 1),
            Vec2::new(box_w, box_h),
            Vec3::new(100.0, y, 0.0),
            Color::rgb(0.1, 0.11, 0.13),
        ));

        comp.layers.push(make_text_layer(
            &format!("T1_Name_{}", i + 1),
            &format!("{{match.team1.player_{}.handle}}", i + 1),
            16.0,
            Vec3::new(100.0, y + 60.0, 0.0),
            Color::WHITE,
        ));
    }

    // Right team: 5 player mini boxes stacked along the right margin
    for i in 0..5 {
        let y = 200.0 + (i as f32 * 140.0);
        comp.layers.push(make_rect_layer(
            &format!("T2_Cam_{}", i + 1),
            Vec2::new(box_w, box_h),
            Vec3::new(1820.0, y, 0.0),
            Color::rgb(0.1, 0.11, 0.13),
        ));

        comp.layers.push(make_text_layer(
            &format!("T2_Name_{}", i + 1),
            &format!("{{match.team2.player_{}.handle}}", i + 1),
            16.0,
            Vec3::new(1820.0, y + 60.0, 0.0),
            Color::WHITE,
        ));
    }

    ArxPackage::new(manifest, comp)
}

/// Creates the Map Veto broadcast board template.
pub fn create_map_veto_template() -> ArxPackage {
    let manifest = Manifest::new("Map Veto", "Zenocast Esports Pack", 1920, 1080, 60.0);
    let mut comp = Composition::new(1920, 1080, 60.0, 15.0);

    comp.layers.push(make_text_layer(
        "Title",
        "MAP VETO SELECTION",
        42.0,
        Vec3::new(960.0, 180.0, 0.0),
        Color::WHITE,
    ));

    for i in 0..5 {
        let x = 320.0 + (i as f32 * 320.0);
        comp.layers.push(make_rect_layer(
            &format!("MapCard_{}", i + 1),
            Vec2::new(280.0, 420.0),
            Vec3::new(x, 540.0, 0.0),
            Color::rgb(0.1, 0.11, 0.13),
        ));

        comp.layers.push(make_text_layer(
            &format!("MapName_{}", i + 1),
            &format!("{{map_veto.map_{}}}", i + 1),
            24.0,
            Vec3::new(x, 700.0, 0.0),
            Color::WHITE,
        ));
    }

    ArxPackage::new(manifest, comp)
}

/// Creates the Kill Feed overlay template.
pub fn create_kill_feed_template() -> ArxPackage {
    let manifest = Manifest::new("Kill Feed", "Zenocast Esports Pack", 1920, 1080, 60.0);
    let mut comp = Composition::new(1920, 1080, 60.0, 10.0);

    for i in 0..4 {
        let y = 100.0 + (i as f32 * 45.0);
        comp.layers.push(make_rect_layer(
            &format!("KillRow_{}", i + 1),
            Vec2::new(340.0, 36.0),
            Vec3::new(1720.0, y, 0.0),
            Color::rgb(0.08, 0.09, 0.1),
        ));

        comp.layers.push(make_text_layer(
            &format!("KillText_{}", i + 1),
            "SEN TenZ  [Vandal]  FNC Derke",
            16.0,
            Vec3::new(1720.0, y, 0.0),
            Color::rgb(0.9, 0.9, 0.95),
        ));
    }

    ArxPackage::new(manifest, comp)
}

/// Creates the Tournament Bracket playoff tree overlay template.
pub fn create_tournament_bracket_template() -> ArxPackage {
    let manifest = Manifest::new(
        "Tournament Bracket",
        "Zenocast Esports Pack",
        1920,
        1080,
        60.0,
    );
    let mut comp = Composition::new(1920, 1080, 60.0, 15.0);

    comp.layers.push(make_text_layer(
        "Title",
        "PLAYOFF BRACKET",
        48.0,
        Vec3::new(960.0, 140.0, 0.0),
        Color::WHITE,
    ));

    // Semifinals Left & Right
    comp.layers.push(make_rect_layer(
        "Semi1",
        Vec2::new(320.0, 120.0),
        Vec3::new(400.0, 400.0, 0.0),
        Color::rgb(0.1, 0.11, 0.13),
    ));

    comp.layers.push(make_rect_layer(
        "Semi2",
        Vec2::new(320.0, 120.0),
        Vec3::new(1520.0, 400.0, 0.0),
        Color::rgb(0.1, 0.11, 0.13),
    ));

    // Finals Center
    comp.layers.push(make_rect_layer(
        "GrandFinals",
        Vec2::new(400.0, 160.0),
        Vec3::new(960.0, 540.0, 0.0),
        Color::rgb(0.12, 0.14, 0.18),
    ));

    comp.layers.push(make_text_layer(
        "FinalsText",
        "{match.team1.tag}  vs  {match.team2.tag}",
        32.0,
        Vec3::new(960.0, 540.0, 0.0),
        Color::rgb(0.95, 0.65, 0.15),
    ));

    ArxPackage::new(manifest, comp)
}

/// Exports all 14 built-in esports broadcast templates into the designated directory.
pub fn export_esports_pack(output_dir: &Path) -> Result<Vec<PathBuf>, FormatError> {
    fs::create_dir_all(output_dir)?;

    let templates: Vec<(&str, ArxPackage)> = vec![
        ("starting_soon.arx", create_starting_soon_template()),
        ("be_right_back.arx", create_be_right_back_template()),
        ("stream_ending.arx", create_stream_ending_template()),
        ("scoreboard_bo1.arx", create_scoreboard_bo1_template()),
        ("scoreboard_bo3.arx", create_scoreboard_bo3_template()),
        ("scoreboard_bo5.arx", create_scoreboard_bo5_template()),
        ("lower_third.arx", create_lower_third_template()),
        ("caster_desk_2.arx", create_caster_desk_template(2)),
        ("caster_desk_3.arx", create_caster_desk_template(3)),
        ("caster_desk_4.arx", create_caster_desk_template(4)),
        ("player_cam_frames.arx", create_player_cam_frames_template()),
        ("map_veto.arx", create_map_veto_template()),
        ("kill_feed.arx", create_kill_feed_template()),
        (
            "tournament_bracket.arx",
            create_tournament_bracket_template(),
        ),
    ];

    let mut written_paths = Vec::with_capacity(templates.len());

    for (filename, package) in templates {
        let dest = output_dir.join(filename);
        let bytes = package.to_bytes()?;
        fs::write(&dest, bytes)?;
        written_paths.push(dest);
    }

    Ok(written_paths)
}
