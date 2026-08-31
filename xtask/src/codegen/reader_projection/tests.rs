use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use serde_json::{Value, json};

use super::*;
use crate::codegen::output_tree::StagedOutputTree;
use crate::codegen::strictness::Strictness;
use crate::codegen::vocabulary::{Vocabularies, vocabularies_path};
use crate::codegen::{FormatOwner, GenerationGroup, find_jtd_codegen, generate_into};
use crate::repo_root;

fn write_json(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}

fn fragments() -> Value {
    json!({
        "closed_mode": {
            "metadata": {"x-vocabulary": "closed"},
            "enum": ["on", "off"]
        },
        "choice": {
            "discriminator": "kind",
            "mapping": {
                "text": {"properties": {"text": {"type": "string"}}},
                "count": {"properties": {"count": {"type": "uint32"}}}
            }
        },
        "nested": {
            "properties": {"value": {"type": "string"}}
        },
        "empty": {
            "properties": {}
        },
        "payload": {
            "metadata": {"x-vocabularies": ["closed_mode", "choice", "empty", "nested"]},
            "properties": {
                "name": {"type": "string"},
                "nested": {"ref": "nested"},
                "empty": {"ref": "empty"},
                "mode": {"ref": "closed_mode"},
                "choice": {"ref": "choice"}
            }
        }
    })
}

fn strict_schema() -> Value {
    json!({
        "metadata": {"x-vocabularies": ["payload"]},
        "ref": "payload"
    })
}

fn projected_schema() -> Value {
    json!({
        "metadata": {"x-vocabularies": ["payload"]},
        "properties": {
            "payload": {
                "ref": "payload",
                "metadata": {"x-reader-projection": "permissive"}
            }
        }
    })
}

fn ordinary_schema() -> Value {
    json!({
        "metadata": {"x-vocabularies": ["payload"]},
        "properties": {"payload": {"ref": "payload"}}
    })
}

fn record(id: &str, schema: &str, role: &str) -> String {
    format!(
        "[format.{id}]\nepoch = 1\nschema = \"{schema}\"\nrecoverable = true\nforeign_parsers = \"{role}\"\ncorpus = \"none\"\nsunset = \"none\"\n\n"
    )
}

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    generated: PathBuf,
}

fn generate_fixture(owner_role: &str, ordinary: bool) -> Result<Fixture> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().to_path_buf();
    let schemas = root.join("schemas");
    let formats = root.join("formats");
    std::fs::create_dir_all(&schemas)?;
    std::fs::create_dir_all(&formats)?;
    write_json(&formats.join("vocabularies.json"), &fragments())?;
    write_json(&schemas.join("strict_payload.jtd.json"), &strict_schema())?;
    write_json(
        &schemas.join("projected_request.jtd.json"),
        &projected_schema(),
    )?;
    let mut registry = record(
        "strict-payload",
        "schemas/strict_payload.jtd.json",
        owner_role,
    );
    registry.push_str(&record(
        "projected-request",
        "schemas/projected_request.jtd.json",
        "many",
    ));
    let mut schema_paths = vec![
        schemas.join("projected_request.jtd.json"),
        schemas.join("strict_payload.jtd.json"),
    ];
    if ordinary {
        write_json(
            &schemas.join("ordinary_request.jtd.json"),
            &ordinary_schema(),
        )?;
        registry.push_str(&record(
            "ordinary-request",
            "schemas/ordinary_request.jtd.json",
            "many",
        ));
        schema_paths.push(schemas.join("ordinary_request.jtd.json"));
        schema_paths.sort();
    }
    std::fs::write(formats.join("REGISTRY.toml"), registry)?;

    let generated = root.join("crates/vibe-wire/src/generated");
    let staged = StagedOutputTree::prepare(&generated)?;
    let binary = find_jtd_codegen(&repo_root()?)?;
    let group = GenerationGroup {
        schema_root: schemas,
        live_out_dir: generated.clone(),
        owner: FormatOwner::Ours,
        schemas: schema_paths,
    };
    let mut vocabularies = Vocabularies::load(&vocabularies_path(&root))?;
    let strictness = Strictness::load(&root)?;
    generate_into(
        &binary,
        &root,
        &group,
        staged.fresh(),
        &mut vocabularies,
        &strictness,
    )?;
    staged.install()?;
    Ok(Fixture {
        _temp: temp,
        root,
        generated,
    })
}

fn include_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "\\\\")
}

fn newest_rlib(deps: &Path, stem: &str) -> Result<PathBuf> {
    let prefix = format!("lib{stem}-");
    let mut candidates = std::fs::read_dir(deps)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&prefix) && name.ends_with(".rlib"))
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|path| {
        std::fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .ok()
    });
    candidates
        .pop()
        .with_context(|| format!("finding {prefix}*.rlib under {}", deps.display()))
}

fn compile_and_run_generated(fixture: &Fixture) -> Result<()> {
    let shared = include_path(&fixture.generated.join("shared/mod.rs"));
    let strict = include_path(&fixture.generated.join("strict_payload/mod.rs"));
    let projected = include_path(&fixture.generated.join("projected_request/mod.rs"));
    let source = format!(
        r###"
mod generated {{
    pub mod shared {{ include!(r#"{shared}"#); }}
    pub mod strict_payload {{ include!(r#"{strict}"#); }}
    pub mod projected_request {{ include!(r#"{projected}"#); }}
}}

fn main() {{
    use std::any::TypeId;
    use generated::projected_request::ProjectedRequest;

    let with_unknown = r#"{{"payload":{{"name":"demo","nested":{{"value":"ok","future":7}},"empty":{{"future":7}},"mode":"on","choice":{{"kind":"text","text":"x","future":true}},"future_root":"ignored"}}}}"#;
    let decoded: ProjectedRequest = serde_json::from_str(with_unknown).unwrap();
    assert_eq!(decoded.payload.nested.value, "ok");
    assert_eq!(
        TypeId::of::<generated::strict_payload::StrictPayload>(),
        TypeId::of::<generated::projected_request::Payload>()
    );

    let strict_payload = r#"{{"name":"demo","nested":{{"value":"ok","future":7}},"empty":{{}},"mode":"on","choice":{{"kind":"text","text":"x"}}}}"#;
    assert!(serde_json::from_str::<generated::strict_payload::StrictPayload>(strict_payload).is_err());
    let missing = r#"{{"payload":{{"name":"demo","empty":{{}},"mode":"on","choice":{{"kind":"text","text":"x"}}}}}}"#;
    assert!(serde_json::from_str::<ProjectedRequest>(missing).is_err());
    let wrong_type = r#"{{"payload":{{"name":"demo","nested":{{"value":7}},"empty":{{}},"mode":"on","choice":{{"kind":"text","text":"x"}}}}}}"#;
    assert!(serde_json::from_str::<ProjectedRequest>(wrong_type).is_err());
    let unknown_tag = r#"{{"payload":{{"name":"demo","nested":{{"value":"ok"}},"empty":{{}},"mode":"on","choice":{{"kind":"future","text":"x"}}}}}}"#;
    assert!(serde_json::from_str::<ProjectedRequest>(unknown_tag).is_err());
    let unknown_enum = r#"{{"payload":{{"name":"demo","nested":{{"value":"ok"}},"empty":{{}},"mode":"future","choice":{{"kind":"text","text":"x"}}}}}}"#;
    assert!(serde_json::from_str::<ProjectedRequest>(unknown_enum).is_err());
    let duplicate_root = r#"{{"payload":{{"name":"one","name":"two","nested":{{"value":"ok"}},"empty":{{}},"mode":"on","choice":{{"kind":"text","text":"x"}}}}}}"#;
    assert!(serde_json::from_str::<ProjectedRequest>(duplicate_root).is_err());
    let duplicate_nested = r#"{{"payload":{{"name":"demo","nested":{{"value":"one","value":"two"}},"empty":{{}},"mode":"on","choice":{{"kind":"text","text":"x"}}}}}}"#;
    assert!(serde_json::from_str::<ProjectedRequest>(duplicate_nested).is_err());
    let duplicate_arm = r#"{{"payload":{{"name":"demo","nested":{{"value":"ok"}},"empty":{{}},"mode":"on","choice":{{"kind":"text","text":"one","text":"two"}}}}}}"#;
    assert!(serde_json::from_str::<ProjectedRequest>(duplicate_arm).is_err());
}}
"###
    );
    let source_path = fixture.root.join("projection_fixture.rs");
    std::fs::write(&source_path, source)?;
    let deps = std::env::current_exe()?
        .parent()
        .context("test executable has no dependency directory")?
        .to_path_buf();
    let serde = newest_rlib(&deps, "serde")?;
    let serde_json = newest_rlib(&deps, "serde_json")?;
    let executable = fixture.root.join(if cfg!(windows) {
        "projection_fixture.exe"
    } else {
        "projection_fixture"
    });
    let status = Command::new("rustc")
        .arg("--edition=2024")
        .arg(&source_path)
        .arg("-L")
        .arg(format!("dependency={}", deps.display()))
        .arg("--extern")
        .arg(format!("serde={}", serde.display()))
        .arg("--extern")
        .arg(format!("serde_json={}", serde_json.display()))
        .arg("-o")
        .arg(&executable)
        .status()
        .context("compiling the generated projection fixture")?;
    anyhow::ensure!(
        status.success(),
        "generated projection fixture did not compile"
    );
    let status = Command::new(&executable)
        .status()
        .context("running the generated projection fixture")?;
    anyhow::ensure!(
        status.success(),
        "generated projection fixture assertions failed"
    );
    Ok(())
}

#[test]
fn full_pipeline_emits_strict_canonical_types_and_a_permissive_field_adapter() -> Result<()> {
    let fixture = generate_fixture("none", false)?;
    let shared = std::fs::read_to_string(fixture.generated.join("shared/mod.rs"))?;
    let strict = std::fs::read_to_string(fixture.generated.join("strict_payload/mod.rs"))?;
    let projected = std::fs::read_to_string(fixture.generated.join("projected_request/mod.rs"))?;
    assert!(shared.contains("#[serde(deny_unknown_fields)]\npub struct Payload"));
    assert!(shared.contains("#[serde(deny_unknown_fields)]\npub struct Nested"));
    assert!(strict.contains("pub type StrictPayload = Payload;"));
    assert!(strict.contains("pub use crate::generated::shared::Payload;"));
    assert!(!strict.contains("pub struct Payload"));
    assert!(projected.contains("__reader_projection::deserialize_payload_0"));
    assert!(projected.contains("object.retain"));
    assert!(projected.contains("object.clear();"));
    assert!(!projected.contains("pub struct Payload"));
    compile_and_run_generated(&fixture)
}

#[test]
fn ordinary_mixed_policy_still_refuses_through_the_full_driver() {
    let error = generate_fixture("none", true)
        .err()
        .expect("an ordinary permissive consumer must retain the mixed-policy refusal");
    let message = format!("{error:#}");
    assert!(
        message.contains("mixed registered reader policy"),
        "{message}"
    );
    assert!(message.contains("strict_payload"), "{message}");
    assert!(message.contains("ordinary_request"), "{message}");
}

#[test]
fn invalid_marker_shapes_and_values_refuse() {
    let fragments = BTreeSet::from(["payload".to_string()]);
    for (schema, needle) in [
        (
            json!({"properties":{"payload":{"type":"string","metadata":{"x-reader-projection":"permissive"}}}}),
            "only on a JTD `ref`",
        ),
        (
            json!({"properties":{"payload":{"ref":"payload","metadata":{"x-reader-projection":"relaxed"}}}}),
            "unknown `x-reader-projection` value",
        ),
        (
            json!({"properties":{"payload":{"ref":"payload","metadata":{"x-reader-projection":true}}}}),
            "must be the string",
        ),
        (
            json!({"ref":"payload","metadata":{"x-reader-projection":"permissive"}}),
            "not an object-member field",
        ),
        (
            json!({"definitions":{"alias":{"ref":"payload","metadata":{"x-reader-projection":"permissive"}}},"properties":{}}),
            "not an object-member field",
        ),
        (
            json!({"properties":{"items":{"elements":{"ref":"payload","metadata":{"x-reader-projection":"permissive"}}}}}),
            "not an object-member field",
        ),
        (
            json!({"properties":{"items":{"values":{"ref":"payload","metadata":{"x-reader-projection":"permissive"}}}}}),
            "not an object-member field",
        ),
        (
            json!({"metadata":{"note":{"x-reader-projection":"permissive"}},"properties":{}}),
            "not the direct metadata member",
        ),
        (
            json!({"x-reader-projection":"permissive","properties":{}}),
            "not the direct metadata member",
        ),
        (
            json!({"enum":[{"x-reader-projection":"permissive"}]}),
            "not the direct metadata member",
        ),
    ] {
        let error = scan_schema(&schema, Path::new("schemas/request.jtd.json"), &fragments)
            .expect_err("invalid projection marker must refuse");
        assert!(error.to_string().contains(needle), "{error:#}");
    }
}

#[test]
fn two_markers_may_not_resolve_to_one_generated_field() {
    let fragments = BTreeSet::from(["payload".to_string()]);
    let schema = json!({
        "properties": {
            "payload": {
                "ref": "payload",
                "metadata": {"x-reader-projection": "permissive"}
            }
        },
        "optionalProperties": {
            "payload": {
                "ref": "payload",
                "metadata": {"x-reader-projection": "permissive"}
            }
        }
    });
    let error = scan_schema(&schema, Path::new("schemas/request.jtd.json"), &fragments)
        .expect_err("two markers cannot own one generated field");
    let message = error.to_string();
    assert!(message.contains("properties.payload"));
    assert!(message.contains("optionalProperties.payload"));
    assert!(message.contains("Request::payload"));
}

#[test]
fn strict_consumer_and_missing_strict_owner_refuse() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(temp.path().join("formats")).unwrap();
    std::fs::write(
        temp.path().join("formats/REGISTRY.toml"),
        record("strict-request", "schemas/strict_request.jtd.json", "none"),
    )
    .unwrap();
    let schema = temp.path().join("schemas/strict_request.jtd.json");
    let projection = ProjectionUse {
        target: "payload".to_string(),
        owner_type: "StrictRequest".to_string(),
        rust_field: "payload".to_string(),
        location: "$.properties.payload".to_string(),
        closure: BTreeSet::from(["payload".to_string()]),
    };
    let strict_resolution = Resolved {
        doc: schema.clone(),
        vocabularies: BTreeSet::from(["payload".to_string()]),
        ordinary_vocabularies: BTreeSet::new(),
        projections: vec![projection],
    };
    let strict = validate_policies(temp.path(), &[(schema, strict_resolution)])
        .expect_err("a strict consumer may not request a permissive projection");
    assert!(format!("{strict:#}").contains("registry-permissive consumer"));

    let missing = generate_fixture("many", false)
        .err()
        .expect("a projected fragment without a strict owner must refuse");
    assert!(format!("{missing:#}").contains("no registered strict consumer"));
}

#[test]
fn a_marker_in_the_canonical_fragment_is_never_silently_unconsumed() {
    let fragments = json!({
        "payload": {
            "ref": "nested",
            "metadata": {"x-reader-projection": "permissive"}
        }
    });
    let error = reject_vocabulary_markers(
        fragments.as_object().unwrap(),
        Path::new("formats/vocabularies.json"),
    )
    .expect_err("projection belongs to a consumer site, never the canonical home");
    let message = error.to_string();
    assert!(message.contains("payload") && message.contains("consumed exactly once"));
}
