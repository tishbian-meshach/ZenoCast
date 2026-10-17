//! Strongly typed identifiers used across the Zenocast engine and live studio.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

macro_rules! define_id {
    ($name:ident, $doc:expr) => {
        #[doc = $doc]
        #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            /// Generate a new random identifier.
            #[inline]
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Create an identifier from an existing UUID.
            #[inline]
            pub const fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// Returns the underlying UUID.
            #[inline]
            pub const fn as_uuid(&self) -> Uuid {
                self.0
            }

            /// Nil / empty identifier.
            #[inline]
            pub const fn nil() -> Self {
                Self(Uuid::nil())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(s).map(Self)
            }
        }

        impl From<Uuid> for $name {
            fn from(uuid: Uuid) -> Self {
                Self(uuid)
            }
        }
    };
}

define_id!(
    LayerId,
    "Unique identifier for a scene graph or composition layer."
);
define_id!(
    AssetId,
    "Unique identifier for an embedded or referenced media asset."
);
define_id!(
    SceneId,
    "Unique identifier for a production broadcast scene."
);
define_id!(
    TransitionId,
    "Unique identifier for a transition rule or transition preset."
);
define_id!(
    LinkId,
    "Unique identifier for a live spreadsheet binding link."
);
define_id!(
    CompositionId,
    "Unique identifier for a standalone composition or template."
);
