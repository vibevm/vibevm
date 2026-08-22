//! Format-selected names and exactly-one resolution for the generated STATIC lane.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-045#materialisation");

use std::path::{Path, PathBuf};

use vibe_core::manifest::SpecFormat;

use crate::WorkspaceError;

/// Markdown/mixed target spelling of the generated static lane.
pub const STATIC_FILE: &str = "STATIC.md";
/// XML target spelling of the generated static lane.
pub const STATIC_XML_FILE: &str = "STATIC.xml";

/// The one generated static-lane filename selected by a project's effective
/// spec format. Mixed is the compatibility-preserving Markdown target.
pub const fn static_file(format: SpecFormat) -> &'static str {
    match format {
        SpecFormat::Xml => STATIC_XML_FILE,
        SpecFormat::Mixed | SpecFormat::Markdown => STATIC_FILE,
    }
}

/// The selected static-lane path relative to a project root.
pub const fn static_path(format: SpecFormat) -> &'static str {
    match format {
        SpecFormat::Xml => "spec/boot/STATIC.xml",
        SpecFormat::Mixed | SpecFormat::Markdown => "spec/boot/STATIC.md",
    }
}

/// Resolve the generated static lane without guessing between its two owned
/// names. Neither is valid, either one alone is valid, and both is corruption.
pub fn resolve_static_path(root: &Path) -> Result<Option<PathBuf>, WorkspaceError> {
    let boot_dir = root.join("spec").join("boot");
    let markdown = boot_dir.join(STATIC_FILE);
    let xml = boot_dir.join(STATIC_XML_FILE);
    match (markdown.is_file(), xml.is_file()) {
        (true, false) => Ok(Some(markdown)),
        (false, true) => Ok(Some(xml)),
        (false, false) => Ok(None),
        (true, true) => Err(WorkspaceError::Io {
            path: boot_dir,
            reason:
                "both STATIC.md and STATIC.xml exist — the generator owns one; delete the stray"
                    .to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn exactly_one_resolver_accepts_neither_or_either_and_rejects_both() {
        let root = tempdir().unwrap();
        let boot = root.path().join("spec/boot");
        fs::create_dir_all(&boot).unwrap();
        assert_eq!(resolve_static_path(root.path()).unwrap(), None);

        let markdown = boot.join(static_file(SpecFormat::Markdown));
        fs::write(&markdown, "md").unwrap();
        assert_eq!(resolve_static_path(root.path()).unwrap(), Some(markdown));

        fs::remove_file(boot.join(static_file(SpecFormat::Markdown))).unwrap();
        let xml = boot.join(static_file(SpecFormat::Xml));
        fs::write(&xml, "xml").unwrap();
        assert_eq!(resolve_static_path(root.path()).unwrap(), Some(xml));

        fs::write(boot.join(static_file(SpecFormat::Markdown)), "md").unwrap();
        let error = resolve_static_path(root.path()).unwrap_err().to_string();
        assert!(error.contains(
            "both STATIC.md and STATIC.xml exist — the generator owns one; delete the stray"
        ));
    }
}
