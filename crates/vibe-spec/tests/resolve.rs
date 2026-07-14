//! End-to-end resolution over the demo fixture corpus (PROP-035 §6, §15).
//!
//! The fixtures under `tests/fixtures/ws` are a throwaway stand-in for a
//! materialised workspace — a host `spec/` tree plus one `vibedeps/` slot —
//! never real packages, so exercising them cannot affect vibevm's own boot.

use std::path::{Path, PathBuf};

use vibe_spec::{DocTree, FileResolver, ResolveError, SpecAddress};

fn ws() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ws")
}

fn resolver() -> FileResolver {
    FileResolver::new(ws(), "vibevm")
}

#[test]
fn resolves_host_prop_inverting_truncation() {
    let addr = SpecAddress::parse("spec://vibevm/modules/demo/PROP-042#root").unwrap();
    let file = resolver().resolve_file(&addr).unwrap();
    assert!(file.ends_with("PROP-042-example-thing.md"), "{file:?}");
}

#[test]
fn resolves_host_plain_doc_end_to_end() {
    // The full chain: address -> file -> tree -> node.
    let addr = SpecAddress::parse("spec://vibevm/common/PROP-000#commits").unwrap();
    let file = resolver().resolve_file(&addr).unwrap();
    let src = std::fs::read_to_string(&file).unwrap();
    let tree = DocTree::parse(&src);
    let node = tree.resolve_path(&addr.anchor).unwrap();
    assert_eq!(tree.node(node).heading, "Commits");
}

#[test]
fn resolves_package_contract_slot() {
    let addr = SpecAddress::parse("spec://org.vibevm.demo/demo-lib/contract/API#root").unwrap();
    let file = resolver().resolve_file(&addr).unwrap();
    assert!(
        file.ends_with(Path::new("contract").join("API.md")),
        "{file:?}"
    );
}

#[test]
fn resolves_package_source_slot() {
    let addr = SpecAddress::parse("spec://org.vibevm.demo/demo-lib/source/impl#root").unwrap();
    let file = resolver().resolve_file(&addr).unwrap();
    assert!(
        file.ends_with(Path::new("source").join("impl.md")),
        "{file:?}"
    );
}

#[test]
fn unknown_host_is_an_error() {
    let addr = SpecAddress::parse("spec://otherproject/x/y#z").unwrap();
    assert!(matches!(
        resolver().resolve_file(&addr),
        Err(ResolveError::UnknownHost { .. })
    ));
}

#[test]
fn missing_package_is_an_error() {
    let addr = SpecAddress::parse("spec://org.vibevm.demo/nonexistent/x#y").unwrap();
    assert!(matches!(
        resolver().resolve_file(&addr),
        Err(ResolveError::PackageSlotNotFound(_))
    ));
}

#[test]
fn missing_doc_is_an_error() {
    let addr = SpecAddress::parse("spec://vibevm/common/PROP-999#root").unwrap();
    assert!(matches!(
        resolver().resolve_file(&addr),
        Err(ResolveError::DocNotFound { .. })
    ));
}
