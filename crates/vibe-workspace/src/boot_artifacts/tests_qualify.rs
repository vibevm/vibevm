//! The qualified-splice surface of `render_static` — B-011 (qualify-on-splice,
//! preamble, tombstone, `@!X`, R3/R4) and its B-006 rider (per-node qualify
//! for cross-origin `normal` closures). Split from `tests.rs` along the
//! feature seam when the file outgrew the 600-line budget; the shared entry
//! builders stay in [`super::tests`].

use super::*;
use tempfile::TempDir;
use vibe_core::manifest::LinkType;
use vibe_spec::{DocTree, FileResolver, FsSectionSource, SectionSource, SpecAddress};

use super::tests::{boot, coord, entry, entry_normal};

// ----- B-011 (W3): qualify-on-splice, preamble, tombstone, @!X, R3, R4 -----

/// Write a `simple` boot file into a fresh dependency slot.
#[cfg(test)]
fn write_simple_boot(ws: &Path, slot: &str, body: &str) -> String {
    let rel = crate::layout_paths::vibedeps(format!("{slot}/1.0.0/boot.md"));
    let p = ws.join(&rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(&p, body).unwrap();
    rel
}

#[test]
fn golden_splice_qualifies_colliding_labels_and_emits_preamble_and_tombstone() {
    // The B-011 golden (acceptance 1): two simple contributions whose
    // `{#root}` and `##FACT` labels collide. After qualify-on-splice every
    // label is unique by construction, the resolution preamble leads, and the
    // tombstone names each retired short name with its qualified heirs.
    let ws = TempDir::new().unwrap();
    let alpha = write_simple_boot(
        ws.path(),
        "org.demo.alpha",
        "# Alpha {#root}\n\n##FACT the alpha fact.\n",
    );
    let beta = write_simple_boot(
        ws.path(),
        "org.demo.beta",
        "# Beta {#root}\n\n##FACT the beta fact.\n",
    );
    let b = boot(vec![
        entry(&alpha, LinkType::Static, "org.demo/alpha"),
        entry(&beta, LinkType::Static, "org.demo/beta"),
    ]);
    let text = render_static(&b, ws.path(), &coord()).unwrap().unwrap();

    // (1) Zero duplicate anchors over the colliding splice — the gate.
    let dups = DocTree::parse(&text).duplicate_anchors().to_vec();
    assert!(
        dups.is_empty(),
        "colliding labels must be qualified apart:\n{text}"
    );
    // The qualified labels are present, one origin each.
    assert!(text.contains("{#org-demo--alpha--root}"), "{text}");
    assert!(text.contains("{#org-demo--beta--root}"), "{text}");
    assert!(text.contains("##org-demo--alpha--FACT"), "{text}");

    // (2) The resolution preamble, verbatim (rule 1 + the header line).
    assert!(text.contains("RESOLUTION RULES"), "{text}");
    assert!(
        text.contains("qualified: <origin-slug>--<original>"),
        "{text}"
    );

    // (3) The tombstone, directly under the header: each short name with both
    // qualified heirs and origins (FACT sorts before root in the BTreeMap).
    assert!(
        text.contains("RENAMED ANCHORS (short → qualified heirs):"),
        "{text}"
    );
    assert!(
        text.contains(
            "root → org-demo--alpha--root (org.demo/alpha), org-demo--beta--root (org.demo/beta)"
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "FACT → org-demo--alpha--FACT (org.demo/alpha), org-demo--beta--FACT (org.demo/beta)"
        ),
        "{text}"
    );
}

#[test]
fn render_static_omits_the_tombstone_when_no_label_was_renamed() {
    // The tombstone appears only when the qualify phase renamed something. A
    // label-free lane carries no tombstone (and the preamble still leads).
    let ws = TempDir::new().unwrap();
    let p = write_simple_boot(
        ws.path(),
        "org.demo.plain",
        "Plain prose, no labels at all.",
    );
    let b = boot(vec![entry(&p, LinkType::Static, "org.demo/plain")]);
    let text = render_static(&b, ws.path(), &coord()).unwrap().unwrap();
    // The tombstone's specific opener is absent (the preamble documents a
    // "RENAMED ANCHORS table" in rule 2, so a literal `contains` would lie).
    assert!(
        !text.contains("RENAMED ANCHORS (short → qualified heirs):"),
        "{text}"
    );
    assert!(text.contains("RESOLUTION RULES"), "{text}");
}

/// Write a `normal` fixture whose contract `#use … as pre`s a prelude and
/// references it via `@!pre` — exercising the compiled-lane `@!X` rewrite
/// (acceptance 1's fourth sub-assertion; necessarily normal-path, since R3
/// forbids `@!` in a `simple` contribution).
#[cfg(test)]
fn write_aliaser_fixture(ws: &Path) -> String {
    let slot = crate::layout_paths::vibedeps("com.example.hello.aliaser/1.0.0");
    let base = ws.join(crate::layout_paths::slot_specs_path(&slot, ""));
    let write = |rel: &str, body: &str| {
        let p = base.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, body).unwrap();
    };
    write(
        "contract/greeting.md",
        "# Greeting {#greet-root}\n\
         #use spec://com.example.hello/aliaser/contract/prelude#root as pre\n\
         Sees @!pre here.\n",
    );
    write("contract/prelude.md", "# Prelude {#root}\n\nPRELUDE_BODY\n");
    crate::layout_paths::slot_specs(slot, "contract/greeting.md")
}

#[test]
fn render_static_rewrites_at_bang_to_the_full_address_in_a_normal_closure() {
    let ws = TempDir::new().unwrap();
    let contract = write_aliaser_fixture(ws.path());
    let b = boot(vec![entry_normal(&contract, "com.example.hello/aliaser")]);
    let text = render_static(&b, ws.path(), &coord()).unwrap().unwrap();

    // `@!pre` became the alias target's full address.
    assert!(
        text.contains("@spec://com.example.hello/aliaser/contract/prelude#root"),
        "{text}"
    );
    assert!(!text.contains("@!pre"), "{text}");
    // The aliased dependency is emitted (it is a real `#use` edge).
    assert!(text.contains("PRELUDE_BODY"), "{text}");
    // The alias declaration left with the stripped `#use` line; the contract's
    // own distinct label is qualified (no collision with the prelude's `#root`).
    assert!(
        text.contains("{#com-example-hello--aliaser--greet-root}"),
        "{text}"
    );
    assert!(
        text.contains("{#com-example-hello--aliaser--root}"),
        "{text}"
    );
    assert!(
        DocTree::parse(&text).duplicate_anchors().is_empty(),
        "{text}"
    );
}

#[test]
fn render_static_errors_when_a_simple_contribution_carries_an_as_clause() {
    // R3: a `#use … as <Alias>` clause is `normal`-format machinery; a `simple`
    // contribution is carried whole and cannot bind aliases.
    let ws = TempDir::new().unwrap();
    let p = write_simple_boot(
        ws.path(),
        "org.demo.bad",
        "# Bad {#root}\n#use spec://org.demo/other/doc#root as dep\nbody\n",
    );
    let b = boot(vec![entry(&p, LinkType::Static, "org.demo/bad")]);
    let err = render_static(&b, ws.path(), &coord()).unwrap_err();
    let WorkspaceError::InlineCompile { reason } = err else {
        panic!("expected InlineCompile, got {err:?}");
    };
    assert!(reason.contains("alias machinery"), "{reason}");
    assert!(reason.contains("PROP-035 §7.2"), "{reason}");
}

#[test]
fn render_static_errors_when_a_simple_contribution_carries_at_bang() {
    // R3: an `@!<Alias>` use is likewise `normal`-format machinery.
    let ws = TempDir::new().unwrap();
    let p = write_simple_boot(
        ws.path(),
        "org.demo.bad2",
        "# Bad {#root}\nSees @!dep here.\n",
    );
    let b = boot(vec![entry(&p, LinkType::Static, "org.demo/bad2")]);
    let err = render_static(&b, ws.path(), &coord()).unwrap_err();
    let WorkspaceError::InlineCompile { reason } = err else {
        panic!("expected InlineCompile, got {err:?}");
    };
    assert!(reason.contains("alias machinery"), "{reason}");
}

#[test]
fn fs_section_source_surfaces_qualified_candidates_on_a_short_anchor_miss() {
    // R4 (B-011 §6.1 layer 3): a missed short anchor answers with its qualified
    // heirs, never emptiness. A document whose `#root` was qualified to
    // `org-x--aaa--root` is queried for the short `root` — the resolver's miss
    // error lists the heir.
    let ws = TempDir::new().unwrap();
    let doc = ws
        .path()
        .join(crate::layout_paths::specs_path("common/TARGET.md"));
    fs::create_dir_all(doc.parent().unwrap()).unwrap();
    fs::write(
        &doc,
        "# Target {#org-x--aaa--root}\n##org-x--aaa--FACT a fact\n",
    )
    .unwrap();
    let src = FsSectionSource::new(FileResolver::new(ws.path(), coord()));
    let addr = SpecAddress::parse("spec://org.vibevm.core/vibevm/common/TARGET#root").unwrap();
    let err = src.section_text(&addr).unwrap_err();
    assert!(err.contains("anchor not found"), "{err}");
    assert!(err.contains("qualified candidates for `root`"), "{err}");
    assert!(err.contains("org-x--aaa--root"), "{err}");
}

// ----- B-006 rider (W-B): per-node qualify for cross-origin closures --------

/// Write a `normal` fixture whose host contract `#use`s a node in a DIFFERENT
/// package — the case per-node qualification exists for. The host package
/// `com.example.host/host` references the dep package's `##THE-RULE` via a
/// short link; without per-node qualify the dep's labels would be
/// mis-attributed to the host's origin. Each package lives in its own
/// dependency slot, matched by the `-<name>` suffix the
/// resolver keys on (identity `com.example.host/host` → slot `com.example.host.host`; `com.example.dep/dep` → slot
/// `com.example.dep.dep`). Returns the host contract's workspace-relative path.
#[cfg(test)]
fn write_cross_pkg_fixture(ws: &Path) -> String {
    let write = |slot: &str, rel: &str, body: &str| {
        let slot = crate::layout_paths::vibedeps(format!("{slot}/1.0.0"));
        let p = ws.join(crate::layout_paths::slot_specs_path(slot, rel));
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(&p, body).unwrap();
    };
    write(
        "com.example.host.host",
        "contract/host.md",
        "# Host {#root}\n\
         #use spec://com.example.dep/dep/contract/dep#root\n\
         See (#THE-RULE) from the dep.\n\
         HOST_BODY\n",
    );
    write(
        "com.example.dep.dep",
        "contract/dep.md",
        "# Dep {#root}\n##THE-RULE the rule\nDEP_BODY\n",
    );
    crate::layout_paths::slot_specs(
        crate::layout_paths::vibedeps("com.example.host.host/1.0.0"),
        "contract/host.md",
    )
}

#[test]
fn render_static_qualifies_a_normal_closure_per_node_across_packages() {
    // Q7 (E4-W2-NODE-QUALIFY): a normal closure spanning two packages is
    // qualified PER-NODE — the dep's labels carry the dep's origin, never the
    // host entry's — and the cross-node short link resolves to the dep's
    // qualified heir. `qualify_contribution` is NOT run over the compiled body
    // (no double prefix); the per-node renames land in the tombstone.
    let ws = TempDir::new().unwrap();
    let contract = write_cross_pkg_fixture(ws.path());
    let b = boot(vec![entry_normal(&contract, "com.example.host/host")]);
    let text = render_static(&b, ws.path(), &coord()).unwrap().unwrap();

    // (1) The dep's labels are qualified under the DEP's origin — per-node, not
    // the host entry's. Whole-body qualify under the entry's origin could never
    // produce this.
    assert!(text.contains("{#com-example-dep--dep--root}"), "{text}");
    assert!(text.contains("##com-example-dep--dep--THE-RULE"), "{text}");
    // The host's own label is under the host's origin.
    assert!(text.contains("{#com-example-host--host--root}"), "{text}");
    // (2) The cross-node short link resolved to the dep's qualified heir.
    assert!(text.contains("(#com-example-dep--dep--THE-RULE)"), "{text}");
    // (3) No double prefix: the dep's label was neither mis-attributed to the
    // host origin (whole-body qualify) nor double-prefixed (whole-body over the
    // per-node compile).
    assert!(
        !text.contains("##com-example-host--host--THE-RULE"),
        "dep's label mis-attributed to host origin: {text}"
    );
    assert!(
        !text.contains("com-example-host--host--com-example-dep"),
        "double prefix from re-qualifying: {text}"
    );
    // (4) The per-node renames land in the tombstone — both origins present.
    assert!(
        text.contains("THE-RULE → com-example-dep--dep--THE-RULE (com.example.dep/dep)"),
        "{text}"
    );
    assert!(
        text.contains("com-example-host--host--root (com.example.host/host)"),
        "{text}"
    );
    // (5) Zero duplicate anchors over the cross-package splice — the gate.
    assert!(
        DocTree::parse(&text).duplicate_anchors().is_empty(),
        "{text}"
    );
}
