//! What the tool decides, which is only ever three things: the argument
//! shape, the projection asked for, and where to look.
//!
//! What a projection CONTAINS is `vibe_doc::agent`'s law and is tested
//! there. Restating it here would be a second opinion about the same
//! bytes.

use std::fs;
use std::path::Path;

use super::*;
use crate::ServerContext;

const PAGE: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<spec xmlns=\"https://vibevm.org/spec/1\">\n  \
  <title id=\"root\">Boot lane</title>\n  \
  <p>The boot lane is what a session reads first.</p>\n\
</spec>\n";

fn project(root: &Path) {
    fs::write(
        root.join("vibe.toml"),
        "[project]\nname = \"host\"\ngroup = \"com.example\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    let dir = root.join("vibevm/vibepacks/com.example/thing-docs/v0.1.0");
    fs::create_dir_all(dir.join("vibevm/vibespecs/model")).unwrap();
    fs::write(
        dir.join("vibe.toml"),
        "[package]\nname = \"thing-docs\"\ngroup = \"com.example\"\nkind = \"doc\"\n\
         version = \"0.1.0\"\ntitle = \"Thing\"\nabstract = \"What it covers.\"\n\
         [i18n]\ncanonical = \"en\"\n\
         [[documents]]\npackage = \"com.example/thing\"\nversion = \"^1.0\"\n",
    )
    .unwrap();
    fs::write(dir.join("vibevm/vibespecs/model/boot-lane.xml"), PAGE).unwrap();
}

fn context(tmp: &tempfile::TempDir) -> ServerContext {
    project(tmp.path());
    ServerContext::with_store_root(tmp.path().to_path_buf(), tmp.path().join("store"))
}

const ADDRESS: &str = "spec://com.example/thing-docs/model/boot-lane";

/// The tool in one call: an address in, a page and the facts about it
/// out.
#[test]
fn an_address_comes_back_as_a_page_and_its_provenance() {
    let tmp = tempfile::tempdir().unwrap();
    let out = ReadDocMcpTool
        .run(&json!({ "address": ADDRESS }), &context(&tmp))
        .expect("the page is in the checkout's own packages");
    let value = out.structured();
    assert_eq!(value["format"], "md");
    assert_eq!(value["lang"], "en");
    assert_eq!(value["source"], "in-tree");
    assert!(
        value["page"].as_str().unwrap_or_default().contains("[p01]"),
        "{value}"
    );
}

/// The island is for a browser. A tool that returned it to an agent
/// would be charging tokens for markup nothing here reads.
#[test]
fn html_is_refused_by_name_rather_than_silently_substituted() {
    let tmp = tempfile::tempdir().unwrap();
    let e = ReadDocMcpTool
        .run(
            &json!({ "address": ADDRESS, "format": "html" }),
            &context(&tmp),
        )
        .expect_err("html is not a projection an agent reads");
    assert!(format!("{e:?}").contains("md"), "{e:?}");
}

/// An address is required, and its absence is an argument error rather
/// than an empty answer.
#[test]
fn a_missing_address_is_an_argument_error() {
    let tmp = tempfile::tempdir().unwrap();
    ReadDocMcpTool
        .run(&json!({}), &context(&tmp))
        .expect_err("`address` is required");
}

/// The descriptor is the whole contract an agent sees, so it names the
/// two projections and the fallback rule rather than leaving either to
/// be discovered.
#[test]
fn the_descriptor_teaches_the_projections_and_the_language_fallback() {
    let d = ReadDocMcpTool.descriptor();
    assert_eq!(d.name, "read_doc");
    assert!(d.description.contains("`md`"), "{}", d.description);
    assert!(d.description.contains("`xml`"), "{}", d.description);
    assert!(d.description.contains("falls back"), "{}", d.description);
    assert!(
        d.description.contains("vibe cache add"),
        "{}",
        d.description
    );
}

#[test]
fn strict_epoch_uses_selected_capture_and_legacy_stays_explicit() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    fs::create_dir_all(root.join("vibevm/vibespecs")).unwrap();
    fs::write(root.join("vibe.toml"), "[project]\nname='host'\ngroup='com.example'\nversion='1.0.0'\n[[project.sources]]\nfile='vibevm/vibespecs/guide.md'\nmedia='application/xml'\ninterpretation='vibe-doc'\nrole='documentation'\nloading='scan'\n").unwrap();
    fs::write(root.join("vibevm/vibespecs/guide.md"), "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">Guide</title><p>Whole page.</p><note kind=\"tip\">A doc-only tip.</note></spec>\n").unwrap();
    let ctx = ServerContext::with_store_root(root, root.join("unused-store"));
    let address = "spec://com.example/host/guide#p01";
    let out = ReadDocMcpTool
        .run(&json!({"schema":1,"address":address,"lang":"ru"}), &ctx)
        .unwrap();
    let value = out.structured();
    let reply = vibe_wire::behaviour::source_query::read_doc_result(value).unwrap();
    assert_eq!(reply.lang, "en");
    assert_eq!(value["source_authority"], "selected-captured");
    assert!(reply.page.contains("[p01]"));
    assert!(reply.page.contains("A doc-only tip."));
    assert_eq!(value["selection"]["dictionary"], "vibe-doc");
    assert_eq!(value["selection"]["location"]["basis"], "native-xml");
    assert!(
        value["selection"]["location"]["path"]
            .as_str()
            .unwrap()
            .ends_with("guide.md")
    );
    assert_eq!(reply.page_bytes as usize, reply.page.len());
    let source = crate::tools::ReadSpecMcpTool.run(&json!({"schema":1,"address":"spec://com.example/host/guide","mode":"resolve","format":"md","children":true}), &ctx).unwrap();
    assert_eq!(source.structured(), &value["selection"]);
    let old = ReadDocMcpTool
        .run(&json!({"address":address}), &ctx)
        .unwrap();
    assert_eq!(old.structured()["source_authority"], "legacy-four-source");
    assert_eq!(old.structured()["page"], value["page"]);
    assert!(!root.join("unused-store").exists());
    if std::env::var("VIBE_D2_EXPORT_DOC_REPLY").as_deref() == Ok("1") {
        println!("D2_DOC_REPLY {}", serde_json::to_string(value).unwrap());
    }
}

#[test]
fn strict_epoch_refuses_unselected_legacy_package_and_invalid_shape_before_preparation() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = context(&tmp);
    let legacy = ReadDocMcpTool
        .run(&json!({"address":ADDRESS}), &ctx)
        .unwrap();
    assert_eq!(legacy.structured()["source"], "in-tree");
    assert_eq!(
        legacy.structured()["source_authority"],
        "legacy-four-source"
    );
    let error = ReadDocMcpTool
        .run(&json!({"schema":1,"address":ADDRESS}), &ctx)
        .unwrap_err();
    let ToolError::StructuredPreflight { report, .. } = error else {
        panic!("typed strict refusal");
    };
    assert_eq!(report["code"], "unknown_package");
    let absent = ServerContext::with_store_root(
        tmp.path().join("never-opened"),
        tmp.path().join("unused-store"),
    );
    for (args, code) in [
        (json!({"schema":2,"address":ADDRESS}), "unsupported_epoch"),
        (
            json!({"schema":1,"address":ADDRESS,"root":"/foreign"}),
            "invalid_request",
        ),
    ] {
        let error = ReadDocMcpTool.run(&args, &absent).unwrap_err();
        let ToolError::StructuredPreflight { report, .. } = error else {
            panic!("typed shape refusal");
        };
        assert_eq!(report["code"], code);
        vibe_wire::behaviour::source_query::read_error(&report).unwrap();
    }
    assert!(!tmp.path().join("never-opened").exists());
    assert!(!tmp.path().join("unused-store").exists());
}

#[test]
fn strict_selected_translation_borrows_original_from_actual_md_xml_capture() {
    use vibe_core::manifest::SpecFormat;
    selected_translation_cases(&[SpecFormat::Markdown, SpecFormat::Xml]);
}

#[test]
fn strict_selected_xml_pivot_translation_preserves_borrowed_example() {
    selected_translation_cases(&[vibe_core::manifest::SpecFormat::Xml]);
}

fn selected_translation_cases(formats: &[vibe_core::manifest::SpecFormat]) {
    use vibe_core::{
        ContentHash,
        manifest::{Lockfile, SpecFormat},
    };
    for &format in formats {
        let tmp = tempfile::tempdir().unwrap();
        let host = tmp.path().join("host");
        fs::create_dir(&host).unwrap();
        fs::write(host.join("vibe.toml"), "[project]\nname='host'\ngroup='com.example'\nversion='1.0.0'\n[requires.packages]\n'doc:com.example/guide-docs'='=0.1.0'\n'doc:com.example/guide-docs-ru'='=0.1.0'\n").unwrap();
        let group = "com.example".parse().unwrap();
        let version = "0.1.0".parse().unwrap();
        let hash = ContentHash::parse(&format!("sha256:{}", "a".repeat(64))).unwrap();
        let mut lock = Lockfile::empty("fixture", "fixture");
        for (name, lang, page, relation) in [
            (
                "guide-docs",
                "en",
                "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">Guide</title><p>Source.</p><example id=\"demo\" fixture=\"none\"><run>vibe --version</run><expect>one held example</expect></example></spec>",
                "",
            ),
            (
                "guide-docs-ru",
                "ru",
                "<spec xmlns=\"https://vibevm.org/spec/1\"><title id=\"root\">Руководство</title><p>Перевод.</p><example ref=\"demo\"/></spec>",
                "[translates]\npackage='com.example/guide-docs'\nversion='^0.1'\n",
            ),
        ] {
            let original = tmp.path().join(name);
            fs::create_dir_all(original.join("vibevm/vibespecs")).unwrap();
            fs::write(original.join("vibe.toml"), format!("[package]\nname='{name}'\ngroup='com.example'\nkind='doc'\nversion='0.1.0'\ntitle='Guide'\nabstract='Documentation.'\n[[package.sources]]\nfile='vibevm/vibespecs/guide.xml'\nmedia='application/xml'\ninterpretation='vibe-doc'\nrole='documentation'\nloading='explicit'\n[i18n]\ncanonical='{lang}'\n[[documents]]\npackage='com.example/thing'\nversion='^1.0'\n{relation}")).unwrap();
            fs::write(original.join("vibevm/vibespecs/guide.xml"), page).unwrap();
            vibe_workspace::vibedeps::materialise_with_spec_format(
                &host,
                &group,
                name,
                &version,
                &original,
                vibe_workspace::vibedeps::CopyMode::Copy,
                format,
                &hash,
            )
            .unwrap();
            lock.packages.push(toml::from_str(&format!("kind='doc'\ngroup='com.example'\nname='{name}'\nversion='0.1.0'\nsource_url='fixture'\ncontent_hash='{hash}'\n")).unwrap());
        }
        lock.write(host.join("vibe.lock")).unwrap();
        let ctx = ServerContext::with_store_root(&host, host.join("unused-store"));
        if format == SpecFormat::Markdown {
            let service =
                vibe_workspace::source_query::SourceQueryService::prepare_host(&host).unwrap();
            let uri = "spec://com.example/guide-docs-ru/guide";
            let raw = service.captured_bytes(uri).unwrap();
            let slot =
                vibe_workspace::vibedeps::slot_abs_path(&host, &group, "guide-docs-ru", &version);
            assert_eq!(
                raw,
                fs::read(slot.join("vibevm/vibespecs/guide.md")).unwrap()
            );
            let before_lock = fs::read(host.join("vibe.lock")).unwrap();
            let error = ReadDocMcpTool
                .run(&json!({"schema":1,"address":uri,"format":"md"}), &ctx)
                .unwrap_err();
            let ToolError::StructuredPreflight { report, .. } = error else {
                panic!("typed projection refusal");
            };
            assert_eq!(report["code"], "structured_unavailable");
            assert_eq!(report["boundary"], "documentation view");
            assert!(
                report["message"]
                    .as_str()
                    .unwrap()
                    .contains("one-way projection")
            );
            assert_eq!(service.captured_bytes(uri).unwrap(), raw);
            assert_eq!(fs::read(host.join("vibe.lock")).unwrap(), before_lock);
            assert!(!host.join("unused-store").exists());
            if std::env::var("VIBE_D2_EXPORT_DOC_FIXTURES").as_deref() == Ok("1") {
                println!("D2_DOC_FIXTURE {format:?} {}", tmp.keep().display());
            }
            continue;
        }
        for output in ["md", "xml"] {
            let out = ReadDocMcpTool.run(&json!({"schema":1,"address":"spec://com.example/guide-docs/guide#p02","lang":"ru","format":output}), &ctx).unwrap();
            let value = out.structured();
            vibe_wire::behaviour::source_query::read_doc_result(value).unwrap();
            assert_eq!(value["lang"], "ru", "{format:?}: {value}");
            assert_eq!(
                value["address"],
                "spec://com.example/guide-docs-ru@0.1.0/guide"
            );
            let page = value["page"].as_str().unwrap();
            if output == "md" {
                assert!(page.contains("vibe --version"), "{format:?}: {value}");
                assert!(page.contains("one held example"), "{format:?}: {value}");
            } else {
                assert!(
                    page.contains("<example ref=\"demo\" p=\"2\"/>"),
                    "{format:?}: {value}"
                );
                assert!(!page.contains("vibe --version"), "{format:?}: {value}");
            }
            assert_eq!(
                value["selection"]["observation"]["strength"],
                "record-consistent"
            );
            assert!(
                value["selection"]["location"]["path"]
                    .as_str()
                    .unwrap()
                    .ends_with(if format == SpecFormat::Xml {
                        "guide.xml"
                    } else {
                        "guide.md"
                    })
            );
        }
        assert!(!host.join("unused-store").exists());
        if std::env::var("VIBE_D2_EXPORT_DOC_FIXTURES").as_deref() == Ok("1") {
            println!("D2_DOC_FIXTURE {format:?} {}", tmp.keep().display());
        }
    }
}
