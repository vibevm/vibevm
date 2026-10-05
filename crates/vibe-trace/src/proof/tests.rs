use super::*;

#[test]
fn saved_join_replacement_during_checker_is_refused() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("proof.json"), b"{}").unwrap();
    let result = stable_saved(root.path(), Path::new("proof.json"), |_| {
        fs::write(root.path().join("replacement.json"), b"{\"forged\":true}").unwrap();
        fs::rename(
            root.path().join("replacement.json"),
            root.path().join("proof.json"),
        )
        .unwrap();
        Ok(())
    });
    assert_eq!(result.unwrap_err().code, "witness_unstable");
}

#[test]
fn unchanged_join_snapshot_returns_its_exact_digest() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("proof.json"), b"{}\n").unwrap();
    let (_, digest) = stable_saved(root.path(), Path::new("proof.json"), |_| Ok(())).unwrap();
    assert_eq!(digest, files::hash(b"{}\n"));
}

pub(super) fn fixture() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/lean-composition");
    for file in [
        "vibe.toml",
        "specmap.toml",
        "proof-policy.toml",
        "lean-proof.toml",
        "Example.lean",
        "rust/Cargo.toml",
        "rust/Cargo.lock",
        "rust/src/lib.rs",
        "vibevm/vibespecs/Protocol.md",
    ] {
        let dest = root.path().join(file);
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::copy(source.join(file), dest).unwrap();
    }
    root
}

fn context(root: &Path) -> Context {
    Context {
        project_root: root.into(),
        lean_toolchain_dir: None,
        rust_toolchain_dir: None,
    }
}
fn request(verify: bool) -> Request {
    let subject = "spec://org.example/same-subject/Protocol#identity".into();
    let witness = "joined-proof.json".into();
    if verify {
        Request::Verify { subject, witness }
    } else {
        Request::Check { subject, witness }
    }
}

#[test]
fn exact_policy_refuses_wrong_subject_grade_dependencies_and_missing_tools() {
    let root = fixture();
    let ctx = context(root.path());
    let reply = execute(
        &ctx,
        Request::Check {
            subject: "spec://org.example/same-subject/Protocol#other".into(),
            witness: "out.json".into(),
        },
    );
    assert_eq!(reply.error.unwrap().code, "subject_mismatch");
    assert_eq!(
        execute(
            &ctx,
            Request::Check {
                subject: "spec://elsewhere/Protocol#identity".into(),
                witness: "out.json".into()
            }
        )
        .error
        .unwrap()
        .code,
        "cross_package_proof_coverage_unsupported"
    );
    let policy = fs::read_to_string(root.path().join("proof-policy.toml")).unwrap();
    fs::write(
        root.path().join("proof-policy.toml"),
        policy.replace("g1_model", "g3_refinement"),
    )
    .unwrap();
    assert_eq!(
        execute(&ctx, request(false)).error.unwrap().code,
        "proof_grade_unsupported"
    );
    fs::write(root.path().join("proof-policy.toml"), &policy).unwrap();
    let manifest = fs::read_to_string(root.path().join("rust/Cargo.toml")).unwrap();
    fs::write(
        root.path().join("rust/Cargo.toml"),
        format!("{manifest}\n[dependencies]\nunused = \"1\"\n"),
    )
    .unwrap();
    assert_eq!(
        execute(&ctx, request(false)).error.unwrap().code,
        "rust_dependency_unsupported"
    );
    fs::write(root.path().join("rust/Cargo.toml"), &manifest).unwrap();
    let mut unavailable = ctx.clone();
    unavailable.rust_toolchain_dir = Some(root.path().join("missing-toolchain"));
    assert_eq!(
        execute(&unavailable, request(false)).error.unwrap().code,
        "rust_toolchain_unavailable"
    );
    assert!(!root.path().join("joined-proof.json").exists());
}

#[test]
fn ambient_compile_inputs_hidden_in_macros_and_attributes_are_refused() {
    let root = fixture();
    let ctx = context(root.path());
    let source = root.path().join("rust/src/lib.rs");
    let original = fs::read_to_string(&source).unwrap();
    for additional in [
        "const X: &str = include_str!(\"../secret\");",
        "macro_rules! hidden { () => { include!(\"../secret\"); }; }",
        "#[cfg_attr(all(), path = \"../secret.rs\")] mod hidden;",
    ] {
        fs::write(&source, format!("{original}\n{additional}\n")).unwrap();
        assert_eq!(
            execute(&ctx, request(false)).error.unwrap().code,
            "rust_source_closure_unsupported"
        );
    }
}

#[test]
#[ignore = "requires pinned Lean 4.34.1 and selected Rust toolchain directories"]
fn real_selected_checks_and_saved_join_recheck() {
    assert!(
        std::env::var_os("LEAN_AI_NATIVE_TOOLCHAIN_DIR").is_some(),
        "explicit real Lean toolchain required"
    );
    assert!(
        std::env::var_os("RUST_LEAN_RUST_TOOLCHAIN_DIR").is_some(),
        "explicit real Rust toolchain required"
    );
    let root = fixture();
    let ctx = context(root.path());
    let checked = execute(&ctx, request(false));
    assert!(checked.ok, "{checked:?}");
    let witness = checked.result.unwrap().witness;
    assert_eq!(witness.rust.tests.len(), 2);
    assert_ne!(witness.rust.execution_id, witness.lean.execution_id);
    assert_eq!(witness.lean.proof.grade, "g1_model");
    assert_eq!(witness.correspondence, "not_established");
    let verified = execute(&ctx, request(true));
    assert!(verified.ok, "{verified:?}");
    assert_eq!(verified.result.unwrap().witness, witness);
}

#[test]
#[ignore = "requires pinned Lean 4.34.1 and selected Rust toolchain directories"]
fn real_selected_join_negative_matrix() {
    assert!(std::env::var_os("LEAN_AI_NATIVE_TOOLCHAIN_DIR").is_some());
    assert!(std::env::var_os("RUST_LEAN_RUST_TOOLCHAIN_DIR").is_some());
    let root = fixture();
    let ctx = context(root.path());
    let baseline = execute(&ctx, request(false));
    assert!(baseline.ok, "{baseline:?}");
    let witness_path = root.path().join("joined-proof.json");
    let baseline_bytes = fs::read(&witness_path).unwrap();
    let refuse = |name: &str, result: Reply, code: &str| {
        assert!(!result.ok, "{name} unexpectedly passed");
        assert_eq!(
            result.error.as_ref().unwrap().code,
            code,
            "{name}: {result:?}"
        );
        assert_eq!(
            fs::read(&witness_path).unwrap(),
            baseline_bytes,
            "{name} replaced the saved join"
        );
        println!("PASS negative {name}: {code}");
    };
    refuse(
        "wrong_subject",
        execute(
            &ctx,
            Request::Verify {
                subject: "spec://other/project/Protocol#identity".into(),
                witness: "joined-proof.json".into(),
            },
        ),
        "witness_subject_mismatch",
    );
    let mut missing = ctx.clone();
    missing.lean_toolchain_dir = Some(root.path().join("absent-lean"));
    refuse(
        "missing_lean_checker",
        execute(&missing, request(false)),
        "lean_toolchain_unavailable",
    );

    // Each mutation has a deterministic guard expectation; failures retain the
    // original output bytes even when an earlier checker has already passed.
    let changes = [
        (
            "rust_failing_test",
            "rust/src/lib.rs",
            "    value\n",
            "    value + 1\n",
            false,
            "rust_checker_failed",
        ),
        (
            "rust_source_drift",
            "rust/src/lib.rs",
            "//! A selected",
            "//! An altered selected",
            true,
            "stale_witness",
        ),
        (
            "rust_missing_exact_test",
            "proof-policy.toml",
            "tests::zero_is_unchanged",
            "tests::does_not_exist",
            false,
            "rust_test_missing",
        ),
        (
            "rust_toolchain_mismatch",
            "proof-policy.toml",
            "c36f1457196e315bc204b9564a6a5a7fe7f5a51f",
            "0000000000000000000000000000000000000000",
            false,
            "rust_toolchain_mismatch",
        ),
        (
            "g3_request",
            "proof-policy.toml",
            "g1_model",
            "g3_refinement",
            false,
            "proof_grade_unsupported",
        ),
        (
            "missing_theorem",
            "lean-proof.toml",
            "Example.identity",
            "Example.absent",
            false,
            "lean_checker_failed",
        ),
        (
            "lean_source_drift",
            "Example.lean",
            "namespace Example",
            "-- changed source snapshot\nnamespace Example",
            true,
            "lean_stale_witness",
        ),
        (
            "lean_failed_checker",
            "Example.lean",
            "n = n := rfl",
            "n = n + 1 := rfl",
            false,
            "lean_checker_failed",
        ),
        (
            "unexpected_axiom",
            "Example.lean",
            "theorem identity (n : Nat) : n = n := rfl",
            "axiom extra (n : Nat) : n = n\ntheorem identity (n : Nat) : n = n := extra n",
            false,
            "lean_unexpected_axiom",
        ),
        (
            "stale_spec_revision",
            "vibevm/vibespecs/Protocol.md",
            "req r1",
            "req r2",
            true,
            "composition_incomplete",
        ),
    ];
    for (name, file, from, to, verify, code) in changes {
        let path = root.path().join(file);
        let original = fs::read_to_string(&path).unwrap();
        assert!(original.contains(from), "fixture mutation {name} must hit");
        fs::write(&path, original.replace(from, to)).unwrap();
        refuse(name, execute(&ctx, request(verify)), code);
        fs::write(path, original).unwrap();
    }
    let manifest = root.path().join("rust/Cargo.toml");
    let original = fs::read_to_string(&manifest).unwrap();
    fs::write(
        &manifest,
        format!("{original}\n[dependencies]\nadded=\"1\"\n"),
    )
    .unwrap();
    refuse(
        "dependency_drift",
        execute(&ctx, request(true)),
        "rust_dependency_unsupported",
    );
    fs::write(&manifest, original).unwrap();
    fs::write(
        root.path().join("rust/src/undeclared.rs"),
        "pub fn extra() {}\n",
    )
    .unwrap();
    refuse(
        "undeclared_source",
        execute(&ctx, request(false)),
        "rust_source_closure_mismatch",
    );
    fs::remove_file(root.path().join("rust/src/undeclared.rs")).unwrap();

    for (name, pointer, replacement, code) in [
        (
            "forged_test_outcome",
            "/rust/tests/0/outcome",
            serde_json::json!("forged-pass"),
            "stale_witness",
        ),
        (
            "forged_toolchain_digest",
            "/rust/toolchain/bin_lib_sha256",
            serde_json::json!("sha256:forged"),
            "stale_witness",
        ),
        (
            "axiom_assumption_drift",
            "/lean/proof/allowed_axioms",
            serde_json::json!(["Classical.choice"]),
            "lean_stale_witness",
        ),
        (
            "forged_grade",
            "/grade",
            serde_json::json!("g3_refinement"),
            "witness_subject_mismatch",
        ),
    ] {
        let mut forged: serde_json::Value = serde_json::from_slice(&baseline_bytes).unwrap();
        *forged.pointer_mut(pointer).unwrap() = replacement;
        let bytes = serde_json::to_vec(&forged).unwrap();
        fs::write(&witness_path, &bytes).unwrap();
        let result = execute(&ctx, request(true));
        assert!(!result.ok, "{name} unexpectedly passed");
        assert_eq!(result.error.unwrap().code, code, "{name}");
        assert_eq!(fs::read(&witness_path).unwrap(), bytes);
        fs::write(&witness_path, &baseline_bytes).unwrap();
        println!("PASS negative {name}: {code}");
    }
}
