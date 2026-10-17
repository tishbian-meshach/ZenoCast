//! # zeno-data
//!
//! Live observable data store, property bindings, variable schema,
//! string template parsing, format specifiers, and animation state machine transitions.

pub mod binding;
pub mod error;
pub mod sheet;
pub mod state_machine;
pub mod store;
pub mod value;

// Re-exports
pub use binding::{
    apply_bindings_to_composition, resolve_template, resolve_template_from_snapshot,
};
pub use error::{DataError, SheetError};
pub use sheet::{
    CellBinding, CellTarget, LinkDirection, SheetCoordinate, SheetLink, SheetLinkStatus,
    SheetSource, SheetSyncEngine, SpreadsheetReader, SpreadsheetTable, VariableKind,
};
pub use state_machine::{DataTriggerRule, StateAction, StateMachine, Transition};
pub use store::{DataStore, ListenerCallback};
pub use value::DataValue;
