//! The one document-grain refusal of `[system_prompt]`: the table is tuning
//! for an opt-in, and the opt-in is a file (PROP-076 `#SYS-OPT-IN`).
//!
//! Presence of `vibevm/vibespecs/boot/SYSTEM-TEMPLATE.md` is the entire
//! opt-in and nothing else is. A `[system_prompt]` table with no template
//! beside it would configure a feature that is switched off — silently, and
//! for as long as nobody noticed — so it refuses by name instead.
//!
//! This refusal needs the node's directory, so it is a separate method rather
//! than part of the path-free [`Manifest::validate`]: a manifest parsed from
//! a string has no directory to look in, and inventing one would be worse
//! than not checking.

use std::path::Path;

use specmark::spec;

use crate::error::Result;
use crate::manifest::{FieldError, FieldPath};

use super::Manifest;

impl Manifest {
    /// Refuse a `[system_prompt]` table that has no authored
    /// `SYSTEM-TEMPLATE.md` beside it, naming the TOML path and the exact
    /// file that would switch the feature on.
    ///
    /// `manifest_path` is this document's own path; its parent is the node
    /// directory. Called by [`Manifest::read`], so every manifest that comes
    /// off disk is checked and one parsed from a string is not.
    #[spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-076#SYS-OPT-IN")]
    pub fn validate_system_prompt_beside(&self, manifest_path: &Path) -> Result<()> {
        if self.system_prompt.is_none() {
            return Ok(());
        }
        let node_dir = manifest_path.parent().unwrap_or(Path::new(""));
        let relative = crate::layout::current_boot_system_template();
        if node_dir.join(&relative).is_file() {
            return Ok(());
        }
        Err(FieldError::new(
            FieldPath::root().table("system_prompt"),
            format!(
                "[system_prompt] configures the generated system-prompt prefix, but this node \
                 has not opted in: `{}` does not exist. The template's presence is the entire \
                 opt-in — an empty one means \"opt in with no frame of my own\" (fix: create \
                 `{}`, or delete [system_prompt])",
                relative.display(),
                relative.display()
            ),
        )
        .into())
    }
}
