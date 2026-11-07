//! Actual publication boundaries refuse retained-input drift before writes.
use super::*;

#[test]
fn captured_source_scope_and_policy_drift_precede_each_first_write() {
    for change in ["bytes", "identity", "policy", "scope", "membership"] {
        for effect in ["refresh", "mirror", "resume", "baseline", "weave"] {
            let tmp = tempfile::tempdir().unwrap();
            let root = tmp.path();
            std::fs::write(root.join("vibe.toml"), "[project]\nname='host'\nversion='1.0.0'\n[[project.sources]]\ndirectory='corpus'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n").unwrap();
            std::fs::create_dir(root.join("corpus")).unwrap();
            std::fs::write(
                root.join("corpus/source.md"),
                "# Native\n\n@fact:A body @impl/done\n",
            )
            .unwrap();
            std::fs::write(
                root.join("progress.toml"),
                "include=['corpus/*.md']\n[progress]\ncache_dir='payload-cache'\n",
            )
            .unwrap();
            let campaign = root.join("campaigns/c");
            std::fs::create_dir_all(campaign.join("run")).unwrap();
            let mut g = ground(&common(root, Some(&campaign))).unwrap();
            match change {
                "bytes" => std::fs::write(root.join("corpus/source.md"), "changed").unwrap(),
                "identity" => {
                    std::fs::rename(root.join("corpus/source.md"), root.join("old.md")).unwrap();
                    std::fs::copy(root.join("old.md"), root.join("corpus/source.md")).unwrap();
                }
                "policy" => std::fs::write(
                    root.join("vibe.toml"),
                    "[project]\nname='host'\nversion='1.0.0'\nsources=[]\n",
                )
                .unwrap(),
                "scope" => {
                    std::fs::write(root.join("progress.toml"), "include=['corpus/absent.md']\n")
                        .unwrap()
                }
                _ => std::fs::write(root.join("corpus/added.md"), "# New").unwrap(),
            }
            let ctx = Context::from_flags(true, false, None, true, AgentModeArg::Auto);
            let result = match effect {
                "refresh" => refresh_state(&mut g).map(|_| ()),
                "mirror" => mirror_grounded(&ctx, &mut g),
                "resume" => resume_grounded(&ctx, &mut g),
                "baseline" => {
                    let projection =
                        progress_core::baseline::project::project(g.docs.iter(), &g.cache, "c");
                    baseline::store_projection(&g, &projection, &campaign.join("baseline.json"))
                        .map(|_| ())
                }
                _ => emit_weave(
                    &ctx,
                    &g,
                    &[(0, "body".into())],
                    Some(&root.join("shards")),
                    "weave",
                ),
            };
            assert!(result.is_err(), "{change}/{effect}");
            for absent in [
                "run/cache.json",
                "run/RESUME.md",
                "run/mirror",
                "baseline.json",
                "run/state",
            ] {
                assert!(
                    !campaign.join(absent).exists(),
                    "{change}/{effect}: {absent}"
                );
            }
            assert!(!root.join("payload-cache").exists());
            assert!(!root.join("shards").exists());
        }
    }
}

#[test]
fn changed_evidence_snapshot_refuses_before_cache_write() {
    for initially_present in [false, true] {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let campaign = root.join("campaigns/c");
        std::fs::create_dir_all(campaign.join("run")).unwrap();
        if initially_present {
            write_specmap(root, false);
        }
        let mut g = ground(&common(root, Some(&campaign))).unwrap();
        let evidence = EvidenceSnapshot::load(root).unwrap();
        write_specmap(root, true);
        assert!(refresh_state_with_evidence(&mut g, &evidence).is_err());
        assert!(!campaign.join("run/cache.json").exists());
    }
}

#[test]
fn installed_binding_drift_precedes_each_first_write() {
    use vibe_core::manifest::SpecFormat;
    for format in [SpecFormat::Mixed, SpecFormat::Markdown, SpecFormat::Xml] {
        for change in ["lock", "binding", "format"] {
            for effect in ["refresh", "mirror", "resume", "baseline", "weave"] {
                let tmp = tempfile::tempdir().unwrap();
                let root = tmp.path();
                let source = tempfile::tempdir().unwrap();
                std::fs::write(source.path().join("vibe.toml"), "[package]\nname='native'\ngroup='org.example'\nkind='flow'\nversion='1.0.0'\n[[package.sources]]\nfile='native.payload'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n").unwrap();
                std::fs::write(source.path().join("native.payload"), "<spec xmlns=\"https://vibevm.org/spec/1\"><p><A fact=\"true\" status=\"impl/done\">body</A></p></spec>").unwrap();
                let label = match format {
                    SpecFormat::Mixed => "mixed",
                    SpecFormat::Markdown => "markdown",
                    SpecFormat::Xml => "xml",
                };
                std::fs::write(
                    root.join("vibe.toml"),
                    format!("[project]\nname='host'\nversion='1.0.0'\nspec_format='{label}'\n"),
                )
                .unwrap();
                let group = "org.example".parse().unwrap();
                let version = "1.0.0".parse().unwrap();
                let hash =
                    vibe_core::ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
                vibe_workspace::vibedeps::materialise_with_spec_format(
                    root,
                    &group,
                    "native",
                    &version,
                    source.path(),
                    vibe_workspace::vibedeps::CopyMode::Copy,
                    format,
                    &hash,
                )
                .unwrap();
                std::fs::write(root.join("vibe.lock"), format!("[meta]\nschema_version={}\ngenerated_by='test'\ngenerated_at='2026-01-01T00:00:00Z'\n[[package]]\nkind='flow'\ngroup='org.example'\nname='native'\nversion='1.0.0'\nsource_url='https://example.invalid/native.git'\ncontent_hash='{hash}'\n", vibe_core::manifest::CURRENT_SCHEMA_VERSION)).unwrap();
                let slot = root.join(vibe_workspace::vibedeps::slot_rel_path(
                    &group, "native", &version,
                ));
                let physical = match format {
                    SpecFormat::Mixed => "native.payload",
                    SpecFormat::Markdown => "native.md",
                    SpecFormat::Xml => "native.xml",
                };
                std::fs::write(
                    slot.join("progress.toml"),
                    format!("include=['{physical}']\n[progress]\ncache_dir='payload-cache'\n"),
                )
                .unwrap();
                let campaign = root.join("campaigns/c");
                std::fs::create_dir_all(campaign.join("run")).unwrap();
                let mut g = ground(&common(&slot, Some(&campaign))).unwrap();
                match change {
                    "lock" => std::fs::write(root.join("vibe.lock"), "changed").unwrap(),
                    "binding" => std::fs::write(
                        slot.join(vibe_workspace::vibedeps::SLOT_RECORD_FILENAME),
                        "changed",
                    )
                    .unwrap(),
                    _ => std::fs::write(
                        root.join("vibe.toml"),
                        "[project]\nname='host'\nversion='1.0.0'\nspec_format='mixed'\n# changed\n",
                    )
                    .unwrap(),
                }
                let ctx = Context::from_flags(true, false, None, true, AgentModeArg::Auto);
                let result = match effect {
                    "refresh" => refresh_state(&mut g).map(|_| ()),
                    "mirror" => mirror_grounded(&ctx, &mut g),
                    "resume" => resume_grounded(&ctx, &mut g),
                    "baseline" => {
                        let p =
                            progress_core::baseline::project::project(g.docs.iter(), &g.cache, "c");
                        baseline::store_projection(&g, &p, &campaign.join("baseline.json"))
                            .map(|_| ())
                    }
                    _ => emit_weave(
                        &ctx,
                        &g,
                        &[(0, "body".into())],
                        Some(&root.join("shards")),
                        "weave",
                    ),
                };
                assert!(result.is_err(), "{label}/{change}/{effect}");
                assert_eq!(std::fs::read_dir(campaign.join("run")).unwrap().count(), 0);
                assert!(!campaign.join("baseline.json").exists());
                assert!(!slot.join("payload-cache").exists());
                assert!(!root.join("shards").exists());
            }
        }
    }
}
