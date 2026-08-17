//! Behaviour of a record's projections — emptiness and the zero
//! value, the two things the generated form does not derive. Emptiness
//! is the writer's normalisation guard: an empty projection is
//! absence on the wire (`"provides": {}` is never written), so every
//! writer-side skip decision reduces to these predicates.
//!
//! The `Default` impls here are hand-written on purpose and not
//! derives: a derive lives at the definition site, the definition site
//! is generated, and the next `cargo xtask codegen` would wipe it. The
//! zero value belongs with the rest of the type's behaviour, so it
//! lives here — which is also why `derivable_impls` is allowed per
//! impl rather than silenced crate-wide.

use crate::generated::shared::{
    CompatibilityEntry, ConflictsEntry, FeaturesEntry, I18nEntry, ObsoletesEntry, ProvidesEntry,
    RequiresEntry,
};

impl CompatibilityEntry {
    pub fn is_empty(&self) -> bool {
        self.min_vibe_version.is_none() && self.requires_kinds.is_empty()
    }
}

#[allow(clippy::derivable_impls)]
impl Default for CompatibilityEntry {
    fn default() -> Self {
        CompatibilityEntry {
            min_vibe_version: None,
            requires_kinds: Vec::new(),
        }
    }
}

impl ProvidesEntry {
    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
    }
}

#[allow(clippy::derivable_impls)]
impl Default for ProvidesEntry {
    fn default() -> Self {
        ProvidesEntry {
            capabilities: Vec::new(),
        }
    }
}

impl RequiresEntry {
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty() && self.capabilities.is_empty()
    }
}

#[allow(clippy::derivable_impls)]
impl Default for RequiresEntry {
    fn default() -> Self {
        RequiresEntry {
            packages: Vec::new(),
            capabilities: Vec::new(),
        }
    }
}

impl ObsoletesEntry {
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }
}

#[allow(clippy::derivable_impls)]
impl Default for ObsoletesEntry {
    fn default() -> Self {
        ObsoletesEntry {
            packages: Vec::new(),
        }
    }
}

impl ConflictsEntry {
    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }
}

#[allow(clippy::derivable_impls)]
impl Default for ConflictsEntry {
    fn default() -> Self {
        ConflictsEntry {
            packages: Vec::new(),
        }
    }
}

impl FeaturesEntry {
    pub fn is_empty(&self) -> bool {
        self.features.is_empty() && self.exclusive.is_empty()
    }
}

#[allow(clippy::derivable_impls)]
impl Default for FeaturesEntry {
    fn default() -> Self {
        FeaturesEntry {
            features: std::collections::BTreeMap::new(),
            exclusive: std::collections::BTreeMap::new(),
        }
    }
}

impl I18nEntry {
    pub fn is_empty(&self) -> bool {
        self.available.is_empty() && self.default.is_none()
    }
}

#[allow(clippy::derivable_impls)]
impl Default for I18nEntry {
    fn default() -> Self {
        I18nEntry {
            available: Vec::new(),
            default: None,
        }
    }
}
