# ADV-SDD-CHANGE-WRITE-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe_requirements::change::capture_origin` returns the locked origin pin and the slot-record sha256 captures of the slot's vibe.toml and of the file a spec:// address resolves to, and refuses a package that is not locked with `OriginCaptureError::NotInstalled`.

**Write.** `crates/vibe-requirements/src/change/mod.rs`, `crates/vibe-requirements/src/change/capture.rs`, `crates/vibe-requirements/src/change/capture_cases.rs`

**Check.** `cargo test -p vibe-requirements --lib change::capture_cases 2>&1 | grep -F "test result: ok. 2 passed"`

**Test.** crates/vibe-requirements/src/change/capture_cases.rs (new module, declared `#[cfg(test)] mod capture_cases;` in change/mod.rs): `capture_reads_locked_pin_and_slot_hashes` and `capture_refuses_a_package_that_is_not_locked`.

**Read first.**
- crates/vibe-requirements/src/change/read.rs (read_saved_change; SavedChangeError shows the `violates REQ ... fix surface:` message style)
- crates/vibe-requirements/src/sources/installed.rs (fn observe: lock row -> slot path -> FileResolver::with_selected_world, the pattern this entry copies)
- crates/vibe-workspace/src/vibedeps/slot_record.rs (read_slot_record, write_slot_record, SlotRecord, SlotFile, sha256_bytes, SLOT_RECORD_SCHEMA)
- crates/vibe-workspace/src/vibedeps.rs (slot_abs_path)
- crates/vibe-spec/src/resolver.rs (SelfCoordinate, SelectedPackage::new, FileResolver::with_selected_world, FileResolver::resolve_file)
- crates/vibe-core/src/manifest/lockfile.rs (Lockfile::FILENAME, Lockfile::find, LockedPackage fields kind/name/group/version/materialization, LockedPackage::as_package_ref)

**Steps.**
1. Create crates/vibe-requirements/src/change/capture.rs. Line 1: `//! Origin captures read from the installed world; never fetched.` Line 2: `specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-AUTHORED-GRAMMAR");`. Imports: `std::collections::BTreeMap`, `std::path::Path`, `vibe_core::manifest::Lockfile`, `vibe_spec::{Authority, FileResolver, SelectedPackage, SelfCoordinate, SpecAddress}`, `vibe_workspace::vibedeps::{SlotRecord, read_slot_record, slot_abs_path}`.
2. In capture.rs add `#[derive(Debug, Clone, PartialEq, Eq)] pub struct OriginCapture { pub package: String, pub manifest_sha256: String, pub document_sha256: String }` and `#[derive(Debug, thiserror::Error)] pub enum OriginCaptureError` with exactly three variants: (a) `Address { address: String, reason: &'static str }` with `#[error("violates REQ spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-AUTHORED-GRAMMAR: origin address {address} refused: {reason}; fix surface: the cited spec address")]`; (b) `NotInstalled { coordinate: String }` with `#[error("violates REQ spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-AUTHORED-GRAMMAR: {coordinate} is not locked in this project; fix surface: vibe install the origin package")]`; (c) `Unavailable(String)` with `#[error("violates REQ spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-AUTHORED-GRAMMAR: installed origin unreadable: {0}; fix surface: reinstall the origin package")]`.
3. Add a private helper `fn row(record: &SlotRecord, path: &str) -> Result<String, OriginCaptureError>` that finds the `record.files` entry whose `path == path` and returns `format!("sha256:{}", file.sha256)`, or `Err(OriginCaptureError::Unavailable(format!("slot record has no row for {path}")))`.
4. Add `#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-AUTHORED-GRAMMAR")] pub fn capture_origin(world_root: &Path, consumer: &SelfCoordinate, address: &str) -> Result<OriginCapture, OriginCaptureError>`. Let `refuse = |reason| OriginCaptureError::Address { address: address.to_string(), reason }`. Body in this order: 1) `let addr = SpecAddress::parse(address).map_err(|_| refuse("not a spec:// address"))?;` 2) match `&addr.authority`: only `Authority::Package { group, name, version: None }` continues; every other authority returns `refuse("must name group/name without a version")`. 3) If `consumer.group.as_deref() == Some(group.as_str()) && consumer.name == *name` return `refuse("names this project, not an installed dependency")`. 4) `let group = vibe_core::Group::parse(group).map_err(|_| refuse("invalid group"))?;` and `let coordinate = format!("{group}/{name}");`. 5) `let lock_path = world_root.join(Lockfile::FILENAME);` if `!lock_path.is_file()` return `NotInstalled { coordinate }`; `let lock = Lockfile::read(&lock_path).map_err(|e| OriginCaptureError::Unavailable(e.to_string()))?;` `let p = lock.find(&group, name).ok_or_else(|| OriginCaptureError::NotInstalled { coordinate: coordinate.clone() })?;`. 6) If `p.materialization.is_in_place()` return `Unavailable(format!("{coordinate} is installed in place and has no slot record"))`. Otherwise `let slot = slot_abs_path(world_root, &p.group, p.name.as_str(), &p.version);`.
5. Continue capture_origin: 7) `let record = read_slot_record(&slot).map_err(OriginCaptureError::Unavailable)?;` 8) Build `let selected = BTreeMap::from([((p.group.to_string(), p.name.as_str().to_string()), SelectedPackage::new(p.version.to_string(), slot.clone()))]);` and `let file = FileResolver::with_selected_world(world_root, world_root, selected, consumer.clone()).resolve_file(&addr).map_err(|_| refuse("does not resolve in the installed world"))?;` 9) `let relative = file.strip_prefix(&slot).map_err(|_| refuse("resolves outside the locked slot"))?;` then `let relative = vibe_core::machine_json_path(relative);` 10) `let manifest_sha256 = row(&record, "vibe.toml")?;` `let document_sha256 = row(&record, &relative)?;` 11) `let package = p.as_package_ref().map_err(|e| OriginCaptureError::Unavailable(e.to_string()))?.to_string();` Return `Ok(OriginCapture { package, manifest_sha256, document_sha256 })`.
6. Give capture_origin this doc example (vibe-requirements already has tempfile as a dev-dependency): ```
use vibe_requirements::change::{OriginCaptureError, capture_origin};
let root = tempfile::tempdir().unwrap();
let consumer = vibe_spec::SelfCoordinate::new(Some("org.consumer".into()), "demo".into());
let result = capture_origin(root.path(), &consumer, "spec://org.example/payments/FEAT-004#DELIVERY");
assert!(matches!(result, Err(OriginCaptureError::NotInstalled { .. })));
```
7. In change/mod.rs append three lines: `mod capture;`, `pub use capture::{OriginCapture, OriginCaptureError, capture_origin};` and, on the line after `#[cfg(test)]`, `mod capture_cases;` (the file keeps its other lines unchanged).
8. Create capture_cases.rs starting with `use super::*;` and imports `std::fs`, `vibe_core::layout::{current_specs_root, current_vibedeps_root}`, `vibe_core::manifest::SpecFormat`, `vibe_workspace::vibedeps::{SLOT_RECORD_SCHEMA, SlotFile, SlotRecord, sha256_bytes, write_slot_record}`. Add `const LOCK: &str` holding exactly the lock text in decisions, `const MANIFEST: &str` = the vibe.toml text in decisions and `const SPEC: &str` = the FEAT-004.md text in decisions. A helper `fn world() -> tempfile::TempDir` creates a tempdir, writes `LOCK` to `vibe.lock`; computes `slot = root.join(current_vibedeps_root()).join("org.example.payments").join("1.0.0")`; creates `slot.join(current_specs_root())`; writes `MANIFEST` to `slot/vibe.toml` and `SPEC` to `slot.join(current_specs_root()).join("FEAT-004.md")`; then calls `write_slot_record(&slot, &SlotRecord { schema: SLOT_RECORD_SCHEMA, source_hash: vibe_core::ContentHash::parse(&format!("sha256:{}", "0".repeat(64))).unwrap(), spec_format: SpecFormat::Mixed, converter_recipe: None, derived_hash: None, overlay_hash: None, files: vec![SlotFile { path: "vibe.toml".into(), sha256: sha256_bytes(MANIFEST.as_bytes()), source: None, disposition: None }, SlotFile { path: vibe_core::machine_json_path(&current_specs_root().join("FEAT-004.md")), sha256: sha256_bytes(SPEC.as_bytes()), source: None, disposition: None }] }).unwrap()` (rows in this order: the validator requires strictly sorted paths and `vibe.toml` sorts before `vibevm/...`).
9. In capture_cases.rs: `consumer()` returns `SelfCoordinate::new(Some("org.consumer".into()), "demo".into())` (import `vibe_spec::SelfCoordinate`). `#[test] fn capture_reads_locked_pin_and_slot_hashes`: `let w = world(); let c = capture_origin(w.path(), &consumer(), "spec://org.example/payments/FEAT-004#DELIVERY").unwrap();` assert `c.package == "feat:org.example/payments@=1.0.0"`, `c.manifest_sha256 == format!("sha256:{}", sha256_bytes(MANIFEST.as_bytes()))` and `c.document_sha256 == format!("sha256:{}", sha256_bytes(SPEC.as_bytes()))`. `#[test] fn capture_refuses_a_package_that_is_not_locked`: with the same world, `capture_origin(w.path(), &consumer(), "spec://org.example/other/FEAT-004#DELIVERY")` matches `Err(OriginCaptureError::NotInstalled { .. })`. Then run the check.

**Decisions already made.**
- Lock fixture text (`LOCK`), exact bytes, every line ending in a newline:
```
[meta]
generated_by = "vibe 1.0.12"
generated_at = "2026-11-22T13:34:20Z"
schema_version = 7
root_dependencies = ["org.example/payments@=1.0.0"]

[[package]]
kind = "feat"
name = "payments"
group = "org.example"
version = "1.0.0"
source_url = "file:///fixture/payments"
content_hash = "sha256:c8301bd4a5915b8c2ef9982582eac2153ef28c84a64b4b887d7803b4c07c5ed8"
files_written = []
admitted_by = "root-edge"
source_kind = "registry"
```
The content_hash value is an arbitrary valid digest; nothing in this entry compares it with anything.
- `MANIFEST` is exactly:
```
[package]
group='org.example'
name='payments'
kind='feat'
version='1.0.0'
```
(with a final newline). `SPEC` is exactly:
```
# Payments

@fact:DELIVERY Payouts version 1.0.0. @status:spec/done
```
(with a final newline).
- Hash meaning: manifest_sha256 is `sha256:` + the slot-record row for `vibe.toml`; document_sha256 is `sha256:` + the slot-record row of the resolved file. Both come from the record and are never recomputed from disk.
- world_root is the directory holding vibe.lock and the vibedeps tree (the workspace root), not the selected node root.
- Resolution uses FileResolver::with_selected_world with exactly the one locked package, so the locked version's slot is read even if another version directory exists. In-place packages are refused with Unavailable because they carry no slot record.
- No new spec unit and no new anchor: every marker cites the existing CHANGE-AUTHORED-GRAMMAR anchor. No generated file, wire type, codegen or specmap step is involved.
- capture.rs and capture_cases.rs are new files, each well under the 600-line budget; change/mod.rs is 11 lines and grows by 4.
- Clippy -D warnings applies. Do not add #[allow]. Remove any import left unused.

**Not in this entry.** Not in this entry: checking that the cited anchor exists inside the document, reading other versions from a registry cache or the network, any CLI wiring, and drift reporting.

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
