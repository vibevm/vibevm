use sha2::{Digest, Sha256};

use super::*;

const LINK_DIGEST_DOMAIN: &[u8] = b"vibe-spec/link-input/v2";

pub(super) fn digest_input(closure: &ClosureIr, plan: &PlannedLink) -> LinkInputDigest {
    let mut digest = LinkDigest::new();
    digest.field(LINK_DIGEST_DOMAIN);
    digest.field(closure.context().artifact().as_str().as_bytes());
    let target = closure.context().target();
    digest.byte(if target.is_static_markdown() {
        0
    } else if target.is_static_xml() {
        1
    } else {
        2
    });
    match closure.context().frame() {
        ArtifactFrame::CompatibilityFragment => digest.byte(0),
        ArtifactFrame::StaticLane {
            generated_path,
            source_root,
        } => {
            digest.byte(1);
            digest.field(generated_path.as_bytes());
            digest.field(source_root.as_bytes());
        }
    }
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
    digest.usize(closure.edges.len());
    for edge in &closure.edges {
        digest.usize(edge.from.0);
        digest.usize(edge.to.0);
        digest.byte(match edge.kind {
            ClosureEdgeKind::Use => 0,
            ClosureEdgeKind::Source => 1,
            ClosureEdgeKind::Embed => 2,
        });
        digest.field(edge.requested_target.to_string().as_bytes());
    }
    digest.usize(plan.contributions.len());
    for contribution in &plan.contributions {
        hash_contribution(&mut digest, contribution);
    }
    digest.usize(plan.occurrences.len());
    for occurrence in &plan.occurrences {
        hash_occurrence(&mut digest, occurrence);
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
        LinkContributionWitness::Elided { meta } => {
            digest.byte(2);
            hash_meta(digest, meta);
        }
        LinkContributionWitness::Hoisted { meta, target } => {
            digest.byte(3);
            hash_meta(digest, meta);
            digest.field(target.to_string().as_bytes());
        }
    }
}

fn hash_occurrence(digest: &mut LinkDigest, occurrence: &InputOccurrence) {
    match occurrence {
        InputOccurrence::Normal {
            contribution,
            occurrence,
            node,
            address,
            marker,
            body,
        } => {
            digest.byte(0);
            digest.usize(*contribution);
            digest.usize(*occurrence);
            digest.usize(node.0);
            digest.field(address.to_string().as_bytes());
            digest.field(marker.as_str().as_bytes());
            digest.field(body.as_bytes());
        }
        InputOccurrence::Simple {
            contribution,
            occurrence,
            address,
            body,
        } => {
            digest.byte(1);
            digest.usize(*contribution);
            digest.usize(*occurrence);
            hash_address(digest, address);
            digest.field(body.as_bytes());
        }
    }
}
