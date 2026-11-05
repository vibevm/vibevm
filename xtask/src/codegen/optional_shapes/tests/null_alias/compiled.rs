#![cfg(test)]
use super::*;
use crate::codegen::{FormatOwner, GenerationGroup, find_jtd_codegen, generate_into};
use crate::codegen::{strictness::Strictness, vocabulary::Vocabularies};
use anyhow::Context;
use std::{fs, path::PathBuf, process::Command};

fn artifacts(deps: &Path, stem: &str) -> Result<Vec<PathBuf>> {
    let mut files = fs::read_dir(deps)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.starts_with(&format!("lib{stem}-")) && s.ends_with(".rlib"))
        })
        .collect::<Vec<_>>();
    files.sort();
    anyhow::ensure!(!files.is_empty(), "missing {stem} artifacts");
    Ok(files)
}
#[test]
fn actual_pipeline_compiles_json_and_toml_aliases_without_changing_required_or_legacy_fields()
-> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path();
    let schemas = root.join("schemas");
    let formats = root.join("formats");
    let output = root.join("generated");
    fs::create_dir_all(&schemas)?;
    fs::create_dir_all(&formats)?;
    fs::create_dir_all(&output)?;
    let mut direct = alias(json!({"elements":{"type":"string"}}));
    direct["metadata"]["x-empty"] = "preserve".into();
    let mut map = alias(json!({"values":{"type":"string"}}));
    map["metadata"]["x-empty"] = "preserve".into();
    let schema = json!({"metadata":{"x-rust-type":"Fixture"},"properties":{"required":{"ref":"required"},"legacy":{"ref":"legacy"},"portable":{"ref":"portable"}},"definitions":{
        "required":{"properties":{"label":{"type":"string","nullable":true},"members":{"ref":"members","nullable":true}}},
        "legacy":{"optionalProperties":{"label":{"type":"string","metadata":{"x-default":null}}}},
        "portable":{"optionalProperties":{"label":alias(json!({"type":"string"})),"members":alias(json!({"ref":"members"})),"direct":direct,"map":map,"value":alias(json!({"ref":"value"})),"object":alias(json!({"ref":"object"})),"enabled":alias(json!({"type":"boolean"})),"count":alias(json!({"type":"int32"}))}},
        "members":{"elements":{"type":"string"}},"object":{"properties":{"name":{"type":"string"}}},
        "value":{"discriminator":"kind","mapping":{"text":{"properties":{"value":{"type":"string"}}}}}
    }});
    let schema_path = schemas.join("fixture.jtd.json");
    fs::write(&schema_path, serde_json::to_vec_pretty(&schema)?)?;
    fs::write(formats.join("vocabularies.json"), "{}")?;
    fs::write(
        formats.join("REGISTRY.toml"),
        "[format.fixture]\nepoch=1\nschema='schemas/fixture.jtd.json'\nrecoverable=false\nforeign_parsers='none'\ncorpus='none'\nsunset='none'\n",
    )?;
    let binary = find_jtd_codegen(&crate::repo_root()?)?;
    let mut vocab = Vocabularies::load(&formats.join("vocabularies.json"))?;
    let strict = Strictness::load(root)?;
    generate_into(
        &binary,
        root,
        &GenerationGroup {
            schema_root: schemas,
            live_out_dir: output.clone(),
            owner: FormatOwner::Ours,
            schemas: vec![schema_path],
        },
        &output,
        &mut vocab,
        &strict,
    )?;
    let helper = crate::repo_root()?.join("crates/vibe-wire/src/behaviour/required_nullable.rs");
    let program = format!(
        "mod generated {{ include!({:?}); }}\nmod behaviour {{ #[path={:?}] pub mod required_nullable; }}\n",
        output.join("fixture/mod.rs"),
        helper
    ) + PROGRAM;
    let source = root.join("verify.rs");
    fs::write(&source, program)?;
    let deps = std::env::current_exe()?
        .parent()
        .context("test artifact directory")?
        .to_path_buf();
    let executable = root.join(if cfg!(windows) {
        "verify.exe"
    } else {
        "verify"
    });
    let mut last = String::new();
    let mut built = false;
    'pairs: for serde in artifacts(&deps, "serde")? {
        for json in artifacts(&deps, "serde_json")? {
            for toml in artifacts(&deps, "toml")? {
                let output = Command::new("rustc")
                    .arg("--edition=2024")
                    .arg(&source)
                    .arg("-L")
                    .arg(format!("dependency={}", deps.display()))
                    .args([
                        "--extern",
                        &format!("serde={}", serde.display()),
                        "--extern",
                        &format!("serde_json={}", json.display()),
                        "--extern",
                        &format!("toml={}", toml.display()),
                    ])
                    .arg("-o")
                    .arg(&executable)
                    .output()?;
                if output.status.success() {
                    built = true;
                    break 'pairs;
                }
                last = String::from_utf8_lossy(&output.stderr).into_owned();
            }
        }
    }
    anyhow::ensure!(built, "generated fixture failed to compile: {last}");
    let run = Command::new(executable).output()?;
    anyhow::ensure!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    Ok(())
}
const PROGRAM: &str = r##"
fn main() {
    use generated::Fixture;
    let empty=r#"{"required":{"label":null,"members":null},"legacy":{},"portable":{}}"#;
    let a:Fixture=serde_json::from_str(empty).unwrap();
    assert!(a.required.label.is_none()&&a.required.members.is_none());
    assert!(serde_json::from_str::<Fixture>(r#"{"required":{"members":null},"legacy":{},"portable":{}}"#).is_err());
    let null=r#"{"required":{"label":null,"members":null},"legacy":{"label":null},"portable":{"label":null,"members":null,"direct":null,"map":null,"value":null,"object":null,"enabled":null,"count":null}}"#;
    let b:Fixture=serde_json::from_str(null).unwrap();assert_eq!(a,b);
    let encoded=serde_json::to_value(&b).unwrap();assert!(encoded["portable"].as_object().unwrap().is_empty());
    // Existing optional-nonnullable legacy admission is deliberately not tightened.
    assert!(b.legacy.label.is_none());
    let full=r#"{"required":{"label":"held","members":[]},"legacy":{},"portable":{"label":"","members":[],"direct":[],"map":{},"value":{"kind":"text","value":""},"object":{"name":""},"enabled":false,"count":0}}"#;
    let c:Fixture=serde_json::from_str(full).unwrap();let wire=serde_json::to_value(&c).unwrap();assert_eq!(wire["portable"]["members"],serde_json::json!([]));assert_eq!(wire["portable"]["direct"],serde_json::json!([]));assert_eq!(wire["portable"]["map"],serde_json::json!({}));assert_eq!(wire["portable"]["enabled"],false);assert_eq!(wire["portable"]["count"],0);assert_eq!(wire["portable"]["label"],"");
    let text=toml::to_string(&c).unwrap();let d:Fixture=toml::from_str(&text).unwrap();assert_eq!(c,d);
    let missing:Fixture=toml::from_str("[required]\nlabel='held'\nmembers=[]\n[legacy]\n[portable]\n").unwrap();assert!(missing.portable.members.is_none());
    assert!(serde_json::from_str::<Fixture>(r#"{"required":{"label":null,"members":null},"legacy":{},"portable":{"value":{"kind":"future"}}}"#).is_err());
}
"##;
