//! End-to-end `#embed` expansion over the demo corpus (PROP-035 §7.1) — the
//! real `FsSectionSource` driving FileResolver + DocTree together.

use std::path::{Path, PathBuf};

use vibe_spec::{FileResolver, FsSectionSource, expand_embeds};

fn ws() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ws")
}

fn source() -> FsSectionSource {
    FsSectionSource::new(FileResolver::new(ws(), "vibevm"))
}

#[test]
fn expands_a_host_section_from_the_corpus() {
    let out = expand_embeds("#embed spec://vibevm/common/PROP-000#commits\n", &source()).unwrap();
    assert!(out.contains("Commit rules live here."), "{out}");
    assert!(!out.contains("#embed"));
}

#[test]
fn embeds_a_package_contract_section() {
    let out = expand_embeds(
        "intro\n#embed spec://org.vibevm.demo/demo-lib/contract/API#root\noutro\n",
        &source(),
    )
    .unwrap();
    assert!(out.contains("contract surface of demo-lib"), "{out}");
    assert!(out.contains("intro"));
    assert!(out.contains("outro"));
}
