//! Logical resolution of `[boot_snippet].source` against a materialised
//! slot — literal first, then the paired spec extension (PROP-045
//! ##BOOT-LANE-SCOPE / S4). Split from `bootgen.rs` at the S4 landing
//! (the file crossed the 600-line budget).

use std::path::{Path, PathBuf};

/// Resolve a `[boot_snippet].source` LOGICALLY against the materialised
/// slot (PROP-045 ##BOOT-LANE-SCOPE): the literal path when it exists, and
/// — when a transforming materialisation re-serialised the snippet to the
/// other extension — the same stem in the other form. The manifest string
/// names the AUTHORED form; the slot holds the MATERIALISED one, and the
/// boot paths (INDEX entries above all) must carry what is actually on
/// disk. Neither form present resolves to the literal unchanged, so a
/// genuinely missing snippet still fails loudly at read time, exactly as
/// before.
pub(super) fn resolve_snippet_source(
    workspace_root: &Path,
    slot_rel: &str,
    source: &Path,
) -> String {
    let source_rel = source.to_string_lossy().replace('\\', "/");
    let literal = format!("{slot_rel}/{source_rel}");
    if workspace_root.join(&literal).is_file() {
        return literal;
    }
    let swapped = swap_spec_extension(Path::new(&source_rel));
    if let Some(swapped) = swapped {
        let alt = format!(
            "{slot_rel}/{}",
            swapped.to_string_lossy().replace('\\', "/")
        );
        if workspace_root.join(&alt).is_file() {
            return alt;
        }
    }
    literal
}

/// The same path with its spec extension flipped `.md` ↔ `.xml` — `None`
/// when the path names neither form (nothing to flip to).
pub(super) fn swap_spec_extension(path: &Path) -> Option<PathBuf> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("md") => {
            let mut p = path.to_path_buf();
            p.set_extension("xml");
            Some(p)
        }
        Some("xml") => {
            let mut p = path.to_path_buf();
            p.set_extension("md");
            Some(p)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[test]
    fn snippet_source_resolves_logically_against_the_slot() {
        let ws = tempfile::tempdir().expect("tempdir");
        let slot = "vibedeps/flow-lib/1.0.0";
        let boot_dir = ws.path().join(slot).join("spec/boot");
        fs::create_dir_all(&boot_dir).expect("mkdir");

        // Neither form: the literal stands (and a later read names it).
        assert_eq!(
            resolve_snippet_source(ws.path(), slot, Path::new("spec/boot/snippet.md")),
            format!("{slot}/spec/boot/snippet.md")
        );

        // Materialised as XML (S3 flipped the extension): the .md literal
        // resolves to the .xml that exists.
        fs::write(boot_dir.join("snippet.xml"), "<spec/>").expect("write");
        assert_eq!(
            resolve_snippet_source(ws.path(), slot, Path::new("spec/boot/snippet.md")),
            format!("{slot}/spec/boot/snippet.xml")
        );
        // And the symmetric case: an .xml literal over a slot materialised
        // as MD (a second stem — the first now holds its literal .xml).
        fs::write(boot_dir.join("other.md"), "# o\n").expect("write");
        assert_eq!(
            resolve_snippet_source(ws.path(), slot, Path::new("spec/boot/other.xml")),
            format!("{slot}/spec/boot/other.md")
        );

        // The literal wins once it exists again (mixed slot edge).
        fs::write(boot_dir.join("snippet.md"), "# s\n").expect("write");
        assert_eq!(
            resolve_snippet_source(ws.path(), slot, Path::new("spec/boot/snippet.md")),
            format!("{slot}/spec/boot/snippet.md")
        );
    }
}
