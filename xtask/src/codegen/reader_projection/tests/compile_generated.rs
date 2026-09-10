use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use super::Fixture;

fn include_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "\\\\")
}

fn rlib_candidates(deps: &Path, stem: &str) -> Result<Vec<PathBuf>> {
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
    candidates.sort();
    anyhow::ensure!(
        !candidates.is_empty(),
        "finding {prefix}*.rlib under {}",
        deps.display()
    );
    Ok(candidates)
}

pub(super) fn compile_and_run_generated(fixture: &Fixture) -> Result<()> {
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
    let serde_candidates = rlib_candidates(&deps, "serde")?;
    let serde_json_candidates = rlib_candidates(&deps, "serde_json")?;
    let executable = fixture.root.join(if cfg!(windows) {
        "projection_fixture.exe"
    } else {
        "projection_fixture"
    });
    // Cargo can leave several feature variants in `deps`; serde and serde_json
    // must agree on the exact serde_core build whose traits cross this boundary.
    let mut compiled = false;
    let mut last_failure = None;
    'serde: for serde in &serde_candidates {
        for serde_json in &serde_json_candidates {
            let output = Command::new("rustc")
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
                .output()
                .context("compiling the generated projection fixture")?;
            if output.status.success() {
                compiled = true;
                break 'serde;
            }
            last_failure = Some((serde, serde_json, output));
        }
    }
    if !compiled {
        let (serde, serde_json, output) = last_failure
            .context("no serde/serde_json artifact pair was available for the generated fixture")?;
        anyhow::bail!(
            "no compatible serde/serde_json artifact pair compiled the generated projection fixture (tried {} pair(s)); last pair: {} + {}\n{}",
            serde_candidates.len() * serde_json_candidates.len(),
            serde.display(),
            serde_json.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let status = Command::new(&executable)
        .status()
        .context("running the generated projection fixture")?;
    anyhow::ensure!(
        status.success(),
        "generated projection fixture assertions failed"
    );
    Ok(())
}
