//! One domain-framed identity recipe for exact migration proposals.
specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-044#MANIFEST-MIGRATION-PLAN");
use super::MigrationTokenEdit;
use sha2::{Digest, Sha256};

fn frame(digest: &mut Sha256, bytes: &[u8]) {
    digest.update((bytes.len() as u64).to_be_bytes());
    digest.update(bytes);
}

pub(super) fn diff_identity(
    source: &str,
    target: &str,
    recipe: &str,
    text: &str,
    after: &str,
    edits: &[MigrationTokenEdit],
) -> String {
    let mut digest = Sha256::new();
    frame(&mut digest, b"vibe-manifest-migration-diff-r1");
    for value in [source, target, recipe, text, after] {
        frame(&mut digest, value.as_bytes());
    }
    frame(&mut digest, &(edits.len() as u64).to_be_bytes());
    for edit in edits {
        frame(&mut digest, edit.field.as_bytes());
        frame(&mut digest, &(edit.before_span.start as u64).to_be_bytes());
        frame(&mut digest, &(edit.before_span.end as u64).to_be_bytes());
        frame(&mut digest, edit.before_token.as_bytes());
        frame(&mut digest, edit.after_token.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

pub(super) fn identity(domain: &[u8], values: &[&[u8]]) -> String {
    let mut digest = Sha256::new();
    frame(&mut digest, domain);
    for bytes in values {
        frame(&mut digest, bytes);
    }
    format!("{:x}", digest.finalize())
}
