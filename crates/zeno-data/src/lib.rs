//! # zeno-data
//!
//! Live observable data store, property bindings, variable schema,
//! string template parsing, format specifiers, and animation state machine transitions.

pub mod binding;
pub mod error;
pub mod esports;
pub mod sheet;
pub mod state_machine;
pub mod store;
pub mod value;

// Re-exports
pub use binding::{
    apply_bindings_to_composition, resolve_template, resolve_template_from_snapshot,
};
pub use error::{DataError, SheetError};
pub use esports::{
    create_be_right_back_template, create_caster_desk_template, create_kill_feed_template,
    create_lower_third_template, create_map_veto_template, create_player_cam_frames_template,
    create_scoreboard_bo1_template, create_scoreboard_bo3_template, create_scoreboard_bo5_template,
    create_starting_soon_template, create_stream_ending_template,
    create_tournament_bracket_template, export_esports_pack, Caster, Player, RosterManager,
    SeriesFormat, Team, TournamentMatch,
};
pub use sheet::{
    CellBinding, CellTarget, LinkDirection, SheetCoordinate, SheetLink, SheetLinkStatus,
    SheetSource, SheetSyncEngine, SpreadsheetReader, SpreadsheetTable, VariableKind,
};
pub use state_machine::{DataTriggerRule, StateAction, StateMachine, Transition};
pub use store::{DataStore, ListenerCallback};
pub use value::DataValue;
