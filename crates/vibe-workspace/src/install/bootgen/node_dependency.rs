//! One node's resolved dependency boot rows, including static unit expansion.

use super::*;

/// Build the dependency-boot inputs for one node: the transitive closure
/// of its `[requires]` within `resolution`, each turned into a
/// [`DependencyBoot`]. Each snippet's path is resolved LOGICALLY against
/// the materialised slot ([`resolve_snippet_source`], PROP-045
/// ##BOOT-LANE-SCOPE) so an XML-materialised dependency's boot path — and
/// the INDEX entry that carries it — names the file that exists.
pub(super) fn node_dependency_boot(
    workspace_root: &Path,
    node_manifest: &Manifest,
    resolution: &[ResolvedDep],
    table: &HashMap<UnitId, UnitInput>,
    with_static: &HashSet<UnitId>,
    spec_format: SpecFormat,
) -> Vec<DependencyBoot> {
    let installed = installed_identities(resolution);
    let index: HashMap<(&Group, &str), &ResolvedDep> = resolution
        .iter()
        .map(|d| ((&d.group, d.name.as_str()), d))
        .collect();

    // The inline-transitive closure (PROP-035 §12): every package reached
    // through a direct edge the consumer declared `inline-transitive` — the
    // edge's target and its whole `requires` closure — is forced `inline`.
    let forced_inline = static_transitive_closure(node_manifest, &index);
    let static_members =
        transitive::static_unit_members(node_manifest, resolution, table, with_static);

    // Breadth-first transitive closure from the node's direct requires.
    // A `[requires.packages]` key is group-qualified (PROP-008 §2.6), so
    // every `iter_pkgrefs` entry carries a group.
    let mut visited: HashSet<(Group, String)> = HashSet::new();
    let mut queue: VecDeque<(Group, String)> = node_manifest
        .requires
        .iter_pkgrefs()
        .filter_map(|(g, n)| g.map(|g| (g.clone(), n.to_string())))
        .collect();
    let mut closure: Vec<&ResolvedDep> = Vec::new();
    while let Some((group, name)) = queue.pop_front() {
        if !visited.insert((group.clone(), name.clone())) {
            continue;
        }
        if let Some(dep) = index.get(&(&group, name.as_str())) {
            closure.push(dep);
            for (rg, rn) in &dep.requires {
                queue.push_back((rg.clone(), rn.clone()));
            }
        }
    }

    closure
        .iter()
        .map(|dep| {
            // An in-place dependency's boot snippet lives in its unversioned
            // slot (PROP-022 §2.4); a copy/hardlink dep's in the versioned
            // one. Field access auto-derefs the `&&ResolvedDep`.
            let in_place = dep
                .manifest
                .package
                .as_ref()
                .is_some_and(|p| p.materialization.is_in_place());
            let slot = if in_place {
                vibedeps::in_place_slot_rel_path(&dep.group, &dep.name)
            } else {
                vibedeps::slot_rel_path(&dep.group, &dep.name, &dep.version)
            };
            let snippet = dep.manifest.boot_snippet.as_ref();
            let active = active_snippet(workspace_root, &slot, snippet, &installed);
            let main = active.main;
            let all_fragments = active.fragments;
            // PROP-038 §2.1: a dependency that statically links a child is read
            // through its compiled STATIC.md (carrying the whole zone), not its
            // raw snippet. A leaf keeps pointing at the snippet (byte-compat).
            // B-006: remember which entries had their path substituted up to a
            // unit-STATIC — `desubstitute_covered_units` rolls the substitution
            // back (or elides it) once the zone is covered member-by-member.
            let (boot_path, when, fragments, unit_substituted) = if with_static
                .contains(&(dep.group.clone(), dep.name.clone()))
                && !static_members.contains(&(dep.group.clone(), dep.name.clone()))
            {
                (
                    Some(path_to_slash(
                        &Path::new(&slot)
                            .join(layout::current_boot_dir())
                            .join(boot_artifacts::static_file(spec_format)),
                    )),
                    main.as_ref()
                        .and_then(|contribution| contribution.when.clone()),
                    all_fragments
                        .into_iter()
                        .filter(|fragment| fragment.when.is_some())
                        .collect(),
                    true,
                )
            } else {
                (
                    main.as_ref().map(|contribution| contribution.path.clone()),
                    main.and_then(|contribution| contribution.when),
                    all_fragments,
                    false,
                )
            };
            DependencyBoot {
                kind: dep.kind,
                group: dep.group.clone(),
                name: dep.name.clone(),
                boot_path,
                fragments,
                category: snippet.and_then(|bs| bs.category),
                // An `inline-transitive` edge (or membership in one's closure)
                // forces `inline` (PROP-035 §12); otherwise only a direct
                // requirement carries a consumer-declared `link` and a
                // transitive dependency reads back as `None`.
                declared_link: if forced_inline.contains(&(dep.group.clone(), dep.name.clone()))
                    || static_members.contains(&(dep.group.clone(), dep.name.clone()))
                {
                    Some(LinkType::Static)
                } else {
                    node_manifest.requires.declared_link(&dep.group, &dep.name)
                },
                suggested_link: snippet.and_then(|bs| bs.link),
                // Only an `os:*` predicate can remain after generation-time
                // `installed:*` resolution.
                when,
                requires: dep.requires.clone(),
                // PROP-035 §3 — the package's declared format. A `normal`
                // dependency pulled `static` is compiled to its closure by
                // `render_static` (PROP-035 §8); absent a `[package]` table,
                // it defaults to `simple` (verbatim, fail-safe).
                format: dep
                    .manifest
                    .package
                    .as_ref()
                    .map(|p| p.format)
                    .unwrap_or_default(),
                unit_substituted,
            }
        })
        .collect()
}
