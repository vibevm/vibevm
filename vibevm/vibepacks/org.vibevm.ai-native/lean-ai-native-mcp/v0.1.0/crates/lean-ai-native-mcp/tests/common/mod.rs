//! Minimal named theorem and versioned spec subject shared by protocol tests.

pub const SUBJECT: &str = "spec://org.example/demo/Protocol#identity";

pub fn consumer() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("vibevm/vibespecs")).unwrap();
    std::fs::write(
        root.path().join("Example.lean"),
        "namespace Example\ntheorem identity (n : Nat) : n = n := rfl\nend Example\n",
    )
    .unwrap();
    std::fs::write(
        root.path().join("vibevm/vibespecs/Protocol.md"),
        "# Protocol {#root}\n\n## Identity {#identity}\n\n`req r1`\n\nEvery natural number equals itself.\n",
    ).unwrap();
    std::fs::write(
        root.path().join("lean-proof.toml"),
        r#"schema = 1
namespace = "org.example/demo"
toolchain = "leanprover/lean4:v4.34.1"
sources = ["Example.lean"]
allowed_axioms = []
timeout_seconds = 30

[[proof]]
subject = "spec://org.example/demo/Protocol#identity"
spec_file = "vibevm/vibespecs/Protocol.md"
spec_revision = 1
module = "Example"
theorem = "Example.identity"
grade = "g1_model"
"#,
    )
    .unwrap();
    root
}
