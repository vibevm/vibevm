use super::*;

#[test]
fn selected_whole_section_table_and_outline_have_validated_shapes() {
    use vibe_core::manifest::SpecFormat;
    for format in [SpecFormat::Markdown, SpecFormat::Xml] {
        let (_root, host) = installed(format);
        for (address, mode, needle) in [
            ("spec://org.demo/dep/DOC", "read", "Unselected."),
            ("spec://org.demo/dep/DOC#root.solution", "read", "Selected."),
            ("spec://org.demo/dep/DOC#CC-A-4-1", "read", "First."),
            ("spec://org.demo/dep/DOC", "outline", ""),
        ] {
            let response = SourceQueryService::execute_json(
                &host,
                &json!({"schema":1,"address":address,"mode":mode,"format":"md","children":true}),
            );
            assert!(
                !response.is_error(),
                "{format:?}/{address}/{mode}: {response:?}"
            );
            let result = wire::read_result(&response.into_value()).unwrap();
            if mode == "outline" {
                assert!(result.content.is_empty());
                assert!(
                    result
                        .outline
                        .iter()
                        .any(|node| node.path.last().is_some_and(|id| id == "CC-A-4-1"))
                );
            } else {
                assert!(result.content.contains(needle));
            }
            if address.ends_with("CC-A-4-1") {
                assert!(!result.content.contains("Second."));
                assert_eq!(result.location.line_start, result.location.line_end);
            }
        }
        // Explicit evidence export preserves the very slots tested above for
        // subsequent real CLI/MCP process checks; ordinary tests clean up.
        if std::env::var("VIBE_D2_EXPORT_FIXTURES").as_deref() == Ok("1") {
            println!("D2_SURFACE_FIXTURE {format:?} {}", _root.keep().display());
        }
    }
}

#[test]
fn unknown_versions_documents_anchors_and_expectations_refuse_by_boundary() {
    let (_root, host) = installed(vibe_core::manifest::SpecFormat::Markdown);
    let base = json!({"schema":1,"address":"spec://org.demo/dep/DOC#ONE","mode":"read","format":"md","children":true});
    for (address, expected) in [
        (
            "spec://org.demo/unselected/DOC#ONE",
            ErrorCode::UnknownPackage,
        ),
        ("spec://org.demo/dep@2.0.0/DOC#ONE", ErrorCode::WrongVersion),
        (
            "spec://org.demo/dep/MISSING#ONE",
            ErrorCode::UnknownDocument,
        ),
        ("spec://org.demo/dep/DOC#MISSING", ErrorCode::UnknownAnchor),
    ] {
        let mut request = base.clone();
        request["address"] = json!(address);
        let response = SourceQueryService::execute_json(&host, &request);
        assert!(response.is_error());
        let error = wire::read_error(&response.into_value()).unwrap();
        assert_eq!(error.code, expected, "{address}: {error:?}");
        assert!(!error.candidates.is_empty());
        if expected == ErrorCode::UnknownPackage {
            assert_eq!(error.candidates, ["org.demo/consumer", "org.demo/dep"]);
        }
        if expected == ErrorCode::UnknownDocument {
            assert!(
                error
                    .candidates
                    .iter()
                    .any(|candidate| candidate == "org.demo/dep/DOC")
            );
            assert!(
                !error
                    .candidates
                    .iter()
                    .any(|candidate| candidate.ends_with("/vibe"))
            );
        }
    }
    for (field, prefix, boundary) in [
        ("expected_document_hash", "sha256:", "document expectation"),
        ("expected_source_hash", "sha256:", "source expectation"),
        (
            "expected_resolution_hash",
            "sha256-query-resolution/1:",
            "resolution expectation",
        ),
    ] {
        let mut request = base.clone();
        request[field] = json!(format!("{prefix}{}", "b".repeat(64)));
        let error =
            wire::read_error(&SourceQueryService::execute_json(&host, &request).into_value())
                .unwrap();
        assert_eq!(error.code, ErrorCode::StaleExpectation);
        assert_eq!(error.boundary, boundary);
    }
    let mut request = base;
    request["root"] = json!("/foreign");
    let error = wire::read_error(
        &SourceQueryService::execute_json(Path::new("/never-opened"), &request).into_value(),
    )
    .unwrap();
    assert_eq!(error.code, ErrorCode::InvalidRequest);
    assert_eq!(error.boundary, "request shape");
}

#[test]
fn utf8_chunks_reassemble_and_changed_request_cursor_refuses() {
    let (_root, host) = installed(vibe_core::manifest::SpecFormat::Markdown);
    let mut request = json!({"schema":1,"address":"spec://org.demo/dep/DOC#ONE","mode":"read","format":"md","children":true,"chunk_bytes":7});
    let mut concatenated = String::new();
    let mut first_cursor = None;
    loop {
        let response = SourceQueryService::execute_json(&host, &request);
        assert!(!response.is_error(), "{response:?}");
        let result = wire::read_result(&response.into_value()).unwrap();
        concatenated.push_str(&result.content);
        if first_cursor.is_none() {
            first_cursor = result.chunk.next_cursor.clone();
        }
        if let Some(cursor) = result.chunk.next_cursor {
            request["cursor"] = json!(cursor);
        } else {
            break;
        }
    }
    assert!(concatenated.contains("Selected. Кириллица 🦀."));
    request["cursor"] = json!(first_cursor.unwrap());
    request["children"] = json!(false);
    let error =
        wire::read_error(&SourceQueryService::execute_json(&host, &request).into_value()).unwrap();
    assert_eq!(error.code, ErrorCode::StaleCursor);
}

#[test]
fn copied_unconverted_refuses_structured_query_without_losing_raw_bytes() {
    use vibe_core::{ContentHash, manifest::SpecFormat};
    let (root, host) = installed(SpecFormat::Xml);
    let original = root.path().join("original");
    let bad = "# D {#root}\n\nPlain prose.\n\n@status:impl/done\n\nFollowing prose.\n";
    std::fs::write(original.join("vibevm/vibespecs/DOC.md"), bad).unwrap();
    let group = "org.demo".parse().unwrap();
    let version = "1.0.0".parse().unwrap();
    let hash = ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
    crate::vibedeps::materialise_with_spec_format(
        &host,
        &group,
        "dep",
        &version,
        &original,
        crate::vibedeps::CopyMode::Copy,
        SpecFormat::Xml,
        &hash,
    )
    .unwrap();
    let output = crate::vibedeps::slot_abs_path(&host, &group, "dep", &version)
        .join("vibevm/vibespecs/DOC.md");
    let before_lock = std::fs::read(host.join("vibe.lock")).unwrap();
    let response = SourceQueryService::execute_json(
        &host,
        &json!({"schema":1,"address":"spec://org.demo/dep/DOC#root","mode":"resolve","format":"md","children":true}),
    );
    assert!(response.is_error(), "{response:?}");
    let error = wire::read_error(&response.into_value()).unwrap();
    assert_eq!(error.code, ErrorCode::StructuredUnavailable);
    assert_eq!(error.boundary, "structured view");
    assert!(error.message.contains("standalone marker"), "{error:?}");
    assert!(error.message.contains("line 5"), "{error:?}");
    assert_eq!(std::fs::read(output).unwrap(), bad.as_bytes());
    assert_eq!(std::fs::read(host.join("vibe.lock")).unwrap(), before_lock);
}

fn installed(format: vibe_core::manifest::SpecFormat) -> (tempfile::TempDir, PathBuf) {
    use vibe_core::{ContentHash, manifest::Lockfile};
    let root = tempfile::tempdir().unwrap();
    let host = root.path().join("host");
    let source = root.path().join("original");
    std::fs::create_dir_all(&host).unwrap();
    std::fs::create_dir_all(source.join("vibevm/vibespecs")).unwrap();
    std::fs::write(host.join("vibe.toml"),"[project]\ngroup='org.demo'\nname='consumer'\nversion='1.0.0'\n[requires.packages]\n'flow:org.demo/dep'='=1.0.0'\n[workspace]\nmembers=['client','unrelated-broken']\n").unwrap();
    std::fs::create_dir(host.join("unrelated-broken")).unwrap();
    std::fs::write(host.join("unrelated-broken/vibe.toml"), "bad = [").unwrap();
    std::fs::create_dir(host.join("client")).unwrap();
    std::fs::write(host.join("client/vibe.toml"),"[package]\ngroup='org.demo'\nname='client'\nkind='flow'\nversion='1.0.0'\n[requires.packages]\n'flow:org.demo/dep'='=1.0.0'\n").unwrap();
    std::fs::write(source.join("vibe.toml"),"[package]\ngroup='org.demo'\nname='dep'\nkind='flow'\nversion='1.0.0'\n[[package.sources]]\nfile='vibevm/vibespecs/DOC.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n").unwrap();
    std::fs::write(source.join("vibevm/vibespecs/DOC.md"),"# D {#root}\n\n## Solution {#solution}\n\n@fact:ONE Selected. Кириллица 🦀. @status:impl/done\n\n- @fact:LIST Item. @status:impl/done\n\n| Heading | Value |\n| --- | --- |\n| @fact:CC-A-4-1 First. @status:impl/done | @fact:CC-A-4-2 Second. @status:impl/done |\n\n### Child {#child}\n\n@fact:DEEP Child. @status:impl/done\n\n## Other {#other}\n\nUnselected.\n").unwrap();
    let group = "org.demo".parse().unwrap();
    let version = "1.0.0".parse().unwrap();
    let hash = ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
    crate::vibedeps::materialise_with_spec_format(
        &host,
        &group,
        "dep",
        &version,
        &source,
        crate::vibedeps::CopyMode::Copy,
        format,
        &hash,
    )
    .unwrap();
    let mut lock = Lockfile::empty("fixture", "fixture");
    lock.packages.push(toml::from_str(&format!("kind='flow'\ngroup='org.demo'\nname='dep'\nversion='1.0.0'\nsource_url='fixture'\ncontent_hash='{hash}'\n")).unwrap());
    lock.write(host.join("vibe.lock")).unwrap();
    (root, host)
}

#[test]
fn hidden_transitive_payload_cannot_poison_public_point_query_or_be_read() {
    use vibe_core::{
        ContentHash,
        manifest::{Lockfile, SpecFormat},
    };
    let (root, host) = installed(SpecFormat::Markdown);
    let original = root.path().join("original");
    let manifest = std::fs::read_to_string(original.join("vibe.toml")).unwrap()
        + "\n[requires.packages]\n'flow:org.demo/hidden'={version='=1.0.0',access='private'}\n";
    std::fs::write(original.join("vibe.toml"), manifest).unwrap();
    let hidden = root.path().join("hidden-original");
    std::fs::create_dir_all(hidden.join("vibevm/vibespecs")).unwrap();
    std::fs::write(hidden.join("vibe.toml"),"[package]\ngroup='org.demo'\nname='hidden'\nkind='flow'\nversion='1.0.0'\n[[package.sources]]\nfile='vibevm/vibespecs/SECRET.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n").unwrap();
    std::fs::write(
        hidden.join("vibevm/vibespecs/SECRET.md"),
        "# Secret {#root}\n\nSecret.\n",
    )
    .unwrap();
    let group = "org.demo".parse().unwrap();
    let version = "1.0.0".parse().unwrap();
    let hash = ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
    for (name, source) in [("dep", &original), ("hidden", &hidden)] {
        crate::vibedeps::materialise_with_spec_format(
            &host,
            &group,
            name,
            &version,
            source,
            crate::vibedeps::CopyMode::Copy,
            SpecFormat::Markdown,
            &hash,
        )
        .unwrap();
    }
    let mut lock = Lockfile::read(host.join("vibe.lock")).unwrap();
    lock.packages[0]
        .dependencies
        .push(vibe_core::PackageRef::parse("flow:org.demo/hidden@=1.0.0").unwrap());
    lock.packages.push(toml::from_str(&format!("kind='flow'\ngroup='org.demo'\nname='hidden'\nversion='1.0.0'\nsource_url='fixture'\ncontent_hash='{hash}'\n")).unwrap());
    lock.write(host.join("vibe.lock")).unwrap();
    let slot = crate::vibedeps::slot_abs_path(&host, &group, "hidden", &version);
    // Actual hidden payload disagrees with its record. Its captured manifest
    // alone suffices for visibility; this is not public query byte authority.
    std::fs::write(
        slot.join("vibevm/vibespecs/SECRET.md"),
        "changed private bytes",
    )
    .unwrap();
    let mut request = json!({"schema":1,"address":"spec://org.demo/dep/DOC#ONE","mode":"read","format":"md","children":true});
    let response = SourceQueryService::execute_json(&host, &request);
    assert!(!response.is_error(), "{response:?}");
    request["address"] = json!("spec://org.demo/hidden/SECRET#root");
    let response = SourceQueryService::execute_json(&host, &request);
    assert!(response.is_error());
    assert_eq!(
        wire::read_error(&response.into_value()).unwrap().code,
        ErrorCode::UnknownPackage
    );
}

#[test]
fn selected_md_xml_slots_and_member_keep_exact_version_and_native_location() {
    use vibe_core::manifest::SpecFormat;
    let mut semantic = None;
    for format in [SpecFormat::Markdown, SpecFormat::Xml] {
        let (_root, host) = installed(format);
        let stray = crate::vibedeps::slot_abs_path(
            &host,
            &"org.demo".parse().unwrap(),
            "dep",
            &"2.0.0".parse().unwrap(),
        );
        std::fs::create_dir_all(stray.join("vibevm/vibespecs")).unwrap();
        std::fs::write(stray.join("vibe.toml"), "unselected = [").unwrap();
        std::fs::write(stray.join("vibevm/vibespecs/DOC.md"), "NEWEST_POISON").unwrap();
        for node in [host.clone(), host.join("client")] {
            let response = SourceQueryService::execute_json(
                &node,
                &json!({"schema":1,"address":"spec://org.demo/dep/DOC#root.solution","mode":"read","format":"md","children":false}),
            );
            assert!(!response.is_error(), "{response:?}");
            let result = wire::read_result(&response.into_value()).unwrap();
            assert_eq!(
                result.address,
                "spec://org.demo/dep@1.0.0/DOC#root.solution"
            );
            assert!(result.content.contains("Selected."));
            assert!(!result.content.contains("Child."));
            assert!(
                result
                    .location
                    .path
                    .ends_with(if format == SpecFormat::Xml {
                        "DOC.xml"
                    } else {
                        "DOC.md"
                    })
            );
            assert_eq!(result.observation.strength,vibe_wire::generated::source_query::e1::result::ObservationStrength::RecordConsistent);
            if let Some(expected) = &semantic {
                assert_eq!(&result.observation.semantic_hash, expected);
            } else {
                semantic = Some(result.observation.semantic_hash);
            }
        }
    }
}

#[test]
fn real_project_source_capture_returns_its_declared_node() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("vibevm/vibespecs")).unwrap();
    std::fs::write(root.path().join("vibe.toml"),"[project]\ngroup='org.demo'\nname='self'\nversion='1.0.0'\n[[project.sources]]\nfile='vibevm/vibespecs/DOC.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n").unwrap();
    std::fs::write(
        root.path().join("vibevm/vibespecs/DOC.md"),
        "# D {#root}\n\n## Solution {#solution}\n\nBody.\n",
    )
    .unwrap();
    let response = SourceQueryService::execute_json(
        root.path(),
        &json!({"schema":1,"address":"spec://org.demo/self/DOC#solution","mode":"read","format":"md","children":true}),
    );
    assert!(!response.is_error(), "{response:?}");
    assert_eq!(
        response.into_value()["location"]["path"],
        "vibevm/vibespecs/DOC.md"
    );
}
#[test]
fn real_self_preparation_returns_validated_selected_solution() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("vibevm/vibespecs")).unwrap();
    std::fs::write(root.path().join("vibe.toml"),"[package]\ngroup='org.demo'\nname='self'\nkind='flow'\nversion='1.0.0'\n[[package.sources]]\nfile='vibevm/vibespecs/DOC.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n[workspace]\nmembers=['unrelated-broken']\n").unwrap();
    std::fs::create_dir(root.path().join("unrelated-broken")).unwrap();
    std::fs::write(
        root.path().join("unrelated-broken/vibe.toml"),
        "malformed = [",
    )
    .unwrap();
    std::fs::write(root.path().join("vibevm/vibespecs/DOC.md"),"# D {#root}\n\n## Solution {#solution}\n\n@fact:ONE Selected. @status:impl/done\n\n## Other {#other}\n\nUnselected.\n").unwrap();
    let request = json!({"schema":1,"address":"spec://org.demo/self/DOC#root.solution","mode":"read","format":"md","children":true});
    let response = SourceQueryService::execute_json(root.path(), &request);
    assert!(!response.is_error(), "{response:?}");
    let value = response.into_value();
    let result = wire::read_result(&value).unwrap();
    assert!(result.content.contains("Selected."));
    assert!(!result.content.contains("Unselected."));
    assert_eq!(result.heading_chain, ["D", "Solution"]);
    assert_eq!(result.location.path, "vibevm/vibespecs/DOC.md");
    assert_eq!(result.location.line_start, 3);
    assert_eq!(
        result.observation.strength,
        vibe_wire::generated::source_query::e1::result::ObservationStrength::Observed
    );
}

#[test]
fn cursor_refuses_same_byte_distinct_object_between_requests() {
    let root = tempfile::tempdir().unwrap();
    let sources = root.path().join("vibevm/vibespecs");
    std::fs::create_dir_all(&sources).unwrap();
    std::fs::write(root.path().join("vibe.toml"), "[package]\ngroup='org.demo'\nname='self'\nkind='flow'\nversion='1.0.0'\n[[package.sources]]\nfile='vibevm/vibespecs/DOC.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n").unwrap();
    let bytes = b"# D {#root}\n\n## Solution {#solution}\n\nSeveral bytes require another chunk.\n";
    let file = sources.join("DOC.md");
    std::fs::write(&file, bytes).unwrap();
    let mut request = json!({"schema":1,"address":"spec://org.demo/self/DOC#solution","mode":"read","format":"md","children":true,"chunk_bytes":7});
    let first =
        wire::read_result(&SourceQueryService::execute_json(root.path(), &request).into_value())
            .unwrap();
    let project = vibe_safefs::Project::open(root.path()).unwrap();
    let before = project
        .read_file_snapshot_bounded("vibevm/vibespecs/DOC.md", 4096)
        .unwrap()
        .unwrap();
    let replacement = sources.join("replacement.md");
    std::fs::write(&replacement, bytes).unwrap();
    std::fs::rename(&replacement, &file).unwrap();
    let after = project
        .read_file_snapshot_bounded("vibevm/vibespecs/DOC.md", 4096)
        .unwrap()
        .unwrap();
    assert_eq!(before.bytes, after.bytes);
    assert_ne!(before.identity, after.identity);
    request["cursor"] = json!(first.chunk.next_cursor.unwrap());
    let response = SourceQueryService::execute_json(root.path(), &request);
    assert!(
        response.is_error(),
        "new object reused old observation cursor: {response:?}"
    );
    let error = wire::read_error(&response.into_value()).unwrap();
    assert_eq!(error.code, ErrorCode::StaleCursor);
    assert_eq!(error.boundary, "chunk cursor");
}

#[test]
fn selected_lock_cannot_override_node_declared_version_constraint() {
    let (_root, host) = installed(vibe_core::manifest::SpecFormat::Markdown);
    let file = host.join("vibe.toml");
    let manifest = std::fs::read_to_string(&file)
        .unwrap()
        .replace("'=1.0.0'", "'=2.0.0'");
    std::fs::write(file, manifest).unwrap();
    let response = SourceQueryService::execute_json(
        &host,
        &json!({"schema":1,"address":"spec://org.demo/dep/DOC#ONE","mode":"read","format":"md","children":true}),
    );
    assert!(
        response.is_error(),
        "old lock override accepted: {response:?}"
    );
    let error = wire::read_error(&response.into_value()).unwrap();
    assert_eq!(error.code, ErrorCode::UnavailableWorld);
    assert_eq!(error.boundary, "selected requirement version");
}

#[test]
fn whole_document_status_does_not_borrow_selected_section_status() {
    let root = tempfile::tempdir().unwrap();
    let sources = root.path().join("vibevm/vibespecs");
    std::fs::create_dir_all(&sources).unwrap();
    std::fs::write(root.path().join("vibe.toml"), "[package]\ngroup='org.demo'\nname='self'\nkind='flow'\nversion='1.0.0'\n[[package.sources]]\nfile='vibevm/vibespecs/DOC.md'\nmedia='text/markdown'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n").unwrap();
    std::fs::write(sources.join("DOC.md"), "# D {#root}\n\n## Solution {#solution}\n\n@status:impl/done\n\n@fact:ONE Selected. @status:impl/done\n").unwrap();
    for (anchor, status) in [
        ("", None),
        ("#root.solution", Some("impl/done")),
        ("#ONE", Some("impl/done")),
    ] {
        let response = SourceQueryService::execute_json(
            root.path(),
            &json!({"schema":1,"address":format!("spec://org.demo/self/DOC{anchor}"),"mode":"read","format":"md","children":true}),
        );
        assert!(!response.is_error(), "{response:?}");
        let result = wire::read_result(&response.into_value()).unwrap();
        assert_eq!(result.status.as_deref(), status, "{anchor}");
    }
}

#[test]
fn ambiguous_dotted_node_refuses_with_actual_candidate_paths() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("vibevm/vibespecs")).unwrap();
    std::fs::write(root.path().join("vibe.toml"), "[project]\nname='host'\ngroup='com.example'\nversion='1.0.0'\n[[project.sources]]\nfile='vibevm/vibespecs/DOC.xml'\nmedia='application/xml'\ninterpretation='vibe-spec'\nrole='spec'\nloading='explicit'\n").unwrap();
    std::fs::write(root.path().join("vibevm/vibespecs/DOC.xml"), "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">D</title><section id=\"a\" title=\"A\"><section id=\"b\" title=\"B\"><p>Nested.</p></section></section><section id=\"a.b\" title=\"Literal\"><p>Literal.</p></section></spec>").unwrap();
    let response = SourceQueryService::execute_json(
        root.path(),
        &json!({"schema":1,"address":"spec://com.example/host/DOC#a.b","mode":"read","format":"md","children":true}),
    );
    assert!(response.is_error(), "{response:?}");
    let error = wire::read_error(&response.into_value()).unwrap();
    assert_eq!(error.code, ErrorCode::AmbiguousAnchor);
    assert_eq!(error.candidates.len(), 2);
    assert!(error.candidates.iter().any(|path| path == "root.a.b"));
    assert!(error.candidates.iter().any(|path| path.contains("\"a.b\"")));
}
