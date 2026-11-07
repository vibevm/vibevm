//! Declared own boot reaches real install and preserves original inputs.
specmark::scope!(
    "spec://org.vibevm.core/vibevm/common/PROP-072#SOURCE-HOST-BOOT",
    r = 1
);
use super::super::test_helpers::{boot_rel, write};
use super::*;

fn fixture(grouped: bool) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let group = if grouped { "group='org.host'\n" } else { "" };
    let boot = path_to_slash(&layout::current_boot_dir());
    write(
        root.path(),
        "vibe.toml",
        &format!(
            "[project]\nname='host'\nversion='1.0.0'\n{group}[[project.sources]]\ndirectory='{boot}'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n[[project.sources]]\nfile='{boot}/10-native.payload'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n[[project.sources]]\nfile='{boot}/10-native.xml'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='documentation'\nloading='scan'\n[[project.sources]]\nfile='{boot}/explicit.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n[[project.sources]]\nfile='outside/missing.xml'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n"
        ),
    );
    write(
        root.path(),
        boot_rel("10-native.payload"),
        "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">Native</title></spec>",
    );
    write(
        root.path(),
        boot_rel("20-native.md"),
        "# Markdown {#root}\n",
    );
    write(
        root.path(),
        boot_rel("10-native.xml"),
        "#use invalid\n#embed invalid\n",
    );
    write(root.path(), boot_rel("explicit.md"), "# Explicit {#root}\n");
    root
}
#[test]
fn grouped_and_ungrouped_install_enroll_only_immediate_native_scan() {
    for grouped in [false, true] {
        let root = fixture(grouped);
        let workspace = Workspace::load(root.path()).unwrap();
        let original = fs::read(root.path().join(boot_rel("10-native.payload"))).unwrap();
        super::super::apply_resolution_with_spec_format(
            &workspace,
            &[],
            vibe_core::user_config::SlotIntegrity::Verify,
            SpecFormat::Xml,
            None,
            None,
        )
        .unwrap();
        let index: toml::Value = toml::from_str(
            &fs::read_to_string(root.path().join(layout::current_boot_index())).unwrap(),
        )
        .unwrap();
        let entries = index["entry"].as_array().unwrap();
        assert_eq!(
            entries
                .iter()
                .map(|e| e["path"].as_str().unwrap())
                .collect::<Vec<_>>(),
            vec![boot_rel("10-native.payload"), boot_rel("20-native.md")]
        );
        assert!(entries.iter().all(|e| e["kind"].as_str() == Some("static")));
        assert!(index.get("static").is_none());
        assert_eq!(
            original,
            fs::read(root.path().join(boot_rel("10-native.payload"))).unwrap()
        );
        regenerate_boot_from_with_spec_format(&workspace, &[], SpecFormat::Mixed).unwrap();
    }
}

#[test]
fn root_member_order_categories_inheritance_and_empty_controls() {
    let root = fixture(false);
    let manifest = root.path().join("vibe.toml");
    fs::write(
        &manifest,
        fs::read_to_string(&manifest).unwrap() + "\n[workspace]\nmembers=['member','empty']\n",
    )
    .unwrap();
    let boot = path_to_slash(&layout::current_boot_dir());
    write(
        root.path(),
        "member/vibe.toml",
        &format!(
            "[project]\nname='member'\nversion='1.0.0'\n[[project.sources]]\ndirectory='{boot}'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n"
        ),
    );
    write(
        root.path(),
        "empty/vibe.toml",
        "[project]\nname='empty'\nversion='1.0.0'\nsources=[]\n",
    );
    for (prefix, name) in [
        ("", "00-core.md"),
        ("", "90-user.md"),
        ("member/", "00-core.md"),
        ("member/", "90-user.md"),
        ("member/", "30-own.md"),
    ] {
        write(
            root.path(),
            format!("{prefix}{}", boot_rel(name)),
            "# Own {#root}\n",
        );
    }
    let ws = Workspace::load(root.path()).unwrap();
    regenerate_boot_from(&ws, &[]).unwrap();
    let index: toml::Value = toml::from_str(
        &fs::read_to_string(
            root.path()
                .join("member")
                .join(layout::current_boot_index()),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        index["entry"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["path"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>(),
        vec![
            boot_rel("00-core.md"),
            format!("member/{}", boot_rel("00-core.md")),
            format!("member/{}", boot_rel("30-own.md")),
            format!("member/{}", boot_rel("90-user.md"))
        ]
    );
    assert!(
        !root
            .path()
            .join("member")
            .join(layout::current_boot_static_md())
            .exists()
    );
    let empty =
        fs::read_to_string(root.path().join("empty").join(layout::current_boot_index())).unwrap();
    assert!(empty.contains(&boot_rel("00-core.md")));
    assert!(!empty.contains("90-user"));
}

#[test]
fn later_selected_invalid_missing_and_ambiguous_nodes_refuse_before_package_effects() {
    for failure in ["invalid", "missing", "ambiguous"] {
        let root = fixture(true);
        let manifest = root.path().join("vibe.toml");
        fs::write(
            &manifest,
            fs::read_to_string(&manifest).unwrap() + "\n[workspace]\nmembers=['later']\n",
        )
        .unwrap();
        let selected = boot_rel("bad.xml");
        write(
            root.path(),
            "later/vibe.toml",
            &format!(
                "[project]\nname='later'\nversion='1.0.0'\n[[project.sources]]\nfile='{selected}'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n"
            ),
        );
        if failure != "missing" {
            write(root.path(), format!("later/{selected}"), "<spec");
        }
        if failure == "ambiguous" {
            let path = root.path().join("later/vibe.toml");
            fs::write(&path,fs::read_to_string(&path).unwrap()+&format!("\n[[project.sources]]\nfile='{selected}'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='scan'\n")).unwrap();
        }
        let (dep, _source) = super::super::test_helpers::dep_with_boot(
            "earlier",
            "1.0.0",
            "[boot_snippet]\nsource='boot.md'\n",
            "boot.md",
            "# Earlier\n",
        );
        let ws = Workspace::load(root.path()).unwrap();
        let before = fs::read(&manifest).unwrap();
        assert!(
            super::super::apply_resolution_with_spec_format(
                &ws,
                &[dep],
                vibe_core::user_config::SlotIntegrity::Verify,
                SpecFormat::Mixed,
                None,
                None
            )
            .is_err(),
            "{failure}"
        );
        assert_eq!(fs::read(&manifest).unwrap(), before);
        assert!(!root.path().join(layout::current_boot_index()).exists());
        assert!(!root.path().join("vibe.lock").exists());
        assert!(!root.path().join(layout::current_vibedeps_root()).exists());
    }
}

#[test]
fn retained_zero_static_publication_refuses_changed_selection() {
    for change in ["bytes", "membership", "media", "parent", "manifest", "role"] {
        let root = fixture(true);
        let ws = Workspace::load(root.path()).unwrap();
        let sources = boot_artifacts::declared_sources::DeclaredBootSources::prepare_with_nodes(
            &ws,
            &[],
            Some(SpecFormat::Mixed),
        )
        .unwrap()
        .unwrap();
        let own = &sources.node(".").unwrap().entries;
        let effective = boot::compute_effective_boot(NodeBootInputs {
            own_boot: own,
            inherited_foundation: &[],
            dependencies: &[],
            default_link: ws.root_manifest.boot.default_link,
        })
        .unwrap();
        let world =
            crate::extension_world::ExtensionWorldEpoch::from_resolution(root.path(), &[]).unwrap();
        let epoch = crate::extension_world::lower_owner_runtimes(
            &ws,
            &world,
            crate::extension_world::OwnerRuntimeLowering::compatibility_root_without_presets(),
        )
        .unwrap()
        .bind_run(crate::extension_world::OwnerRuntimeRunFacts {
            run_id: "0123456789abcdef0123456789abcdef".into(),
            state_root: root.path().join(".vibe"),
            platform: "linux-x86_64".into(),
            offline: true,
            created_at: "2026-11-07T00:00:00Z".into(),
        });
        match change {
            "bytes" => write(root.path(), boot_rel("20-native.md"), "# changed\n"),
            "membership" => write(root.path(), boot_rel("30-new.md"), "# new\n"),
            "parent" => {
                let boot = root.path().join(layout::current_boot_dir());
                fs::rename(&boot, root.path().join("old-boot")).unwrap();
                fs::create_dir(&boot).unwrap();
                for file in fs::read_dir(root.path().join("old-boot")).unwrap() {
                    let file = file.unwrap();
                    fs::hard_link(file.path(), boot.join(file.file_name())).unwrap();
                }
            }
            "role" => write(root.path(), layout::SLOT_RECORD_FILENAME, "not parsed"),
            _ => {
                let path = root.path().join("vibe.toml");
                let text = fs::read_to_string(&path).unwrap();
                fs::write(
                    path,
                    if change == "media" {
                        text.replace("application/xml", "text/markdown")
                    } else {
                        text + "\n# changed\n"
                    },
                )
                .unwrap();
            }
        }
        let error = boot_artifacts::write_boot_artifacts_traced(
            root.path(),
            ".",
            root.path(),
            &root_self_coordinate(&ws.root_manifest),
            &effective,
            SpecFormat::Mixed,
            Some(&sources),
            None,
            vibe_spec::CompilePlans::transforms_only(vibe_spec::TransformPlan::empty()),
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("SOURCE-HOST-BOOT"),
            "{change}: {error}"
        );
        let managed =
            boot_artifacts::native_managed::write_boot_artifacts_owner_managed_with_sources::<
                crate::boot_artifacts::native_managed_tests::FakeProvider,
            >(
                root.path(),
                ".",
                root.path(),
                &root_self_coordinate(&ws.root_manifest),
                &effective,
                SpecFormat::Mixed,
                Some(&sources),
                None,
                epoch.node(".").unwrap(),
                None,
            );
        assert!(
            managed.is_err(),
            "zero-static managed writer must recheck {change}"
        );
        assert!(!root.path().join(layout::current_boot_index()).exists());
        assert!(!root.path().join("CLAUDE.md").exists());
    }
}

#[test]
fn explicit_static_consumer_uses_captured_media_without_changing_automatic_linkage() {
    for grouped in [false, true] {
        let root = fixture(grouped);
        let ws = Workspace::load(root.path()).unwrap();
        let sources = boot_artifacts::declared_sources::DeclaredBootSources::prepare_with_nodes(
            &ws,
            &[],
            Some(SpecFormat::Mixed),
        )
        .unwrap()
        .unwrap();
        let mut effective = boot::compute_effective_boot(NodeBootInputs {
            own_boot: &sources.node(".").unwrap().entries,
            inherited_foundation: &[],
            dependencies: &[],
            default_link: Some(LinkType::Static),
        })
        .unwrap();
        assert!(
            effective.static_entries().next().is_none(),
            "automatic authored inputs remain Dynamic"
        );
        // Exercise the existing artifact consumer explicitly; this is not an
        // assertion that automatic own boot is compiled into STATIC.
        for entry in &mut effective.entries {
            entry.link = LinkType::Static;
        }
        let text = boot_artifacts::render_static_observed(
            &effective,
            root.path(),
            &root_self_coordinate(&ws.root_manifest),
            SpecFormat::Mixed,
            Some(&sources),
            None,
            vibe_spec::CompilePlans::transforms_only(vibe_spec::TransformPlan::empty()),
        )
        .unwrap()
        .unwrap();
        assert!(
            text.contains("Native") && text.contains("Markdown"),
            "{text}"
        );
        assert!(
            !text.contains("<spec") && !text.contains("invalid"),
            "{text}"
        );
        assert!(
            boot_artifacts::render_static(
                &effective,
                root.path(),
                &root_self_coordinate(&ws.root_manifest)
            )
            .is_err(),
            "unbound entry cannot regain suffix semantics"
        );
    }
}

#[test]
fn empty_declared_scope_can_be_created_by_generated_output_and_legacy_is_preserved() {
    for rows in ["sources=[]\n", ""] {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "vibe.toml",
            &format!(
                "[project]\nname='root'\nversion='1.0.0'\n{rows}[workspace]\nmembers=['member']\n"
            ),
        );
        write(
            root.path(),
            "member/vibe.toml",
            "[project]\nname='member'\nversion='1.0.0'\nsources=[]\n",
        );
        let ws = Workspace::load(root.path()).unwrap();
        regenerate_boot_from(&ws, &[]).unwrap();
        regenerate_boot_from(&ws, &[]).unwrap();
        assert!(root.path().join(layout::current_boot_index()).is_file());
        assert!(
            root.path()
                .join("member")
                .join(layout::current_boot_index())
                .is_file()
        );
    }
}

#[test]
fn explicit_only_host_point_read_remains_available_but_not_auto_enrolled() {
    use vibe_spec::SectionSource;
    let root = fixture(true);
    let ws = Workspace::load(root.path()).unwrap();
    regenerate_boot_from(&ws, &[]).unwrap();
    let source = vibe_spec::FsSectionSource::new(vibe_spec::FileResolver::new(
        root.path(),
        root_self_coordinate(&ws.root_manifest),
    ));
    let address = vibe_spec::SpecAddress::parse("spec://org.host/host/boot/explicit#root").unwrap();
    assert!(source.section_text(&address).unwrap().contains("Explicit"));
    assert!(
        !fs::read_to_string(root.path().join(layout::current_boot_index()))
            .unwrap()
            .contains("explicit.md")
    );
}
