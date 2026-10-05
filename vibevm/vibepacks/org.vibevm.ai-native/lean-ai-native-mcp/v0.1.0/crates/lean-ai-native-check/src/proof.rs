use crate::{
    Context, Diagnostic, LEAN_COMMIT, ProofWitness, TOOLCHAIN, ToolchainBinding, WITNESS_SCHEMA,
    carrier::Capture, files, process,
};
use serde::Deserialize;
use std::{fs, path::PathBuf};

const INSPECTOR: &str = include_str!("inspector.lean");

fn checker_source_hash() -> String {
    let files = [
        include_str!("lib.rs"),
        include_str!("wire.rs"),
        include_str!("carrier.rs"),
        include_str!("files.rs"),
        include_str!("process.rs"),
        include_str!("proof.rs"),
        INSPECTOR,
        include_str!("../Cargo.toml"),
    ];
    let mut bytes = b"lean-ai-native:checker-source:v1\0".to_vec();
    for file in files {
        bytes.extend_from_slice(&(file.len() as u64).to_be_bytes());
        bytes.extend_from_slice(file.as_bytes());
    }
    files::hash(&bytes)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Inspection {
    theorem: String,
    statement: String,
    axioms: Vec<String>,
}

fn compiled_bindings(
    root: &std::path::Path,
    captured: &Capture,
) -> Result<Vec<crate::FileBinding>, Diagnostic> {
    let mut result = Vec::new();
    for source in &captured.carrier.sources {
        let base = source.trim_end_matches(".lean");
        for suffix in [".olean", ".olean.server", ".olean.private"] {
            let relative = format!("{base}{suffix}");
            if suffix == ".olean" || root.join(&relative).exists() {
                result.push(files::binding(
                    &relative,
                    &files::read_relative(root, std::path::Path::new(&relative))?,
                ));
            }
        }
    }
    result.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(result)
}

#[specmark::spec(
    implements = "spec://org.vibevm.ai-native/lean-ai-native-lang/mechanisms/LEAN-PROOF-RUNNER#RUNNER-CHECK"
)]
pub(crate) fn check(
    context: &Context,
    captured: &Capture,
    subject: &str,
) -> Result<ProofWitness, Diagnostic> {
    let selected = captured.selected(subject)?;
    let toolchain = context.toolchain_dir.clone().or_else(|| std::env::var_os("LEAN_AI_NATIVE_TOOLCHAIN_DIR").map(PathBuf::from))
        .ok_or_else(|| Diagnostic::new("toolchain_unavailable", "set a trusted --toolchain-dir or LEAN_AI_NATIVE_TOOLCHAIN_DIR for pinned Lean 4.34.1"))?
        .canonicalize().map_err(|e| Diagnostic::new("toolchain_unavailable", e.to_string()))?;
    let scratch =
        tempfile::tempdir().map_err(|e| Diagnostic::new("scratch_unavailable", e.to_string()))?;
    let root = scratch.path();
    let timeout = captured.carrier.timeout_seconds;
    let version = process::run(&toolchain, root, "lean", &["--version".into()], timeout)?
        .trim()
        .to_owned();
    let commit = process::run(&toolchain, root, "lean", &["--githash".into()], timeout)?
        .trim()
        .to_owned();
    if !version.starts_with("Lean (version 4.34.1,") || commit != LEAN_COMMIT {
        return Err(Diagnostic::new(
            "toolchain_mismatch",
            "the checker requires the pinned Lean 4.34.1 release commit",
        ));
    }
    let toolchain_hash = files::distribution_hash(&toolchain)?;
    for source in &captured.carrier.sources {
        let target = root.join(source);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| Diagnostic::new("scratch_unavailable", e.to_string()))?;
        }
        fs::write(&target, &captured.files[source])
            .map_err(|e| Diagnostic::new("scratch_unavailable", e.to_string()))?;
    }
    for source in &captured.carrier.sources {
        let object = source
            .strip_suffix(".lean")
            .map(|s| format!("{s}.olean"))
            .ok_or_else(|| Diagnostic::new("malformed_carrier", "invalid source suffix"))?;
        process::run(
            &toolchain,
            root,
            "lean",
            &[
                "-j1".into(),
                "-M1024".into(),
                "-o".into(),
                object,
                source.clone(),
            ],
            timeout,
        )?;
    }
    // Bind generated definitions as well as authored bytes and the selected
    // statement; a constant name alone does not describe its elaborated body.
    let compiled_modules = compiled_bindings(root, captured)?;
    let inspector_file = root.join("LeanAiNativeInspector.lean");
    fs::write(&inspector_file, INSPECTOR)
        .map_err(|e| Diagnostic::new("scratch_unavailable", e.to_string()))?;
    process::run(
        &toolchain,
        root,
        "lean",
        &[
            "-j1".into(),
            "-M4096".into(),
            "--run".into(),
            "LeanAiNativeInspector.lean".into(),
            selected.module.clone(),
            selected.theorem.clone(),
            "inspection.json".into(),
        ],
        timeout,
    )?;
    let inspection_bytes = files::read_relative(root, std::path::Path::new("inspection.json"))?;
    let inspection: Inspection = serde_json::from_slice(&inspection_bytes)
        .map_err(|e| Diagnostic::new("invalid_checker_output", e.to_string()))?;
    if inspection.theorem != selected.theorem || inspection.statement.is_empty() {
        return Err(Diagnostic::new(
            "invalid_checker_output",
            "checker did not identify the exact selected theorem",
        ));
    }
    if inspection
        .axioms
        .iter()
        .any(|a| !captured.carrier.allowed_axioms.contains(a))
    {
        return Err(Diagnostic::new(
            "unexpected_axiom",
            format!(
                "observed transitive axioms {:?} are not admitted by policy {:?}",
                inspection.axioms, captured.carrier.allowed_axioms
            ),
        ));
    }
    // Replay every local module in the declared closure, including imported
    // helpers, through Lean's bundled kernel (not an independent checker).
    let mut replay_args = vec!["-v".into()];
    for source in &captured.carrier.sources {
        replay_args.push(source.trim_end_matches(".lean").replace('/', "."));
    }
    process::run(&toolchain, root, "leanchecker", &replay_args, timeout)?;
    if compiled_bindings(root, captured)? != compiled_modules {
        return Err(Diagnostic::new(
            "compiled_input_unstable",
            "compiled modules changed during inspection or kernel replay",
        ));
    }
    if files::distribution_hash(&toolchain)? != toolchain_hash {
        return Err(Diagnostic::new(
            "toolchain_unstable",
            "toolchain bytes changed while checking",
        ));
    }
    let mut allowed = captured.carrier.allowed_axioms.clone();
    allowed.sort();
    let mut sources = captured.sources();
    sources.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(ProofWitness {
        schema:WITNESS_SCHEMA.into(), grade:"g1_model".into(), outcome:"passed".into(), subject:subject.into(), spec:captured.specs[subject].clone(),
        module:selected.module.clone(), theorem:inspection.theorem, statement_sha256:files::hash(inspection.statement.as_bytes()), statement:inspection.statement,
        carrier_sha256:captured.carrier_sha256.clone(), sources, compiled_modules, allowed_axioms:allowed, observed_axioms:inspection.axioms,
        toolchain:ToolchainBinding { identifier:TOOLCHAIN.into(), version, commit, bin_lib_sha256:toolchain_hash },
        runner_version:env!("CARGO_PKG_VERSION").into(), checker_source_sha256:checker_source_hash(), inspector_sha256:files::hash(INSPECTOR.as_bytes()),
        assurance:"G1 checked declared theorem/model only; no Rust refinement or semantic equivalence to prose; trusted local source/toolchain; bundled kernel replay trusts imported distribution".into(),
    })
}
