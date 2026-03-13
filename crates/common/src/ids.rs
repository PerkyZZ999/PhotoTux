//! Stable identifier types used across PhotoTux.

/// Stable document identifier.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DocumentId(u128);

impl DocumentId {
    /// Create a document identifier from a raw value.
    #[must_use]
    pub const fn new(raw: u128) -> Self {
        Self(raw)
    }

    /// Return the raw identifier value.
    #[must_use]
    pub const fn get(self) -> u128 {
        self.0
    }
}

/// Stable layer identifier.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LayerId(u128);

impl LayerId {
    /// Create a layer identifier from a raw value.
    #[must_use]
    pub const fn new(raw: u128) -> Self {
        Self(raw)
    }

    /// Return the raw identifier value.
    #[must_use]
    pub const fn get(self) -> u128 {
        self.0
    }
}
