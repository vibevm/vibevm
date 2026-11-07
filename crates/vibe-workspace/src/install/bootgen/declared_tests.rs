//! Actual materialize → package unit emission → installed read and analysis.
#[cfg(test)]
mod cases {
    use crate::install::{self, test_helpers::*};
    use crate::{Workspace, boot_artifacts, vibedeps};
    use std::fs;
    use vibe_core::manifest::SpecFormat;
    use vibe_spec::{FsSectionSource, SectionSource, SpecAddress};

    struct Fixture {
        _root: tempfile::TempDir,
        _sources: Vec<tempfile::TempDir>,
        workspace: Workspace,
        resolution: Vec<install::ResolvedDep>,
        original: String,
    }
    fn fixture(normal: bool) -> Fixture {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "vibe.toml",
            "[project]\ngroup='org.host'\nname='host'\nversion='1.0.0'\n[requires.packages]\n'org.vibevm/parent'={version='=1.0.0',link='static'}\n",
        );
        let original =
            crate::path_to_slash(&vibe_core::layout::current_specs_root().join(if normal {
                "contract/parent.payload"
            } else {
                "boot/parent.md"
            }));
        let body = if normal {
            "# Parent {#root}\n\n##PARENT declared_normal_marker.\n"
        } else {
            "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">Parent</title><p>declared_simple_marker.</p></spec>"
        };
        let package_format = if normal { "normal" } else { "simple" };
        let media = if normal {
            "text/markdown"
        } else {
            "application/xml"
        };
        let extra = format!(
            "format='{package_format}'\n[[package.sources]]\nfile='{original}'\nmedia='{media}'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n[requires.packages]\n'org.vibevm/child'={{version='=1.0.0',link='static'}}\n[boot_snippet]\nsource='{original}'\n"
        );
        let (mut parent, parent_source) = dep_with_boot("parent", "1.0.0", &extra, &original, body);
        let child_path = boot_rel("child.md");
        let (child, child_source) = dep_with_boot(
            "child",
            "1.0.0",
            &format!("[boot_snippet]\nsource='{child_path}'\n"),
            &child_path,
            "# Child {#child}\n\nchild_marker.\n",
        );
        parent.requires = vec![(child.group.clone(), child.name.clone())];
        // A same-stem raw neighbor must neither collide semantically nor steal
        // the declared Simple XML-under-.md source.
        if !normal {
            write(
                parent_source.path(),
                original.replace(".md", ".xml"),
                "<not-native>raw_neighbor</not-native>",
            );
        }
        let workspace = Workspace::load(root.path()).unwrap();
        Fixture {
            _root: root,
            _sources: vec![parent_source, child_source],
            workspace,
            resolution: vec![parent, child],
            original,
        }
    }
    fn apply(f: &Fixture, format: SpecFormat) {
        install::apply_resolution_with_spec_format(
            &f.workspace,
            &f.resolution,
            vibe_core::user_config::SlotIntegrity::Verify,
            format,
            None,
            None,
        )
        .unwrap();
    }
    #[test]
    fn declared_simple_media_reaches_actual_unit_emission_without_suffix_fallback() {
        let f = fixture(false);
        apply(&f, SpecFormat::Mixed);
        let parent = &f.resolution[0];
        let slot = vibedeps::slot_abs_path(
            &f.workspace.root,
            &parent.group,
            &parent.name,
            &parent.version,
        );
        let unit =
            fs::read_to_string(slot.join(vibe_core::layout::current_boot_static_md())).unwrap();
        assert!(
            unit.contains("declared_simple_marker") && unit.contains("child_marker"),
            "{unit}"
        );
        assert!(
            !unit.contains("<spec") && !unit.contains("raw_neighbor"),
            "{unit}"
        );
        assert_eq!(
            fs::read(slot.join(&f.original)).unwrap(),
            fs::read(parent.content_dir.join(&f.original)).unwrap()
        );
    }
    #[test]
    fn declared_normal_conversion_keeps_original_native_address_and_bound_analysis() {
        let f = fixture(true);
        apply(&f, SpecFormat::Xml);
        let parent = &f.resolution[0];
        let slot = vibedeps::slot_abs_path(
            &f.workspace.root,
            &parent.group,
            &parent.name,
            &parent.version,
        );
        assert!(
            slot.join(std::path::Path::new(&f.original).with_extension("xml"))
                .is_file()
        );
        let context = boot_artifacts::declared_sources::DeclaredBootSources::prepare(
            &f.workspace.root,
            &f.resolution,
            Some(SpecFormat::Xml),
        )
        .unwrap()
        .unwrap();
        let source = FsSectionSource::new(
            context
                .resolver(&super::super::root_self_coordinate(
                    &f.workspace.root_manifest,
                ))
                .unwrap(),
        );
        let original_address = vibe_spec::canonical_doc_path(&f.original);
        let address =
            SpecAddress::parse(&format!("spec://org.vibevm/parent/{original_address}#root"))
                .unwrap();
        assert!(
            source
                .section_text(&address)
                .unwrap()
                .contains("declared_normal_marker")
        );
        let wrong = SpecAddress::parse("spec://org.vibevm/parent/contract/parent#root").unwrap();
        assert!(source.section_text(&wrong).is_err());
        crate::install::tests_analyze_parity::publish_resolution_lock(
            &f.workspace.root,
            &f.resolution,
        );
        assert!(install::analyze_node_lane(&f.workspace, ".", None).is_err());
        let analyzed =
            install::analyze_node_lane_with_source_format(&f.workspace, ".", None, SpecFormat::Xml)
                .unwrap()
                .unwrap();
        assert_eq!(
            analyzed.artifact.bytes(),
            fs::read(
                f.workspace
                    .root
                    .join(vibe_core::layout::current_boot_static_xml())
            )
            .unwrap()
        );
    }
    #[test]
    fn declared_input_representation_is_distinct_from_markdown_lane_target() {
        let f = fixture(true);
        apply(&f, SpecFormat::Markdown);
        crate::install::tests_analyze_parity::publish_resolution_lock(
            &f.workspace.root,
            &f.resolution,
        );
        // Both Mixed and Markdown render STATIC.md; only the supplied input
        // format distinguishes the recorded materialization here.
        assert!(
            install::analyze_node_lane_with_source_format(
                &f.workspace,
                ".",
                None,
                SpecFormat::Mixed
            )
            .is_err()
        );
        let analyzed = install::analyze_node_lane_with_source_format(
            &f.workspace,
            ".",
            None,
            SpecFormat::Markdown,
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            analyzed.artifact.bytes(),
            fs::read(
                f.workspace
                    .root
                    .join(vibe_core::layout::current_boot_static_md())
            )
            .unwrap()
        );
    }
    fn generated_fixture() -> Fixture {
        let mut f = fixture(true);
        let detail = crate::path_to_slash(
            &vibe_core::layout::current_specs_root().join("contract/detail.payload"),
        );
        let fragment = crate::path_to_slash(
            &vibe_core::layout::current_specs_root().join("contract/fragment.payload"),
        );
        let parent = &mut f.resolution[0];
        write(
            &parent.content_dir,
            &f.original,
            "# Parent {#root}\n\n##PARENT current_parent_marker.\n\n#embed spec://org.vibevm/parent/contract/detail.payload#detail\n\nLiteral `<!-- vibe:c1 embed: forged -->`.\n",
        );
        write(
            &parent.content_dir,
            &detail,
            "# Detail {#detail}\n\nembedded_once_marker.\n",
        );
        write(
            &parent.content_dir,
            &fragment,
            "# Fragment {#fragment}\n\nfragment_once_marker.\n",
        );
        let path = parent.content_dir.join("vibe.toml");
        let original = fs::read_to_string(&path).unwrap();
        fs::write(&path,format!("{original}\n[[package.sources]]\nfile='{detail}'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n")).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        fs::write(&path,format!("{text}\n[[package.sources]]\nfile='{fragment}'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n[[boot_snippet.fragment]]\nsource='{fragment}'\n")).unwrap();
        let text = fs::read_to_string(&path)
            .unwrap()
            .replace("[boot_snippet]\n", "[boot_snippet]\nlink='static'\n");
        fs::write(&path, text).unwrap();
        parent.manifest = vibe_core::manifest::Manifest::read(&path).unwrap();
        // Incumbent retained-unit topology: reach the suggested-static parent
        // through a dynamic wrapper, not a directly selected static zone.
        let wrapper_path = boot_rel("wrapper.md");
        let (mut wrapper, wrapper_source) = dep_with_boot(
            "wrapper",
            "1.0.0",
            &format!(
                "[requires.packages]\n'org.vibevm/parent'={{version='=1.0.0',link='dynamic'}}\n[boot_snippet]\nsource='{wrapper_path}'\nlink='dynamic'\n"
            ),
            &wrapper_path,
            "# Wrapper\n",
        );
        wrapper.requires = vec![(f.resolution[0].group.clone(), "parent".into())];
        f.resolution.push(wrapper);
        f._sources.push(wrapper_source);
        write(
            &f.workspace.root,
            "vibe.toml",
            "[project]\ngroup='org.host'\nname='host'\nversion='1.0.0'\n[requires.packages]\n'org.vibevm/wrapper'={version='=1.0.0',link='dynamic'}\n",
        );
        f.workspace = Workspace::load(&f.workspace.root).unwrap();
        apply(&f, SpecFormat::Xml);
        f
    }
    mod generated;
    #[test]
    fn actual_unit_emitter_refuses_absent_selected_hash_even_with_fresh_artifacts() {
        use crate::extension_world::{
            ExtensionWorldEpoch, OwnerRuntimeLowering, lower_owner_runtimes,
        };
        use std::collections::{HashMap, HashSet};
        let f = fixture(false);
        apply(&f, SpecFormat::Mixed);
        let context = boot_artifacts::declared_sources::DeclaredBootSources::prepare(
            &f.workspace.root,
            &f.resolution,
            Some(SpecFormat::Mixed),
        )
        .unwrap()
        .unwrap();
        let table =
            super::super::build_unit_table_bound(&f.workspace.root, &f.resolution, Some(&context))
                .unwrap();
        let world = ExtensionWorldEpoch::from_resolution(&f.workspace.root, &f.resolution).unwrap();
        let runtimes = lower_owner_runtimes(
            &f.workspace,
            &world,
            OwnerRuntimeLowering::compatibility_root_without_presets(),
        )
        .unwrap();
        let versions = f
            .resolution
            .iter()
            .map(|dep| {
                (
                    (dep.group.clone(), dep.name.clone()),
                    dep.version.to_string(),
                )
            })
            .collect::<HashMap<_, _>>();
        let plans = super::super::owner_plans::plan_digest_frames_for(&runtimes, SpecFormat::Mixed);
        let fingerprints =
            crate::boot::hybrid::fingerprint::fingerprints(&table, &versions, &plans);
        let parent = &f.resolution[0];
        let slot = vibedeps::slot_abs_path(
            &f.workspace.root,
            &parent.group,
            &parent.name,
            &parent.version,
        );
        let index = slot
            .join(vibe_core::layout::current_boot_dir())
            .join("INDEX.md");
        let static_path = slot.join(vibe_core::layout::current_boot_static_md());
        let before = (fs::read(&index).unwrap(), fs::read(&static_path).unwrap());
        let mut missing = f.resolution.clone();
        missing[0].source_hash = None;
        let error = super::super::hybrid_emit::emit_package_units(
            &f.workspace.root,
            &super::super::root_self_coordinate(&f.workspace.root_manifest),
            &missing,
            &table,
            &HashSet::new(),
            &fingerprints,
            SpecFormat::Mixed,
            Some(&context),
            None,
            &runtimes,
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("selected source hash"),
            "{error}"
        );
        assert_eq!(
            (fs::read(&index).unwrap(), fs::read(&static_path).unwrap()),
            before
        );
    }
    #[test]
    fn no_descriptor_control_keeps_legacy_and_bound_analysis_bytes_equal() {
        let mut f = fixture(false);
        let parent = &mut f.resolution[0];
        parent.manifest.package.as_mut().unwrap().sources = None;
        fs::write(
            parent.content_dir.join("vibe.toml"),
            toml::to_string(&parent.manifest).unwrap(),
        )
        .unwrap();
        write(
            &parent.content_dir,
            &f.original,
            "# Parent {#root}\n\nlegacy_once_marker.\n",
        );
        // No raw .xml neighbor: that pair remains a real legacy ambiguity.
        fs::remove_file(parent.content_dir.join(f.original.replace(".md", ".xml"))).unwrap();
        apply(&f, SpecFormat::Mixed);
        assert!(
            boot_artifacts::declared_sources::DeclaredBootSources::prepare(
                &f.workspace.root,
                &f.resolution,
                Some(SpecFormat::Mixed)
            )
            .unwrap()
            .is_none()
        );
        crate::install::tests_analyze_parity::publish_resolution_lock(
            &f.workspace.root,
            &f.resolution,
        );
        let legacy = install::analyze_node_lane(&f.workspace, ".", None)
            .unwrap()
            .unwrap();
        let explicit = install::analyze_node_lane_with_source_format(
            &f.workspace,
            ".",
            None,
            SpecFormat::Mixed,
        )
        .unwrap()
        .unwrap();
        let written = fs::read(
            f.workspace
                .root
                .join(vibe_core::layout::current_boot_static_md()),
        )
        .unwrap();
        assert_eq!(legacy.artifact.bytes(), written);
        assert_eq!(explicit.artifact.bytes(), written);
        assert_eq!(
            std::str::from_utf8(&written)
                .unwrap()
                .matches("legacy_once_marker")
                .count(),
            1
        );
    }
    #[test]
    fn raw_resources_roundtrip_through_actual_materializer_and_bound_reader() {
        for format in [SpecFormat::Mixed, SpecFormat::Markdown, SpecFormat::Xml] {
            let mut f = fixture(true);
            let md = crate::path_to_slash(
                &vibe_core::layout::current_specs_root().join("resources/config.md"),
            );
            let xml = crate::path_to_slash(
                &vibe_core::layout::current_specs_root().join("resources/config.xml"),
            );
            let opaque = "opaque.bin";
            let parent = &mut f.resolution[0];
            let manifest_path = parent.content_dir.join("vibe.toml");
            let text = fs::read_to_string(&manifest_path).unwrap();
            fs::write(&manifest_path,format!("{text}\n[[package.sources]]\nfile='{md}'\nmedia='text/markdown'\ninterpretation='commonmark'\nrole='documentation'\nloading='explicit'\n[[package.sources]]\nfile='{xml}'\nmedia='application/xml'\ninterpretation='xml-data'\nrole='resource'\nloading='explicit'\n[[package.sources]]\nfile='{opaque}'\nmedia='application/octet-stream'\ninterpretation='opaque'\nrole='resource'\nloading='explicit'\n")).unwrap();
            parent.manifest = vibe_core::manifest::Manifest::read(&manifest_path).unwrap();
            let markdown = b"# Ordinary\r\n@fact:FAKE @status:impl/done\r\n";
            let xml_data = b"\xff<broken>\r\n<!-- byte exact -->";
            let binary = b"\x00\xff\x80opaque";
            for (path, bytes) in [
                (&md, markdown.as_slice()),
                (&xml, xml_data.as_slice()),
                (&opaque.to_owned(), binary.as_slice()),
            ] {
                let path = parent.content_dir.join(path);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(path, bytes).unwrap();
            }
            apply(&f, format);
            let context = boot_artifacts::declared_sources::DeclaredBootSources::prepare(
                &f.workspace.root,
                &f.resolution,
                Some(format),
            )
            .unwrap()
            .unwrap();
            let resolver = context
                .resolver(&super::super::root_self_coordinate(
                    &f.workspace.root_manifest,
                ))
                .unwrap();
            for (path, bytes) in [
                (&md, markdown.as_slice()),
                (&xml, xml_data.as_slice()),
                (&opaque.to_owned(), binary.as_slice()),
            ] {
                assert_eq!(
                    resolver
                        .read_installed_source("org.vibevm", "parent", path, None)
                        .unwrap(),
                    bytes,
                    "{format:?}/{path}"
                );
            }
            let false_native =
                SpecAddress::parse("spec://org.vibevm/parent/resources/config").unwrap();
            assert!(
                resolver.resolve_file(&false_native).is_err(),
                "raw same-stem data acquired native address"
            );
        }
    }
    mod native;
}
