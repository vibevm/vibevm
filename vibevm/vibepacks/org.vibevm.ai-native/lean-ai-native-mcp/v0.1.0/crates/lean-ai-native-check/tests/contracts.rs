use lean_ai_native_check::{Context, Reply, Request, execute};
use std::{fs, path::PathBuf};

const SUBJECT: &str = "spec://org.example/demo/Protocol#identity";
const SOURCE: &str =
    "import Std\nnamespace Example\ntheorem identity (n : Nat) : n = n := rfl\nend Example\n";
const SPEC: &str = "# Protocol {#root}\n\n## Identity {#identity}\n\n`req r1`\n\nNatural-number reflexivity in the selected model.\n";
const CARRIER: &str = r#"schema = 1
namespace = "org.example/demo"
toolchain = "leanprover/lean4:v4.34.1"
sources = ["Example.lean"]
allowed_axioms = []
[[proof]]
subject = "spec://org.example/demo/Protocol#identity"
spec_file = "vibevm/vibespecs/Protocol.md"
spec_revision = 1
module = "Example"
theorem = "Example.identity"
grade = "g1_model"
"#;

fn fixture(toolchain: Option<PathBuf>) -> (tempfile::TempDir, Context) {
    let dir = tempfile::tempdir().expect("fixture directory");
    fs::create_dir_all(dir.path().join("vibevm/vibespecs")).expect("spec directory");
    fs::write(dir.path().join("Example.lean"), SOURCE).expect("source");
    fs::write(dir.path().join("vibevm/vibespecs/Protocol.md"), SPEC).expect("spec");
    fs::write(dir.path().join("lean-proof.toml"), CARRIER).expect("carrier");
    let context = Context {
        project_root: dir.path().to_path_buf(),
        toolchain_dir: toolchain,
    };
    (dir, context)
}
fn check(context: &Context) -> Reply {
    execute(
        context,
        Request::Check {
            carrier: "lean-proof.toml".into(),
            subject: SUBJECT.into(),
        },
    )
}
fn verify(context: &Context, subject: &str) -> Reply {
    execute(
        context,
        Request::VerifyWitness {
            carrier: "lean-proof.toml".into(),
            subject: subject.into(),
            witness: "proof.json".into(),
        },
    )
}
fn failure(reply: Reply, code: &str) {
    assert!(!reply.ok, "unexpected success: {reply:?}");
    assert!(
        reply.result.is_none(),
        "failure must not carry a proof result"
    );
    assert_eq!(reply.error.expect("diagnostic").code, code);
}

#[test]
fn scan_is_declared_only_and_binds_actual_subject_revision() {
    let (_dir, context) = fixture(None);
    let reply = execute(
        &context,
        Request::Scan {
            carrier: "lean-proof.toml".into(),
        },
    );
    assert!(reply.ok, "{reply:?}");
    let value = reply.result.expect("inventory");
    assert_eq!(value["status"], "declared");
    assert_eq!(value["claims"][0]["subject"], SUBJECT);
    assert_eq!(value["claims"][0]["spec"]["revision"], 1);
    assert!(value.get("outcome").is_none());
}

#[test]
fn malformed_carrier_and_unknown_members_refuse_without_toolchain() {
    let (dir, context) = fixture(None);
    fs::write(dir.path().join("lean-proof.toml"), "schema = [").expect("malformed carrier");
    failure(check(&context), "malformed_carrier");
    fs::write(
        dir.path().join("lean-proof.toml"),
        format!("unexpected = true\n{CARRIER}"),
    )
    .expect("unknown member");
    failure(check(&context), "malformed_carrier");
}

#[test]
fn wrong_subject_revision_grade_and_path_refuse_before_checker() {
    let (dir, context) = fixture(None);
    failure(
        execute(
            &context,
            Request::Check {
                carrier: "lean-proof.toml".into(),
                subject: format!("{SUBJECT}-other"),
            },
        ),
        "unknown_subject",
    );
    fs::write(
        dir.path().join("lean-proof.toml"),
        CARRIER.replace("spec_revision = 1", "spec_revision = 2"),
    )
    .expect("revision");
    failure(check(&context), "spec_revision_mismatch");
    fs::write(
        dir.path().join("lean-proof.toml"),
        CARRIER.replace("g1_model", "g3_implementation"),
    )
    .expect("grade");
    failure(check(&context), "unsupported_grade");
    failure(
        execute(
            &context,
            Request::Scan {
                carrier: "../lean-proof.toml".into(),
            },
        ),
        "invalid_path",
    );
}

#[test]
fn missing_checker_is_not_an_empty_success() {
    let (dir, mut context) = fixture(None);
    context.toolchain_dir = Some(dir.path().join("absent-toolchain"));
    failure(check(&context), "toolchain_unavailable");
}

#[cfg(unix)]
#[test]
fn mismatched_toolchain_fails_before_proof_execution() {
    use std::os::unix::fs::PermissionsExt;
    let toolchain = tempfile::tempdir().expect("toolchain fixture");
    fs::create_dir(toolchain.path().join("bin")).expect("bin");
    let binary = toolchain.path().join("bin/lean");
    fs::write(
        &binary,
        "#!/bin/sh\nprintf 'Lean (version 4.0.0, test)\\n'\n",
    )
    .expect("version probe");
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).expect("executable");
    let (_dir, context) = fixture(Some(toolchain.path().to_path_buf()));
    failure(check(&context), "toolchain_mismatch");
}

#[test]
fn arbitrary_axiom_policy_and_module_shadowing_are_refused() {
    let (dir, context) = fixture(None);
    fs::write(
        dir.path().join("lean-proof.toml"),
        CARRIER.replace("allowed_axioms = []", "allowed_axioms = [\"sorryAx\"]"),
    )
    .expect("policy");
    failure(check(&context), "axiom_policy");
    fs::write(
        dir.path().join("lean-proof.toml"),
        CARRIER.replace("Example.lean", "Lean.lean"),
    )
    .expect("shadow");
    fs::write(dir.path().join("Lean.lean"), SOURCE).expect("shadow source");
    failure(check(&context), "malformed_carrier");
}

/// Explicit integration gate. This must be run with the pinned distribution;
/// absence fails visibly rather than skipping or substituting a fake checker.
#[test]
fn pinned_lean_success_reproducibility_and_negative_proof_contracts() {
    let toolchain = std::env::var_os("LEAN_AI_NATIVE_TEST_TOOLCHAIN")
        .map(PathBuf::from)
        .expect("set LEAN_AI_NATIVE_TEST_TOOLCHAIN to the pinned Lean 4.34.1 distribution root");
    let (dir, context) = fixture(Some(toolchain));
    let first = check(&context);
    assert!(first.ok, "{first:?}");
    let proof = first.result.clone().expect("witness");
    assert_eq!(proof["grade"], "g1_model");
    assert!(
        !proof["compiled_modules"]
            .as_array()
            .expect("compiled object bindings")
            .is_empty()
    );
    assert_eq!(proof["observed_axioms"], serde_json::json!([]));
    assert_eq!(proof["toolchain"]["identifier"], "leanprover/lean4:v4.34.1");
    assert_eq!(
        check(&context),
        first,
        "same content/toolchain yields identical witness"
    );
    fs::write(
        dir.path().join("proof.json"),
        serde_json::to_vec(&proof).expect("serialize"),
    )
    .expect("save");
    assert!(verify(&context, SUBJECT).ok, "fresh saved proof must match");
    failure(
        verify(&context, "spec://org.example/demo/Protocol#other"),
        "witness_subject_mismatch",
    );
    let mut forged = proof.clone();
    forged["statement"] = serde_json::json!("False");
    fs::write(
        dir.path().join("proof.json"),
        serde_json::to_vec(&forged).expect("serialize forged claim"),
    )
    .expect("forged claim");
    failure(verify(&context, SUBJECT), "stale_witness");
    fs::write(
        dir.path().join("proof.json"),
        serde_json::to_vec(&proof).expect("serialize"),
    )
    .expect("restore witness");

    // The theorem statement changes but stays provable; the old witness is stale.
    fs::write(
        dir.path().join("Example.lean"),
        SOURCE.replace("n = n", "n = n + 0"),
    )
    .expect("changed theorem");
    failure(verify(&context, SUBJECT), "stale_witness");
    fs::write(dir.path().join("Example.lean"), SOURCE).expect("restore source");
    fs::write(
        dir.path().join("vibevm/vibespecs/Protocol.md"),
        format!("{SPEC}\nA changed requirement.\n"),
    )
    .expect("changed spec");
    failure(verify(&context, SUBJECT), "stale_witness");
    fs::write(dir.path().join("vibevm/vibespecs/Protocol.md"), SPEC).expect("restore spec");

    fs::write(
        dir.path().join("Example.lean"),
        SOURCE.replace("theorem identity", "theorem different"),
    )
    .expect("missing theorem");
    failure(check(&context), "checker_failed");
    fs::write(
        dir.path().join("Example.lean"),
        "namespace Example\ntheorem identity : False := by rfl\nend Example\n",
    )
    .expect("failed proof");
    failure(check(&context), "checker_failed");
    fs::write(dir.path().join("Example.lean"), "namespace Example\naxiom untrusted : False\ntheorem identity : False := untrusted\nend Example\n").expect("unexpected axiom");
    failure(check(&context), "unexpected_axiom");
    fs::write(
        dir.path().join("Example.lean"),
        "namespace Example\ntheorem identity : False := by sorry\nend Example\n",
    )
    .expect("incomplete proof");
    failure(check(&context), "unexpected_axiom");
}

#[test]
fn pinned_lean_binds_local_dependency_closure() {
    let toolchain = std::env::var_os("LEAN_AI_NATIVE_TEST_TOOLCHAIN")
        .map(PathBuf::from)
        .expect("set LEAN_AI_NATIVE_TEST_TOOLCHAIN to pinned Lean 4.34.1");
    let (dir, context) = fixture(Some(toolchain));
    let helper = "namespace Helper\ntheorem identity (n : Nat) : n = n := rfl\nend Helper\n";
    fs::write(dir.path().join("Helper.lean"), helper).expect("helper");
    fs::write(dir.path().join("Example.lean"), "import Helper\nnamespace Example\ntheorem identity (n : Nat) : n = n := Helper.identity n\nend Example\n").expect("dependent source");
    fs::write(
        dir.path().join("lean-proof.toml"),
        CARRIER.replace(
            "sources = [\"Example.lean\"]",
            "sources = [\"Helper.lean\", \"Example.lean\"]",
        ),
    )
    .expect("closure");
    let reply = check(&context);
    assert!(reply.ok, "{reply:?}");
    let witness = reply.result.expect("proof");
    assert_eq!(
        witness["sources"]
            .as_array()
            .expect("source bindings")
            .len(),
        2
    );
    fs::write(
        dir.path().join("proof.json"),
        serde_json::to_vec(&witness).expect("serialize"),
    )
    .expect("save");
    fs::write(
        dir.path().join("Helper.lean"),
        format!("{helper}\n-- changed dependency bytes\n"),
    )
    .expect("changed helper");
    failure(verify(&context, SUBJECT), "stale_witness");
    // No previous local .olean or ambient project path may fill an omitted dependency.
    fs::write(dir.path().join("lean-proof.toml"), CARRIER).expect("omitted helper");
    failure(check(&context), "checker_failed");
}
