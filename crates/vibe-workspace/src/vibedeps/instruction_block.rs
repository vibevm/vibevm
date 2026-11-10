//! Neutralising the managed boot block in a materialised dependency copy.
//!
//! Gemini CLI and Qwen Code discover context files in sub-directories as
//! well as parent ones and concatenate what they find; Claude Code resolves
//! imports downward too. A dependency copy under the live dependency root is
//! therefore reachable from a session rooted above it, and the copy carries
//! the *package's* own managed block — a second project's boot rules inside
//! this project's context (PROP-076 `#SYS-NO-RECURSION`).
//!
//! The router already scopes itself in prose, which is a sentence a model
//! may skip. This is the mechanical half: `vibe` replaces the block's body
//! with a notice in every copy it materialises, so the bytes a harness
//! concatenates do not describe a boot at all.
//!
//! Two limits, both deliberate. **Only the block changes.** Every byte of
//! the copy outside the markers is the package's own and is copied verbatim,
//! which is the same ownership rule PROP-012 puts on a co-tenant file.
//! **A copy is never repaired.** A file that is not UTF-8, or whose markers
//! are not exactly one ordered pair, is copied byte for byte and left to the
//! package that published it: rewriting a malformed marker pair in someone
//! else's file is the repair `locate_block` refuses on our own.

specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-076#SYS-NO-RECURSION");

use std::path::Path;

use crate::boot_artifacts::{
    BlockLocation, REDIRECT_FILES, locate_block, render_neutralised_block,
};

/// Is `relative` an agent instruction file, at any depth inside the copy?
///
/// Any depth, because a harness's downward discovery does not stop at a
/// package root: a nested project inside a published package is exactly the
/// cross-project pollution this closes.
pub(super) fn is_instruction_file(relative: &Path) -> bool {
    relative
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| REDIRECT_FILES.contains(&name))
}

/// The bytes a materialised copy of `bytes` carries, or `None` when there is
/// nothing to neutralise and the file is copied verbatim.
pub(super) fn neutralise(bytes: &[u8]) -> Option<Vec<u8>> {
    let content = std::str::from_utf8(bytes).ok()?;
    let BlockLocation::Present { start, end } = locate_block(content) else {
        // Absent: nothing to neutralise. Malformed: not ours to repair.
        return None;
    };
    let block = render_neutralised_block();
    let mut out = String::with_capacity(content.len() + block.len());
    out.push_str(&content[..start]);
    out.push_str(&block);
    out.push_str(&content[end..]);
    if out == content {
        return None;
    }
    Some(out.into_bytes())
}

/// The neutralised bytes for one file of a copy, or `None` when the file is
/// copied verbatim — the one call the three materialisation walks share.
pub(super) fn for_file(relative: &Path, bytes: &[u8]) -> Option<Vec<u8>> {
    is_instruction_file(relative)
        .then(|| neutralise(bytes))
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boot_artifacts::NEUTRALISED_BODY;

    #[test]
    fn only_an_instruction_file_is_touched_and_only_inside_its_block() {
        let before = "# Package rules\n\nkeep me\n\n<vibevm>\nROUTER\n</vibevm>\n\ntail\n";
        let out = for_file(Path::new("CLAUDE.md"), before.as_bytes())
            .expect("an instruction file with a block is neutralised");
        let out = String::from_utf8(out).unwrap();
        assert!(out.starts_with("# Package rules\n\nkeep me\n\n"));
        assert!(out.ends_with("\n\ntail\n"));
        assert!(out.contains(NEUTRALISED_BODY));
        assert!(!out.contains("ROUTER"));
        // A file of another name keeps its bytes whatever it carries.
        assert!(for_file(Path::new("README.md"), before.as_bytes()).is_none());
    }

    #[test]
    fn an_instruction_file_at_any_depth_is_neutralised() {
        let before = "<vibevm>\nROUTER\n</vibevm>\n";
        for relative in ["AGENTS.md", "nested/GEMINI.md", "a/b/c/CLAUDE.md"] {
            assert!(
                for_file(Path::new(relative), before.as_bytes()).is_some(),
                "{relative} is reachable by downward discovery"
            );
        }
    }

    #[test]
    fn a_copy_with_no_block_a_malformed_pair_or_non_utf8_is_left_alone() {
        assert!(for_file(Path::new("AGENTS.md"), b"# Mine\n").is_none());
        assert!(
            for_file(
                Path::new("AGENTS.md"),
                b"<vibevm>\na\n</vibevm>\n<vibevm>\nb\n</vibevm>\n"
            )
            .is_none(),
            "a malformed pair in someone else's file is not ours to repair"
        );
        assert!(for_file(Path::new("AGENTS.md"), &[0xff, 0xfe]).is_none());
    }

    #[test]
    fn neutralising_twice_changes_nothing_the_second_time() {
        let once = for_file(Path::new("AGENTS.md"), b"<vibevm>\nROUTER\n</vibevm>\n")
            .expect("the first pass replaces the body");
        assert!(
            for_file(Path::new("AGENTS.md"), &once).is_none(),
            "an already-neutralised copy is byte-stable"
        );
    }
}
