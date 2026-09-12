//! `doc-shell.lock` — the shell pinned beside `vibe` (PROP-057
//! `##SHELL-PIN`).
//!
//! The pin says which shell this build of `vibe` belongs with: the web
//! package's coordinate, its version, and the digest of the shell that
//! was built from it. It is a committed, human-readable record, and it is
//! compiled INTO the binary, so a pin cannot be edited under a `vibe` that
//! has already shipped — the strongest reading of «beside `vibe`» the
//! phase-0 finding offered.
//!
//! It is the fixed point of three numbers a reader can be asked about:
//! the pin, the digest the build recorded in the shell's own index, and
//! the digest of the bytes the binary is carrying right now. Agreement is
//! the whole claim; `vibe doc shell status` is where the three meet.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-057#SHELL-PIN");

use crate::error::{ShellError, ShellResult};

/// The pin as committed, compiled in.
pub const PIN_TOML: &str = include_str!("../doc-shell.lock");

/// The pin's own shape version.
pub const PIN_SCHEMA: u32 = 1;

/// Which shell this `vibe` belongs with.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pin {
    /// The pin's shape version.
    pub schema: u32,
    /// The web package, `<group>/<name>`.
    pub package: String,
    /// Its version.
    pub version: String,
    /// The digest of the shell built from it, lowercase hex. Empty until
    /// a shell has ever been built in this checkout — which is the state
    /// of a fresh clone, and an honest one to report.
    pub sha256: String,
}

impl Pin {
    /// The pin this binary was built with.
    ///
    /// ```
    /// let pin = vibe_doc_shell::pin::Pin::compiled_in().unwrap();
    /// assert_eq!(pin.package, "org.vibevm.doc/web");
    /// ```
    pub fn compiled_in() -> ShellResult<Pin> {
        Pin::parse(PIN_TOML)
    }

    /// Read a pin from its TOML text.
    pub fn parse(text: &str) -> ShellResult<Pin> {
        let pin: Pin = toml::from_str(text).map_err(|error| ShellError::Pin {
            message: format!("does not parse: {error}"),
        })?;
        if pin.schema != PIN_SCHEMA {
            return Err(ShellError::Pin {
                message: format!(
                    "is written to pin schema {} and this reader knows {PIN_SCHEMA}",
                    pin.schema
                ),
            });
        }
        Ok(pin)
    }

    /// Is this pin naming a shell at all, or is it the state of a
    /// checkout where none was ever built?
    pub fn is_set(&self) -> bool {
        !self.sha256.is_empty()
    }
}
