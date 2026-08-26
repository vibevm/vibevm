use sha2::{Digest, Sha256};

use super::*;

const LINK_DIGEST_DOMAIN: &[u8] = b"vibe-spec/link-input/v1";

pub(super) fn digest_input(closure: &ClosureIr, plan: &PlannedLink) -> LinkInputDigest {
    let mut digest = LinkDigest::new();
    digest.field(LINK_DIGEST_DOMAIN);
    digest.field(closure.artifact.as_str().as_bytes());
    digest.byte(match plan.mode {
        StaticCompileMode::Plain => 0,
        StaticCompileMode::QualifyPerNode => 1,
    });
    digest.usize(closure.renames.len());
    for rename in &closure.renames {
        digest.field(rename.origin.as_bytes());
        digest.field(rename.rename.original.as_bytes());
        digest.field(rename.rename.qualified.as_bytes());
    }
    digest.usize(plan.contributions.len());
    for contribution in &plan.contributions {
        hash_contribution(&mut digest, contribution);
    }
    digest.usize(plan.chunks.len());
    for chunk in &plan.chunks {
        hash_chunk(&mut digest, chunk);
    }
    LinkInputDigest(digest.finish())
}

struct LinkDigest(Sha256);

impl LinkDigest {
    fn new() -> Self {
        Self(Sha256::new())
    }

    fn byte(&mut self, value: u8) {
        self.0.update([value]);
    }

    fn usize(&mut self, value: usize) {
        self.0.update((value as u64).to_le_bytes());
    }

    fn field(&mut self, value: &[u8]) {
        self.0.update((value.len() as u64).to_le_bytes());
        self.0.update(value);
    }

    fn finish(self) -> [u8; 32] {
        self.0.finalize().into()
    }
}

fn hash_meta(digest: &mut LinkDigest, meta: &ContributionMeta) {
    digest.field(meta.origin.as_bytes());
    digest.field(meta.path.as_bytes());
}

fn hash_address(digest: &mut LinkDigest, address: &DocumentAddress) {
    match address {
        DocumentAddress::Spec(address) => {
            digest.byte(0);
            digest.field(address.to_string().as_bytes());
        }
        DocumentAddress::StaticEntry { origin, path } => {
            digest.byte(1);
            digest.field(origin.as_bytes());
            digest.field(path.as_bytes());
        }
    }
}

fn hash_contribution(digest: &mut LinkDigest, contribution: &LinkContributionWitness) {
    match contribution {
        LinkContributionWitness::Normal {
            meta,
            seed,
            seed_address,
            occurrence_count,
        } => {
            digest.byte(0);
            hash_meta(digest, meta);
            digest.usize(seed.0);
            digest.field(seed_address.to_string().as_bytes());
            digest.usize(*occurrence_count);
        }
        LinkContributionWitness::Simple { meta, address } => {
            digest.byte(1);
            hash_meta(digest, meta);
            hash_address(digest, address);
        }
    }
}

fn hash_chunk(digest: &mut LinkDigest, chunk: &InputChunk) {
    match chunk {
        InputChunk::Literal { kind, bytes } => {
            digest.byte(0);
            digest.byte(match kind {
                LinkLiteralKind::NormalOpen => 0,
                LinkLiteralKind::ForcedNewline => 1,
                LinkLiteralKind::NormalClose => 2,
            });
            digest.field(bytes.as_bytes());
        }
        InputChunk::NormalOccurrence {
            contribution,
            occurrence,
            node,
            address,
            bytes,
        } => {
            digest.byte(1);
            digest.usize(*contribution);
            digest.usize(*occurrence);
            digest.usize(node.0);
            digest.field(address.to_string().as_bytes());
            digest.field(bytes.as_bytes());
        }
        InputChunk::SimpleOccurrence {
            contribution,
            occurrence,
            address,
            bytes,
        } => {
            digest.byte(2);
            digest.usize(*contribution);
            digest.usize(*occurrence);
            hash_address(digest, address);
            digest.field(bytes.as_bytes());
        }
    }
}
