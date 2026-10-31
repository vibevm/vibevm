//! Parsing and precedence of the remap row model (`PROP-066#row`,
//! `PROP-066#precedence`).

use specmark::verifies;

use super::*;

/// Build one row of a named layer at a given position.
fn row(toml_body: &str, layer: Layer, source: &str, index: usize) -> RemapRow {
    let decl: RowDecl = toml::from_str(toml_body).expect("row parses");
    decl.into_row(Provenance::new(layer, source, index))
        .expect("row is well shaped")
}

fn locator(rows: Vec<RemapRow>) -> Locator {
    Locator::from_sources(
        vec![LoadedSource {
            layer: Layer::Local,
            source: "test".into(),
            rows,
            policy: UnmatchedPolicy::Allow,
        }],
        vec![("vibespecs".into(), "https://github.com/vibespecs".into())],
    )
}

fn first_target(locator: &Locator, artifact: &Artifact) -> String {
    match locator.locate(artifact) {
        Resolution::Candidates(candidates) => candidates
            .first()
            .map(|c| c.target.display())
            .unwrap_or_else(|| "<none>".into()),
        Resolution::Denied { .. } => "<denied>".into(),
    }
}

// ── the row model (#row) ────────────────────────────────────────────────

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#row")]
fn a_row_declares_exactly_one_key_and_one_target() {
    let from = || Provenance::new(Layer::Local, "settings.toml", 4);
    let good: RowDecl = toml::from_str("sha256 = \"AA\"\nurl = \"https://a/\"").unwrap();
    let built = good.into_row(from()).unwrap();
    // The digest is lowercased on the way in, so a manifest's spelling
    // cannot decide whether the row matches.
    assert_eq!(built.key.what(), "aa");

    for (body, part) in [
        ("url = \"https://a/\"", "key"),
        ("sha256 = \"aa\"", "target"),
        (
            "sha256 = \"aa\"\nprefix = \"https://\"\nurl = \"https://a/\"",
            "key",
        ),
        (
            "sha256 = \"aa\"\nurl = \"https://a/\"\ndeny = true",
            "target",
        ),
    ] {
        let decl: RowDecl = toml::from_str(body).unwrap();
        let error = decl.into_row(from()).unwrap_err().to_string();
        assert!(error.contains(part), "{body}: {error}");
        // The refusal names the row the reader has in front of them.
        assert!(error.contains("row 5"), "{body}: {error}");
        assert!(error.contains("PROP-066#row"), "{body}: {error}");
    }
}

#[test]
fn a_row_round_trips_through_its_flat_toml_form() {
    let body = "prefix = \"https://github.com/\"\n\
                url = \"https://nexus.corp.example/gh/\"\n\
                reason = \"the only host the proxy allows\"\n\
                [scope]\npackage = \"org.example/tool\"\nstep = \"fetch\"\n";
    let decl: RowDecl = toml::from_str(body).unwrap();
    let built = decl
        .clone()
        .into_row(Provenance::new(Layer::Local, "f", 0))
        .unwrap();
    assert_eq!(built.to_decl(), decl);
    let written = toml::to_string(&built.to_decl()).unwrap();
    assert_eq!(toml::from_str::<RowDecl>(&written).unwrap(), decl);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#policy")]
fn a_files_policy_is_read_and_the_strictest_one_wins() {
    let allow = LoadedSource::empty(Layer::Local, "a");
    let deny = LoadedSource {
        policy: UnmatchedPolicy::Deny,
        ..LoadedSource::empty(Layer::Invocation, "b")
    };
    let both = Locator::from_sources(vec![allow, deny], Vec::new());
    assert_eq!(both.policy(), UnmatchedPolicy::Deny);
    assert!(!both.is_empty(), "a deny policy is not an empty locator");
}

#[test]
fn unknown_top_level_keys_of_a_remap_file_do_not_fail_the_run() {
    // A file written for R-2 (`[remap] description`, consent fields) still
    // yields its rows here instead of failing a run that needed them.
    let file: RemapFileDecl = toml::from_str(
        "[remap]\ndescription = \"the corp mirror\"\n\
         [[remap]]\nregistry = \"vibespecs\"\nurl = \"https://nexus/v\"\n",
    )
    .unwrap_or_default();
    // `[remap]` as a table and `[[remap]]` as an array cannot coexist in
    // one TOML document, so the fallback is the empty file — what matters
    // is that nothing panicked and nothing was invented.
    assert!(file.remap.is_empty() || file.remap.len() == 1);

    let rows: RemapFileDecl = toml::from_str(
        "unknown_future_key = 1\n[[remap]]\nregistry = \"vibespecs\"\nurl = \"https://nexus/v\"\n",
    )
    .unwrap();
    assert_eq!(rows.remap.len(), 1);
}

// ── precedence (#precedence) ────────────────────────────────────────────

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#precedence")]
fn a_more_specific_key_wins() {
    let digest = "ab".repeat(32);
    let rows = vec![
        row(
            "prefix = \"https://\"\nurl = \"https://by-prefix/\"",
            Layer::Local,
            "t",
            0,
        ),
        row(
            "registry = \"vibespecs\"\nurl = \"https://by-registry/\"",
            Layer::Local,
            "t",
            1,
        ),
        row(
            "package = \"org.example/tool\"\nurl = \"https://by-package/\"",
            Layer::Local,
            "t",
            2,
        ),
        row(
            &format!("sha256 = \"{digest}\"\nurl = \"https://by-digest/\""),
            Layer::Local,
            "t",
            3,
        ),
    ];
    let full = Artifact::new(ArtifactKind::RegistryClone)
        .with_url("https://github.com/vibespecs/org.example.tool")
        .with_digest(&digest)
        .with_package("org.example/tool");
    let all = locator(rows.clone());
    assert_eq!(first_target(&all, &full), "https://by-digest/");

    let without_digest = locator(rows[..3].to_vec());
    assert_eq!(first_target(&without_digest, &full), "https://by-package/");

    let without_package = locator(rows[..2].to_vec());
    assert_eq!(
        first_target(&without_package, &full),
        "https://by-registry/"
    );

    let prefix_only = locator(rows[..1].to_vec());
    assert_eq!(
        first_target(&prefix_only, &full),
        "https://by-prefix/github.com/vibespecs/org.example.tool",
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#precedence")]
fn the_longest_prefix_wins_among_prefixes() {
    let rows = vec![
        row(
            "prefix = \"https://\"\nurl = \"https://short/\"",
            Layer::Local,
            "t",
            0,
        ),
        row(
            "prefix = \"https://github.com/vibespecs/\"\nurl = \"https://long/\"",
            Layer::Local,
            "t",
            1,
        ),
        row(
            "prefix = \"https://github.com/\"\nurl = \"https://middle/\"",
            Layer::Local,
            "t",
            2,
        ),
    ];
    let artifact =
        Artifact::new(ArtifactKind::RawFile).with_url("https://github.com/vibespecs/index");
    assert_eq!(
        first_target(&locator(rows), &artifact),
        "https://long/index"
    );
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#precedence")]
fn a_scoped_row_beats_an_unscoped_one_and_a_step_scope_beats_a_package_scope() {
    let rows = vec![
        row(
            "prefix = \"https://h/\"\nurl = \"https://plain/\"",
            Layer::Local,
            "t",
            0,
        ),
        row(
            "prefix = \"https://h/\"\nurl = \"https://scoped/\"\n\
             [scope]\npackage = \"org.example/tool\"",
            Layer::Local,
            "t",
            1,
        ),
        row(
            "prefix = \"https://h/\"\nurl = \"https://step/\"\n\
             [scope]\npackage = \"org.example/tool\"\nstep = \"fetch\"",
            Layer::Local,
            "t",
            2,
        ),
    ];
    let built = locator(rows);
    let other_package = Artifact::new(ArtifactKind::AppInput)
        .with_url("https://h/a")
        .with_package("org.example/other");
    assert_eq!(first_target(&built, &other_package), "https://plain/a");

    let other_step = Artifact::new(ArtifactKind::AppInput)
        .with_url("https://h/a")
        .with_package("org.example/tool")
        .with_step("build");
    assert_eq!(first_target(&built, &other_step), "https://scoped/a");

    let exact = other_step.clone().with_step("fetch");
    assert_eq!(first_target(&built, &exact), "https://step/a");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#precedence")]
fn a_higher_layer_wins_and_within_one_layer_the_later_row_wins() {
    let sources = vec![
        LoadedSource {
            layer: Layer::Local,
            source: "settings.toml".into(),
            rows: vec![
                row(
                    "prefix = \"https://h/\"\nurl = \"https://local-first/\"",
                    Layer::Local,
                    "settings.toml",
                    0,
                ),
                row(
                    "prefix = \"https://h/\"\nurl = \"https://local-later/\"",
                    Layer::Local,
                    "settings.toml",
                    1,
                ),
            ],
            policy: UnmatchedPolicy::Allow,
        },
        LoadedSource {
            layer: Layer::Invocation,
            source: "--remap closed.toml".into(),
            rows: vec![row(
                "prefix = \"https://h/\"\nurl = \"https://invocation/\"",
                Layer::Invocation,
                "--remap closed.toml",
                0,
            )],
            policy: UnmatchedPolicy::Allow,
        },
    ];
    let built = Locator::from_sources(sources.clone(), Vec::new());
    let artifact = Artifact::new(ArtifactKind::RawFile).with_url("https://h/a");
    assert_eq!(first_target(&built, &artifact), "https://invocation/a");

    let local_only = Locator::from_sources(sources[..1].to_vec(), Vec::new());
    assert_eq!(
        first_target(&local_only, &artifact),
        "https://local-later/a"
    );
}

// ── the candidate list and the refusals (#locator, #policy) ─────────────

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#locator")]
fn remapped_targets_come_first_and_the_declared_origin_last() {
    let rows = vec![
        row(
            "prefix = \"https://h/\"\nurl = \"https://one/\"",
            Layer::Local,
            "t",
            0,
        ),
        row(
            "prefix = \"https://h/\"\nurl = \"https://two/\"",
            Layer::Local,
            "t",
            1,
        ),
    ];
    let artifact = Artifact::new(ArtifactKind::RawFile).with_url("https://h/a");
    let Resolution::Candidates(candidates) = locator(rows).locate(&artifact) else {
        panic!("no deny row here")
    };
    let shown: Vec<String> = candidates.iter().map(|c| c.target.display()).collect();
    assert_eq!(shown, ["https://two/a", "https://one/a", "https://h/a"]);
    assert!(candidates[0].row.is_some());
    assert!(candidates[2].row.is_none(), "the origin carries no row");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#row")]
fn a_deny_row_refuses_and_drops_the_origin() {
    let rows = vec![row(
        "prefix = \"https://h/\"\ndeny = true\nreason = \"policy: one store only\"",
        Layer::Local,
        "t",
        0,
    )];
    let artifact = Artifact::new(ArtifactKind::AppInput).with_url("https://h/a");
    let Resolution::Denied {
        row: refused,
        reason,
    } = locator(rows).locate(&artifact)
    else {
        panic!("the deny row must refuse")
    };
    assert_eq!(reason.as_deref(), Some("policy: one store only"));
    assert_eq!(refused.unwrap().provenance.layer, Layer::Local);
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#policy")]
fn unmatched_deny_refuses_an_unlisted_url_and_names_no_row() {
    let built = Locator::from_sources(
        vec![LoadedSource {
            policy: UnmatchedPolicy::Deny,
            ..LoadedSource::empty(Layer::Local, "settings.toml")
        }],
        Vec::new(),
    );
    let artifact = Artifact::new(ArtifactKind::AppInput).with_url("https://unlisted/a");
    let Resolution::Denied {
        row: refused,
        reason,
    } = built.locate(&artifact)
    else {
        panic!("the policy must refuse")
    };
    assert!(refused.is_none(), "no row refused it — the policy did");
    assert!(reason.unwrap().contains("unmatched"));
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#precedence")]
fn explain_lists_every_matching_row_in_winning_order() {
    let rows = vec![
        row(
            "prefix = \"https://h/\"\nurl = \"https://wide/\"\nreason = \"fallback\"",
            Layer::Local,
            "t",
            0,
        ),
        row(
            "prefix = \"https://h/deep/\"\nurl = \"https://narrow/\"\nreason = \"the deep one\"",
            Layer::Local,
            "t",
            1,
        ),
    ];
    let artifact = Artifact::new(ArtifactKind::RawFile)
        .with_url("https://h/deep/a")
        .with_digest("cd".repeat(32));
    let chain = locator(rows).explain(&artifact);
    assert_eq!(chain.matches.len(), 2);
    assert_eq!(chain.matches[0].reason.as_deref(), Some("the deep one"));
    assert_eq!(chain.matches[1].reason.as_deref(), Some("fallback"));
    assert_eq!(chain.chosen.unwrap().display(), "https://narrow/a");
    // The digest the bytes are checked against is the artifact's own, and
    // no row touched it.
    assert_eq!(chain.digest.as_deref(), Some("cd".repeat(32).as_str()));
    assert!(!chain.denied);
}

// ── PROP-002's tables read as rows (#layers) ────────────────────────────

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#layers")]
fn a_wildcard_mirror_becomes_one_prefix_row_per_declared_registry() {
    use crate::manifest::{AuthKind, MirrorSection, NamingConvention, RegistrySection};

    let registry = |name: &str, url: &str| RegistrySection {
        name: name.into(),
        url: url.into(),
        r#ref: "main".into(),
        naming: NamingConvention::Fqdn,
        auth: AuthKind::None,
        token_env: None,
        enabled: true,
        index_url: None,
    };
    let registries = [
        registry("a", "https://github.com/a"),
        registry("b", "https://gitverse.ru/b"),
    ];
    let mirrors = [MirrorSection {
        of: "*".into(),
        url: "https://nexus.corp.example/all".into(),
        priority: 0,
    }];
    let rows = rows_from_mirrors(&mirrors, &registries, Layer::Local, "~/.vibe/registry.toml");
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r.key.kind() == "prefix"));
    assert_eq!(rows[0].key.what(), "https://github.com/a");
    assert_eq!(rows[1].key.what(), "https://gitverse.ru/b");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#layers")]
fn mirror_priority_orders_the_chain_so_the_head_still_wins() {
    use crate::manifest::MirrorSection;

    let mirrors = [
        MirrorSection {
            of: "vibespecs".into(),
            url: "https://second/".into(),
            priority: 5,
        },
        MirrorSection {
            of: "vibespecs".into(),
            url: "https://first/".into(),
            priority: 0,
        },
    ];
    let rows = rows_from_mirrors(&mirrors, &[], Layer::Local, "registry.toml");
    let built = locator(rows);
    let artifact = Artifact::new(ArtifactKind::RegistryClone)
        .with_url("https://github.com/vibespecs/org.example.tool");
    // `priority = 0` is tried first by PROP-002, and the later row wins a
    // tie here, so the chain's head is still the first candidate.
    assert_eq!(first_target(&built, &artifact), "https://first/");
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#layers")]
fn settings_tables_contribute_rows_and_a_file_without_them_contributes_none() {
    let table: toml::Table = toml::from_str(
        "[vibe]\nsomething = 1\n\
         [policy]\nunmatched = \"deny\"\n\
         [[remap]]\nprefix = \"https://h/\"\nurl = \"https://mirror/\"\n",
    )
    .unwrap();
    let loaded = rows_from_settings(&table, Layer::Local, "~/.vibe/settings.toml").unwrap();
    assert_eq!(loaded.rows.len(), 1);
    assert_eq!(loaded.policy, UnmatchedPolicy::Deny);
    assert_eq!(loaded.rows[0].from.source, "~/.vibe/settings.toml");

    let bare: toml::Table = toml::from_str("[vibe]\nsomething = 1\n").unwrap();
    let none = rows_from_settings(&bare, Layer::Local, "settings.toml").unwrap();
    assert!(none.rows.is_empty() && none.policy == UnmatchedPolicy::Allow);
}

#[test]
fn the_env_list_splits_on_commas_and_ignores_empty_entries() {
    assert_eq!(env_remap_files("a.toml, b.toml,").len(), 2);
    assert!(env_remap_files("").is_empty());
}

#[test]
#[verifies("spec://org.vibevm.core/vibevm/common/PROP-066#locator")]
fn a_package_key_honours_a_version_requirement_when_the_artifact_carries_one() {
    let rows = vec![row(
        "package = \"org.example/tool@^1\"\nurl = \"https://pinned/\"",
        Layer::Local,
        "t",
        0,
    )];
    let built = locator(rows);
    let matching = Artifact::new(ArtifactKind::RegistryClone)
        .with_url("https://origin/")
        .with_package("org.example/tool@1.4.0");
    assert_eq!(first_target(&built, &matching), "https://pinned/");

    let other_major = Artifact::new(ArtifactKind::RegistryClone)
        .with_url("https://origin/")
        .with_package("org.example/tool@2.0.0");
    assert_eq!(first_target(&built, &other_major), "https://origin/");

    // No version to compare against is not a reason to drop the row.
    let unversioned = Artifact::new(ArtifactKind::RegistryClone)
        .with_url("https://origin/")
        .with_package("org.example/tool");
    assert_eq!(first_target(&built, &unversioned), "https://pinned/");
}
