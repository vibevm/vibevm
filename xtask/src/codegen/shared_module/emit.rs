//! The emission half of the shared module: run the pinned generator
//! over the synthetic all-fragments document, take the output through
//! the SAME post-processing passes a schema module takes, then strip
//! the one emission a definitions-only document adds — the parasitic
//! root alias.
//!
//! The module goes through the SAME entry a schema module goes through
//! — `postproc::rewrite_generated` — and not through a second copy of
//! the pass list. The pass ORDER is a normative value (a pass keyed to
//! the generator's emission shape must run while the file is still that
//! emission), and a normative value duplicated into a second file
//! diverges from its original sooner or later
//! (`spec://org.vibevm.world/addressable-specs/…#single-source`); here
//! it would diverge in the one way this step cannot survive, since the
//! whole replacement stitches on the two emissions being byte-identical.
//!
//! Exactly one slot of the nine asks a question a synthetic document
//! cannot answer: strictness rules through `formats/REGISTRY.toml` by
//! the schema's own path, and no record claims this document. That is
//! why the entry takes `StrictnessSource`: the shared module's verdict
//! is decided one storey up, before emission, from every registered
//! consumer in the resolved schema closure. A unanimous `none` fragment
//! is stamped strict, a fragment with no `none` consumer remains byte-
//! identical to the former permissive emission, and a mixture has
//! already refused.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use super::super::postproc::{StrictnessSource, rewrite_generated};
use super::{SharedStrictness, prune_orphan_imports};

/// The shared module's directory name under the generated tree — and,
/// through the synthetic document's stem, the name of the parasitic
/// root this pass strips. Checked against every schema stem the layout
/// rules admit: no schema is named `shared`.
pub(crate) const SHARED_MODULE: &str = "shared";

/// The one emission a definitions-only document adds: an empty root
/// form renders as `Option<Value>`, and the generator mints the root's
/// name from the stem (`shared.jtd.json` → `Shared`). Measured against
/// the pinned build. It carries no doc comment — the synthetic
/// document has no root metadata — and the strip refuses a file in
/// which the line is absent, because its absence means the emission
/// shape moved.
const PARASITIC_ROOT: &str = "pub type Shared = Option<Value>;";

/// Emit the shared module: spawn the generator over the synthetic
/// document into `<out_dir>/shared/`, pass the output through the
/// schema modules' pipeline, strip the parasitic root, and hand the
/// module file's path back for the replacement phase to parse.
///
/// `vocab_home` — the path of `formats/vocabularies.json` — stands in
/// for the schema argument the doc-reading passes take: the synthetic
/// document IS the vocabulary home, unfolded, and a refusal that names
/// it names the file a human edits.
pub(crate) fn emit_shared_module(
    binary: &Path,
    out_dir: &Path,
    shared_doc: &Path,
    vocab_home: &Path,
    strictness: &SharedStrictness,
) -> Result<PathBuf> {
    let sub_out = out_dir.join(SHARED_MODULE);
    std::fs::create_dir_all(&sub_out)
        .with_context(|| format!("creating the shared module dir {}", sub_out.display()))?;
    eprintln!("  - vocabularies (shared) → {}/", sub_out.display());
    let status = Command::new(binary)
        .arg("--rust-out")
        .arg(&sub_out)
        .arg(shared_doc)
        .status()
        .with_context(|| format!("spawning {}", binary.display()))?;
    if !status.success() {
        bail!(
            "jtd-codegen failed for the shared vocabulary document {} \
             (exit code {:?})",
            shared_doc.display(),
            status.code()
        );
    }
    let file = sub_out.join("mod.rs");
    // The one entry, with the one slot ruled per fragment — see this
    // file's header. Refusals inside the passes name `vocab_home`, the
    // document's authored side, because that is the file a human edits.
    rewrite_generated(
        &file,
        shared_doc,
        vocab_home,
        StrictnessSource::Shared(strictness),
    )?;
    // The parasitic root is not a pass: it is the one emission a
    // definitions-only document adds, so it is stripped after the shared
    // pipeline rather than inside it — no schema module ever carries it,
    // and a pass that looked for it would refuse on all thirteen.
    let name = file.display().to_string();
    let src = std::fs::read_to_string(&file)
        .with_context(|| format!("reading the post-processed {}", name))?;
    let stripped = strip_parasitic_root(&src, &name)?;
    super::super::write::write_generated(&file, &stripped)
        .with_context(|| format!("writing the stripped {name}"))?;
    Ok(file)
}

/// Strip the parasitic root alias and exactly the import it strands.
/// The alias is the root form's rendering — `Option<Value>` — and it
/// is the only user `serde_json::Value` has in this file, so the
/// import goes with it; an import with a user left behind stays (the
/// same rule the domain-types pass follows, and `prune_orphan_imports`
/// encodes). A removed line between two blanks takes the second blank
/// with it, so the file stays `cargo fmt --all --check`-clean — a
/// generated file is never hand-formatted.
pub(super) fn strip_parasitic_root(src: &str, file: &str) -> Result<String> {
    if !src
        .split_inclusive('\n')
        .any(|chunk| chunk.trim_end_matches(['\r', '\n']).trim() == PARASITIC_ROOT)
    {
        bail!(
            "{file}: the parasitic root `{PARASITIC_ROOT}` is absent. A \
             definitions-only document makes the generator emit exactly \
             that line — the stem of the synthetic schema mints the name \
             — so its absence means the pinned emission shape has moved.\n\
             Fix: restore the pinned jtd-codegen version (the synthetic \
             document's stem lives in `vocabulary.rs` as `SHARED_STEM`; \
             the two must agree), or teach `strip_parasitic_root` in \
             `xtask/src/codegen/shared_module/emit.rs` the new shape, \
             then run `cargo xtask codegen`."
        );
    }
    let mut out = String::with_capacity(src.len());
    let mut previous_blank = false;
    let mut squash_next_blank = false;
    for chunk in src.split_inclusive('\n') {
        let blank = chunk.trim().is_empty();
        if squash_next_blank {
            squash_next_blank = false;
            if blank {
                continue;
            }
        }
        if chunk.trim_end_matches(['\r', '\n']).trim() == PARASITIC_ROOT {
            squash_next_blank = previous_blank;
            continue;
        }
        out.push_str(chunk);
        previous_blank = blank;
    }
    prune_orphan_imports(&out, file)
}
