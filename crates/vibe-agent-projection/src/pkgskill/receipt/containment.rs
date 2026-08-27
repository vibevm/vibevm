//! Package-skill names for the shared portable-path and no-follow laws.
//!
//! `vibe-core` owns literal component legality. `vibe-safefs` owns physical
//! identity, overlap, lexical containment and no-follow inspection. Receipt
//! code keeps these aliases only so its domain vocabulary remains readable;
//! it carries no second Unicode, Win32 or filesystem implementation.

pub(crate) use vibe_safefs::{
    ensure_lexically_contained, ensure_no_follow_walk, judge_selection, paths_overlap,
};

/// Receipt maps need an ordered physical-identity key. Exact spelling remains
/// the ownership key; this alias is used only for collision/rename refusal.
pub(crate) type FoldKey = String;

#[must_use]
pub(crate) fn fold_key(value: impl AsRef<str>) -> FoldKey {
    vibe_safefs::path_identity_key(value.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_aliases_delegate_to_the_shared_identity() {
        assert_eq!(fold_key("Maße.md"), fold_key("MASSE.md"));
        assert!(judge_selection(["SKILL.md", "skill.md"]).is_err());
        assert!(paths_overlap(
            std::path::Path::new("/project/.claude/skills/demo"),
            std::path::Path::new("/PROJECT/.CLAUDE/skills/demo/nested"),
        ));
    }
}
