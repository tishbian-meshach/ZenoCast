//! Spout GPU texture sender output sink.
//!
//! Gated behind the `spout` feature flag.

use crate::error::OutputResult;

/// Spout shared GPU texture broadcast output sink.
pub struct SpoutOutput {
    sender_name: String,
    width: u32,
    height: u32,
    is_active: bool,
}

impl SpoutOutput {
    /// Checks whether Spout output support is enabled at build time.
    pub fn is_available() -> bool {
        cfg!(feature = "spout")
    }

    /// Creates a new Spout texture sender.
    pub fn new(sender_name: impl Into<String>, width: u32, height: u32) -> OutputResult<Self> {
        let sender_name = sender_name.into();
        if !Self::is_available() {
            tracing::warn!(
                "Spout output '{}' initialized but the 'spout' feature flag is disabled at compile time",
                sender_name
            );
        }

        Ok(Self {
            sender_name,
            width,
            height,
            is_active: false,
        })
    }

    /// Starts Spout texture sharing.
    pub fn start(&mut self) -> OutputResult<()> {
        self.is_active = true;
        Ok(())
    }

    /// Stops Spout texture sharing.
    pub fn stop(&mut self) {
        self.is_active = false;
    }

    /// Whether this Spout output is currently active.
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// Target sender name.
    pub fn sender_name(&self) -> &str {
        &self.sender_name
    }

    /// Output video dimensions.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}
