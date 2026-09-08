use vibe_core::manifest::{ExtensionIrLevel, ExtensionPass, ExtensionPassKind};

use crate::state::fingerprint::pass_payload_fingerprint_for_test;

fn raw_pass(kind: ExtensionPassKind) -> ExtensionPass {
    ExtensionPass {
        kind,
        level: Some(ExtensionIrLevel::Closure),
        from: None,
        to: None,
        after: Some("qualify".into()),
        before: None,
        replace: None,
        formats: None,
        artifact: None,
    }
}

#[test]
fn pass_kind_alone_moves_the_production_declaration_frame() {
    let transform = raw_pass(ExtensionPassKind::Transform);
    let mut lowering = transform.clone();
    lowering.kind = ExtensionPassKind::Lowering;
    assert_ne!(
        pass_payload_fingerprint_for_test(&transform),
        pass_payload_fingerprint_for_test(&lowering),
    );
}
