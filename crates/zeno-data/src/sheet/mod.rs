//! Spreadsheet integration modules: models, coordinate parsing, OpenXML/CSV file readers,
//! and live DataStore synchronization engine.

pub mod model;
pub mod reader;
pub mod sync;

pub use model::{
    CellBinding, CellTarget, LinkDirection, SheetCoordinate, SheetLink, SheetLinkStatus,
    SheetSource, VariableKind,
};
pub use reader::{SpreadsheetReader, SpreadsheetTable};
pub use sync::SheetSyncEngine;
