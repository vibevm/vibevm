# ADV-FIX-CONVFB-1

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** A transformed (spec_format xml or markdown) legacy materialisation reports each opposite-format spec document it kept unconverted as a typed ConversionFallback row on MaterialiseReport, InstallOutcome and SubtreeOutcome.

**Write.** `crates/vibe-workspace/src/vibedeps/derived.rs`, `crates/vibe-workspace/src/vibedeps/derived/materialise.rs`, `crates/vibe-workspace/src/vibedeps/slot_diff/report.rs`, `crates/vibe-workspace/src/install/model.rs`, `crates/vibe-workspace/src/install.rs`, `crates/vibe-workspace/src/install/lifecycle_apply.rs`, `crates/vibe-workspace/src/install/subtree.rs`, `crates/vibe-workspace/src/vibedeps/tests_transformed.rs`, `specmap.json`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-workspace --lib xml_conversion_fallback_reports_unconverted_markdown`

**Test.** crates/vibe-workspace/src/vibedeps/tests_transformed.rs, new #[test] fn xml_conversion_fallback_reports_unconverted_markdown. Source tempdir (no vibe.toml) holds crate::layout_paths::specs_path("PROP-001.md") = "# H {#h}\n\n@fact:DUP one\n\n@fact:DUP two\n" (vibe_specdoc::from_markdown rejects this; see vibe-specdoc md_in_tests source_markup_errors_are_loud) and specs_path("guide.md") = "# Guide\n\nText.\n". It calls materialise_with_spec_format_report(workspace, &group("org.acme"), "pkg", &version("1.2.0"), source, CopyMode::Copy, SpecFormat::Xml, &source_hash(), None).unwrap() and asserts: report.conversion_fallbacks.len()==1; row.package=="org.acme/pkg"; row.version=="1.2.0"; row.file==crate::layout_paths::specs("PROP-001.md"); row.kept_format==SpecFormat::Markdown; row.reason contains "twice"; row.to_string() starts with format!("org.acme/pkg@1.2.0: {} kept as Markdown (conversion failed: ", specs("PROP-001.md")); the slot (slot_abs_path) holds PROP-001.md byte-equal to the input and guide.xml exists.

**Read first.**
- crates/vibe-workspace/src/vibedeps/derived.rs:316-380
- crates/vibe-workspace/src/vibedeps/derived/materialise.rs:1-155
- crates/vibe-workspace/src/vibedeps/slot_diff/report.rs:1-59
- crates/vibe-workspace/src/install/model.rs:225-260
- crates/vibe-workspace/src/install.rs:100-112,355-390
- crates/vibe-workspace/src/vibedeps/tests_transformed.rs:1-45,125-180

**Steps.**
1. install/model.rs: add `pub struct ConversionFallback { pub package: String, pub version: String, pub file: String, pub kept_format: SpecFormat, pub reason: String }` with #[derive(Debug, Clone, PartialEq, Eq)] and doc comments, plus `impl std::fmt::Display` writing `{package}@{version}: {file} kept as {fmt} (conversion failed: {reason})`, where fmt is "XML" for SpecFormat::Xml and "Markdown" for Markdown and Mixed. Add `pub conversion_fallbacks: Vec<ConversionFallback>` to InstallOutcome directly after integrity_warnings, with a doc comment. In install.rs add ConversionFallback to the `pub use model::{...}` line.
2. derived.rs: add a private `enum Transformed { Copy, Write(PathBuf, Vec<u8>), Fallback(String) }` and change transform_file to return Result<Transformed, WorkspaceError>. `Ok(None)` becomes `Ok(Transformed::Copy)`, `Ok(Some((p, b)))` becomes `Ok(Transformed::Write(p, b))`, and the instruction-file branch maps its Option the same way. The non-overlay read failure `Err(_) => return Ok(None)` becomes `Err(error) => return Ok(Transformed::Fallback(format!("cannot be read as UTF-8 text: {error}")))`. The non-overlay parse failure becomes `Err(error) => return Ok(Transformed::Fallback(error.to_string()))`. The overlay-targeted hard errors stay unchanged.
3. slot_diff/report.rs: add `pub(crate) conversion_fallbacks: Vec<crate::install::ConversionFallback>` to MaterialiseReport, initialised to Vec::new() in MaterialiseReport::new.
4. derived/materialise.rs legacy loop: match on Transformed. Write behaves like today's Some (Converted). Copy and Fallback behave like today's None (Copied). For Fallback(reason), also push ConversionFallback { package: package.clone(), version: version.to_string(), file: path_to_slash(&rel), kept_format: SpecFormat::Markdown if rel has extension "md", else SpecFormat::Xml, reason } into a local `fallbacks` vec. At the end, replace `reconcile_slot(&slot, &incoming, &record)` with `let mut report = reconcile_slot(&slot, &incoming, &record)?; report.conversion_fallbacks = fallbacks; Ok(report)`.
5. install.rs: add `conversion_fallbacks: Vec<ConversionFallback>` to struct Materialised and a local vec in materialise_resolution_with_prepared. After each materialise_with_spec_format_report call, run `conversion_fallbacks.extend(materialise_report.conversion_fallbacks.iter().cloned())`, and return it in Materialised. In lifecycle_apply.rs (both destructures and both InstallOutcome literals) and in subtree.rs, thread the field through. SubtreeOutcome gains `pub conversion_fallbacks: Vec<ConversionFallback>` after integrity_warnings.
6. Write the test, run the check, then run `cargo xtask specmap` from the repository root and include the regenerated specmap.json in the commit.

**Decisions already made.**
- Type name ConversionFallback, defined in crates/vibe-workspace/src/install/model.rs and exported as vibe_workspace::install::ConversionFallback
- Field name conversion_fallbacks on MaterialiseReport, Materialised, InstallOutcome and SubtreeOutcome
- file is the slot-relative source path with forward slashes (path_to_slash(&rel)), e.g. vibevm/vibespecs/PROP-001.md
- package is "group/name" and version is the semver string
- Only the two non-overlay Err(_) arms of transform_file produce rows. Those arms are reached only for opposite-format spec documents, so ordinary copied files and same-format files never produce rows.
- Overlay-targeted read and parse failures keep their existing hard WorkspaceError::SpecMaterialization
- The slot record schema is unchanged: a fallback file keeps disposition Copied
- The declared-source path (derived/declared.rs) and the Mixed path are not touched
- A skipped (identity-current) slot produces no row, because it is not materialised
- The public footprint wrapper materialise_with_spec_format keeps ignoring the rows (into_footprint)

**Not in this entry.** Printing, tracing or JSON-reporting the rows anywhere, and any strictness switch. Changing derived/declared.rs or the slot record schema is also out.

Entries that write the same file run one after another, never in parallel.
