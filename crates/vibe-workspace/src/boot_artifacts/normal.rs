//! `normal + static` compilation (PROP-035 §8) — the branch of the static
//! renderer ([`super::render_static`]) that compiles a `normal`-format
//! package's contribution to its `#use` / `#source`-resolved, tree-shaken
//! closure, rather than concatenating the file verbatim (the `simple` path).
//!
//! The hard algorithmic work is [`vibe_spec::compile_static`]; this cell only
//! derives the closure's seed from a [`BootEntry`] and adapts the compiler's
//! error into a REQ-citing [`WorkspaceError`].

specmark::scope!("spec://vibevm/modules/vibe-workspace/PROP-035#pipeline");

use std::path::Path;

use specmark::spec;
use vibe_spec::{FileResolver, FsSectionSource, SpecAddress, compile_static};

use super::HOST_NAMESPACE;
use crate::WorkspaceError;
use crate::boot::BootEntry;

/// Compile a `normal` package's static contribution (PROP-035 §8): the
/// `#use` / `#source`-resolved, tree-shaken, topologically-ordered closure
/// reachable from the entry's boot-snippet contract, rather than the file
/// concatenated verbatim. Resolution runs against the materialised
/// `vibedeps/` tree (the same [`FileResolver`] the `#embed` pass uses), so the
/// closure may span `source/` and other packages the contract `#use`s.
///
/// Errors are surfaced as [`WorkspaceError::InlineCompile`] naming the package
/// and the governing requirement (PROP-035 §8) — a structured, REQ-citing
/// diagnostic the installer prints rather than a bare compiler string.
#[spec(
    implements = "spec://vibevm/modules/vibe-workspace/PROP-035#pipeline",
    r = 1
)]
pub(super) fn compile_normal_entry(
    entry: &BootEntry,
    workspace_root: &Path,
) -> Result<String, WorkspaceError> {
    let seed =
        normal_seed(&entry.origin, &entry.path).ok_or_else(|| WorkspaceError::InlineCompile {
            reason: format!(
                "cannot derive a spec:// seed for the normal package `{}` at `{}` \
                 (PROP-035 §8): expected a `<group>/<name>` origin and a path under a \
                 package's `vibedeps/…/spec/` root",
                entry.origin, entry.path
            ),
        })?;
    let source = FsSectionSource::new(FileResolver::new(workspace_root, HOST_NAMESPACE));
    compile_static(&seed, &source).map_err(|e| WorkspaceError::InlineCompile {
        reason: format!(
            "compiling the normal package `{}` closure (PROP-035 §8): {e}",
            entry.origin
        ),
    })
}

/// Derive the `spec://` seed for a `normal` static entry — the whole-document
/// address of its boot-snippet contract, from which [`compile_static`] walks
/// the `#use` / `#source` closure (PROP-035 §6/§8).
///
/// `origin` is the entry's `<group>/<name>` provenance (a hoisted entry may
/// append a ` [shared by …]` suffix, dropped here); `path` is the
/// workspace-relative path of the contract inside the package's `vibedeps/`
/// slot (e.g. `vibedeps/flow-greeter/1.0.0/spec/contract/greeting.md`). The
/// doc-path is the segment after the slot's `spec/` root minus the `.md`
/// extension (`contract/greeting`); the seed carries no anchor, so it names the
/// whole document (`DocTree` resolves an empty anchor to the root). Returns
/// `None` when the origin or path is not the expected package shape.
fn normal_seed(origin: &str, path: &str) -> Option<SpecAddress> {
    let coord = origin.split_whitespace().next()?;
    let (group, name) = coord.split_once('/')?;
    let (_, doc_rest) = path.split_once("/spec/")?;
    let doc_path = doc_rest.strip_suffix(".md").unwrap_or(doc_rest);
    SpecAddress::parse(&format!("spec://{group}/{name}/{doc_path}")).ok()
}

#[cfg(test)]
mod tests {
    use specmark::verifies;

    use super::*;

    #[test]
    #[verifies("spec://vibevm/modules/vibe-workspace/PROP-035#addressing")]
    fn normal_seed_derives_the_whole_doc_contract_address() {
        // The seed is the contract's whole-document address — no anchor, so the
        // compiler walks from the root (PROP-035 §6): `<group>/<name>` from the
        // origin, doc-path from the slot's `spec/` root minus `.md`.
        let s = normal_seed(
            "com.example.hello/greeter",
            "vibedeps/flow-greeter/1.0.0/spec/contract/greeting.md",
        )
        .unwrap();
        assert_eq!(
            s.without_pin(),
            "spec://com.example.hello/greeter/contract/greeting"
        );

        // A hoisted entry's ` [shared by …]` origin suffix is dropped.
        let h = normal_seed(
            "com.example.hello/greeter [shared by a/b]",
            "vibedeps/flow-greeter/1.0.0/spec/contract/greeting.md",
        )
        .unwrap();
        assert_eq!(
            h.without_pin(),
            "spec://com.example.hello/greeter/contract/greeting"
        );

        // A path with no `spec/` root, or a nameless origin, is not derivable.
        assert!(normal_seed("com.example.hello/greeter", "some/other/path.md").is_none());
        assert!(normal_seed("nogroup", "vibedeps/x/1.0.0/spec/a.md").is_none());
    }
}
