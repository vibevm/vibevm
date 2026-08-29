//! T10B typed subjects at birth: the constructors that mint one, and the two
//! that deliberately still do not.
//!
//! The ABI §5.1 revisit trigger fired here. `Undetermined` was the honest
//! answer for every producer while `origin` was the only provenance an input
//! carried; a caller that HAS the typed components now says so, and the two
//! forms sit side by side so the difference is a property of the call, never
//! of the caller's luck.

use specmark::verifies;

use vibe_core::{Group, PackageName};

use super::{ArtifactInput, ArtifactInputKind, DocumentProvider};
use crate::SpecAddress;

/// The four provider identities a workspace-built input can carry — the
/// dependency arm plus the three host arms, exactly the shapes the kernel's
/// own host identity has.
fn every_declarable_provider() -> Vec<DocumentProvider> {
    let group = Group::parse("org.demo").expect("a valid test group");
    let name = PackageName::parse("tools").expect("a valid test package name");
    vec![
        DocumentProvider::Dependency {
            group: group.clone(),
            name: name.clone(),
        },
        DocumentProvider::HostCoordinate { group, name },
        DocumentProvider::HostUngrouped {
            name: "demo".to_owned(),
        },
        DocumentProvider::HostVirtualWorkspace,
    ]
}

/// A `normal` input built with a typed provider carries exactly it, beside
/// the row's OWN declared path — which is deliberately not the seed
/// address' `doc_path`.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn a_typed_normal_input_carries_the_exact_provider_and_the_rows_own_path() {
    for provider in every_declarable_provider() {
        let seed = SpecAddress::parse("spec://org.demo/tools/boot/entry#root")
            .expect("the seed address parses");
        let input = ArtifactInput::normal_declared_by(
            "org.demo/tools",
            "boot/declared.md",
            seed,
            provider.clone(),
        )
        .expect("a lawful typed normal contribution");
        assert_eq!(input.subject().provider(), &provider);
        assert_eq!(
            input.subject().declared_path(),
            "boot/declared.md",
            "the subject's path is the row's, never the seed address' `boot/entry`"
        );
    }
}

/// A `simple` input built with a typed provider carries it, AND the document
/// it already holds carries the same one — the two cannot disagree, and
/// `ArtifactPlan::new`'s own check would refuse them if they did.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn a_typed_simple_input_and_the_document_it_holds_share_one_subject() {
    for provider in every_declarable_provider() {
        let input = ArtifactInput::simple_declared_by(
            "org.demo/tools",
            "boot/simple.md",
            "# Simple {#root}\nBODY\n",
            provider.clone(),
        )
        .expect("a lawful typed simple contribution");
        assert_eq!(input.subject().provider(), &provider);
        let ArtifactInputKind::Simple { source, .. } = input.kind() else {
            panic!("a simple contribution holds its source")
        };
        assert_eq!(
            source.subject(),
            input.subject(),
            "one subject per document — the input's and its source's are the same value"
        );
    }
}

/// The compatibility forms still say `Undetermined`, and that is the honest
/// answer rather than a leftover: they take provenance as a display string
/// only, so no typed provider exists at the call. They are kept because
/// crate-internal callers — the wire rebuild, the compatibility wrapper —
/// have nothing else to say.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn the_display_only_forms_still_answer_undetermined() {
    let seed =
        SpecAddress::parse("spec://org.demo/tools/boot/entry#root").expect("the seed parses");
    let normal = ArtifactInput::normal("org.demo/tools", "boot/declared.md", seed)
        .expect("the legacy form still builds");
    assert_eq!(normal.subject().provider(), &DocumentProvider::Undetermined);
    let simple = ArtifactInput::simple("org.demo/tools", "boot/simple.md", "# S {#root}\n")
        .expect("the legacy form still builds");
    assert_eq!(simple.subject().provider(), &DocumentProvider::Undetermined);
    // And the untyped and typed forms differ in EXACTLY the provider: same
    // kind, same meta, same everything else.
    let seed =
        SpecAddress::parse("spec://org.demo/tools/boot/entry#root").expect("the seed parses");
    let typed = ArtifactInput::normal_declared_by(
        "org.demo/tools",
        "boot/declared.md",
        seed,
        DocumentProvider::HostUngrouped {
            name: "demo".to_owned(),
        },
    )
    .expect("the typed form builds");
    assert_eq!(typed.meta(), normal.meta());
    assert_ne!(typed.subject(), normal.subject());
}

/// `elided` and `hoisted` gain nothing, deliberately: they produce no
/// document, so no source/document transform is ever invoked for them and
/// there is no subject for a selector to judge.
#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-054#CONTRIB-SELECTOR")]
fn the_document_less_kinds_gain_no_typed_form() {
    let elided = ArtifactInput::elided("org.demo/tools", "boot/STATIC.md")
        .expect("an elided contribution builds");
    let target =
        SpecAddress::parse("spec://org.demo/tools/boot/entry").expect("the hoist target parses");
    let hoisted = ArtifactInput::hoisted("org.demo/tools", "boot/hoisted.md", target)
        .expect("a hoisted contribution builds");
    for input in [&elided, &hoisted] {
        assert_eq!(
            input.subject().provider(),
            &DocumentProvider::Undetermined,
            "a kind that produces no document carries the subject it always did"
        );
    }
}
