//! Unit tests for [`super`], out-of-line per the file-length budget.
//! Included via `#[cfg(test)] #[path] mod tests;`, so the module-tree
//! position — and therefore `use super::*` — is unchanged from the inline
//! form. Non-`#[test]` helpers carry `#[cfg(test)]` so file-grain scanners
//! (the conform frontend) scope their `unwrap`s as test code.

use super::*;
use specmark::verifies;
use vibe_core::manifest::{PackageFormat, TargetOs};

#[cfg(test)]
fn authored(path: &str, category: Option<BootCategory>) -> AuthoredBoot {
    AuthoredBoot {
        path: path.to_string(),
        category,
        origin: ".".to_string(),
    }
}

/// The canonical first-party `Group` for tests.
#[cfg(test)]
fn org() -> Group {
    Group::parse("org.vibevm").unwrap()
}

/// A dependency with a boot snippet, no link declarations, given edges.
#[cfg(test)]
fn dep(name: &str, has_boot: bool, requires: &[&str]) -> DependencyBoot {
    DependencyBoot {
        kind: PackageKind::Flow,
        group: org(),
        name: name.to_string(),
        boot_path: has_boot.then(|| format!("vibedeps/flow-{name}/1.0.0/boot.md")),
        category: None,
        declared_link: None,
        suggested_link: None,
        when: None,
        requires: requires.iter().map(|r| (org(), r.to_string())).collect(),
        format: Default::default(),
    }
}

#[cfg(test)]
fn compute(
    own: &[AuthoredBoot],
    inherited: &[AuthoredBoot],
    deps: &[DependencyBoot],
    default_link: Option<LinkType>,
) -> EffectiveBoot {
    compute_effective_boot(NodeBootInputs {
        own_boot: own,
        inherited_foundation: inherited,
        dependencies: deps,
        default_link,
    })
    .unwrap()
}

#[test]
fn empty_inputs_yield_empty_boot() {
    let boot = compute(&[], &[], &[], None);
    assert!(boot.is_empty());
}

#[test]
#[verifies("spec://vibevm/modules/vibe-workspace/PROP-035#formats")]
fn dependency_format_reaches_its_boot_entry() {
    // The static renderer keys on `BootEntry.format` to decide compile vs
    // verbatim (PROP-035 §8); the composition must carry a `normal`
    // dependency's format through to its entry, and a node's own authored
    // boot stays `simple`.
    let mut d = dep("greeter", true, &[]);
    d.format = PackageFormat::Normal;
    d.suggested_link = Some(LinkType::Static);
    let own = vec![authored("spec/boot/notes.md", None)];
    let boot = compute(&own, &[], &[d], None);

    let dep_entry = boot
        .entries
        .iter()
        .find(|e| e.origin == "org.vibevm/greeter")
        .expect("the dependency contributes an entry");
    assert_eq!(dep_entry.format, PackageFormat::Normal);
    assert_eq!(dep_entry.link, LinkType::Static);

    let own_entry = boot
        .entries
        .iter()
        .find(|e| e.path == "spec/boot/notes.md")
        .expect("the node's own boot contributes an entry");
    assert_eq!(
        own_entry.format,
        PackageFormat::Simple,
        "a node's own authored boot is always simple"
    );
}

#[test]
fn own_boot_bands_by_category() {
    let own = vec![
        authored("spec/boot/00-core.md", Some(BootCategory::Foundation)),
        authored("spec/boot/notes.md", None),
        authored("spec/boot/90-user.md", Some(BootCategory::UserOverride)),
    ];
    let boot = compute(&own, &[], &[], None);
    let bands: Vec<BootBand> = boot.entries.iter().map(|e| e.band).collect();
    assert_eq!(
        bands,
        vec![
            BootBand::Foundation,
            BootBand::NodeOwn,
            BootBand::UserOverride
        ]
    );
    // Authored boot is always `static`.
    assert!(boot.entries.iter().all(|e| e.link == LinkType::Dynamic));
}

#[test]
fn declared_order_survives_even_when_input_is_shuffled() {
    // Override declared before foundation — the engine re-bands it.
    let own = vec![
        authored("spec/boot/90-user.md", Some(BootCategory::UserOverride)),
        authored("spec/boot/00-core.md", Some(BootCategory::Foundation)),
    ];
    let boot = compute(&own, &[], &[], None);
    assert_eq!(boot.entries[0].path, "spec/boot/00-core.md");
    assert_eq!(boot.entries[1].path, "spec/boot/90-user.md");
}

#[test]
fn inherited_foundation_precedes_own_foundation() {
    let inherited = vec![authored(
        "spec/boot/00-core.md",
        Some(BootCategory::Foundation),
    )];
    let own = vec![authored(
        "packages/x/spec/boot/00-core.md",
        Some(BootCategory::Foundation),
    )];
    let boot = compute(&own, &inherited, &[], None);
    assert_eq!(boot.entries[0].path, "spec/boot/00-core.md");
    assert_eq!(boot.entries[1].path, "packages/x/spec/boot/00-core.md");
}

#[test]
fn dependency_boot_is_topologically_ordered() {
    // `a` requires `b` — so `b` must come first.
    let deps = vec![dep("a", true, &["b"]), dep("b", true, &[])];
    let boot = compute(&[], &[], &deps, None);
    let origins: Vec<&str> = boot.entries.iter().map(|e| e.origin.as_str()).collect();
    assert_eq!(origins, vec!["org.vibevm/b", "org.vibevm/a"]);
    assert!(boot.entries.iter().all(|e| e.band == BootBand::Dependency));
}

#[test]
fn bootless_dependency_contributes_no_entry_but_still_orders() {
    // `a` → `m` (no boot) → `b`. The topo order is b, m, a; filtered
    // to boot-bearing packages it is b, a — `m` still transmitted the
    // ordering between them.
    let deps = vec![
        dep("a", true, &["m"]),
        dep("m", false, &["b"]),
        dep("b", true, &[]),
    ];
    let boot = compute(&[], &[], &deps, None);
    let origins: Vec<&str> = boot.entries.iter().map(|e| e.origin.as_str()).collect();
    assert_eq!(origins, vec!["org.vibevm/b", "org.vibevm/a"]);
}

#[test]
fn link_precedence_declared_beats_suggested_and_default() {
    let mut d = dep("x", true, &[]);
    d.declared_link = Some(LinkType::Dynamic);
    d.suggested_link = Some(LinkType::Static);
    let boot = compute(&[], &[], &[d], Some(LinkType::Dynamic));
    assert_eq!(boot.entries[0].link, LinkType::Dynamic);
}

#[test]
fn link_precedence_suggested_beats_default() {
    let mut d = dep("x", true, &[]);
    d.suggested_link = Some(LinkType::Static);
    let boot = compute(&[], &[], &[d], Some(LinkType::Dynamic));
    assert_eq!(boot.entries[0].link, LinkType::Static);
}

#[test]
fn link_precedence_falls_through_to_default() {
    let d = dep("x", true, &[]);
    let boot = compute(&[], &[], &[d], Some(LinkType::Dynamic));
    assert_eq!(boot.entries[0].link, LinkType::Dynamic);
}

#[test]
fn link_precedence_defaults_to_static() {
    let d = dep("x", true, &[]);
    let boot = compute(&[], &[], &[d], None);
    assert_eq!(boot.entries[0].link, LinkType::Dynamic);
}

#[test]
fn dependency_with_foundation_category_joins_the_foundation_band() {
    let mut d = dep("x", true, &[]);
    d.category = Some(BootCategory::Foundation);
    let boot = compute(&[], &[], &[d], None);
    assert_eq!(boot.entries[0].band, BootBand::Foundation);
}

#[test]
fn dependency_cycle_is_rejected() {
    let deps = vec![dep("a", true, &["b"]), dep("b", true, &["a"])];
    let err = compute_effective_boot(NodeBootInputs {
        own_boot: &[],
        inherited_foundation: &[],
        dependencies: &deps,
        default_link: None,
    })
    .unwrap_err();
    match err {
        WorkspaceError::BootDependencyCycle { packages } => {
            assert!(packages.contains("org.vibevm/a"), "{packages}");
            assert!(packages.contains("org.vibevm/b"), "{packages}");
        }
        other => panic!("expected a boot dependency cycle, got {other}"),
    }
}

#[test]
fn inline_and_dynamic_entries_split_by_link() {
    let mut inline = dep("crit", true, &[]);
    inline.declared_link = Some(LinkType::Static);
    let mut dynamic = dep("rust", true, &[]);
    dynamic.declared_link = Some(LinkType::Dynamic);
    let plain = dep("wal", true, &[]); // static
    let boot = compute(&[], &[], &[inline, dynamic, plain], None);

    let inline_origins: Vec<&str> = boot.static_entries().map(|e| e.origin.as_str()).collect();
    assert_eq!(inline_origins, vec!["org.vibevm/crit"]);

    let indexed_origins: Vec<&str> = boot.dynamic_entries().map(|e| e.origin.as_str()).collect();
    // `static` and `dynamic` both land in the index, in composed order.
    assert_eq!(indexed_origins, vec!["org.vibevm/rust", "org.vibevm/wal"]);
}

#[test]
fn full_composition_orders_all_four_bands() {
    let inherited = vec![authored(
        "spec/boot/00-core.md",
        Some(BootCategory::Foundation),
    )];
    let own = vec![
        authored("packages/x/spec/boot/intro.md", None),
        authored(
            "packages/x/spec/boot/90-user.md",
            Some(BootCategory::UserOverride),
        ),
    ];
    let deps = vec![dep("a", true, &["b"]), dep("b", true, &[])];
    let boot = compute(&own, &inherited, &deps, None);
    let bands: Vec<BootBand> = boot.entries.iter().map(|e| e.band).collect();
    assert_eq!(
        bands,
        vec![
            BootBand::Foundation,   // inherited 00-core.md
            BootBand::NodeOwn,      // intro.md
            BootBand::Dependency,   // flow:b
            BootBand::Dependency,   // flow:a
            BootBand::UserOverride  // 90-user.md
        ]
    );
    let origins: Vec<&str> = boot.entries.iter().map(|e| e.origin.as_str()).collect();
    assert_eq!(origins, vec![".", ".", "org.vibevm/b", "org.vibevm/a", "."]);
}

// --- PROP-009 §2.4 / §2.6 — the `when` OS gate ----------------------

#[test]
fn when_propagates_to_the_boot_entry_and_forces_dynamic() {
    // A dependency with a `when` and no link declaration at all: the
    // condition rides through to the entry, and the entry is `dynamic`
    // even though the precedence chain would otherwise pick `static`.
    let mut d = dep("rust", true, &[]);
    d.when = Some(WhenCondition::Os(TargetOs::Linux));
    let boot = compute(&[], &[], &[d], None);
    assert_eq!(boot.entries[0].link, LinkType::Dynamic);
    assert_eq!(
        boot.entries[0].when,
        Some(WhenCondition::Os(TargetOs::Linux))
    );
}

#[test]
fn when_forces_dynamic_even_over_an_explicit_inline() {
    // The consumer asked for `inline`, but the package's snippet is
    // OS-conditional — `when` wins, because a condition cannot be
    // honoured by the verbatim inline lane.
    let mut d = dep("win-only", true, &[]);
    d.declared_link = Some(LinkType::Static);
    d.when = Some(WhenCondition::Os(TargetOs::Windows));
    let boot = compute(&[], &[], &[d], Some(LinkType::Dynamic));
    assert_eq!(boot.entries[0].link, LinkType::Dynamic);
    // And it lands in the index, not the inline lane.
    assert_eq!(boot.static_entries().count(), 0);
    let indexed: Vec<&str> = boot.dynamic_entries().map(|e| e.origin.as_str()).collect();
    assert_eq!(indexed, vec!["org.vibevm/win-only"]);
}

#[test]
fn authored_boot_never_carries_a_when() {
    // `when` is a property of a dependency's `[boot_snippet]`; a node's
    // own and inherited authored boot are unconditional.
    let inherited = vec![authored(
        "spec/boot/00-core.md",
        Some(BootCategory::Foundation),
    )];
    let own = vec![authored("spec/boot/notes.md", None)];
    let boot = compute(&own, &inherited, &[], None);
    assert!(boot.entries.iter().all(|e| e.when.is_none()));
}

#[test]
fn inline_transitive_resolves_to_inline_at_emission() {
    // A dependency whose resolved link is `inline-transitive` lands in the
    // inline lane (PROP-035 §12): bootgen propagated the mode across the
    // closure, and the engine just emits inline.
    let mut d = dep("x", true, &[]);
    d.declared_link = Some(LinkType::StaticTransitive);
    let boot = compute(&[], &[], &[d], None);
    assert_eq!(boot.entries[0].link, LinkType::Static);
    assert_eq!(boot.static_entries().count(), 1);
}
