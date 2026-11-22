# ADV-SDD-CHANGE-WRITE-2

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** `vibe_core::manifest::change::plan_origin_rebase` rewrites only the package pin, manifest_sha256 and document sha256 tokens of the named origins, keeps every other byte and comment of change.toml, and refuses an origin pin that is not in the file.

**Write.** `crates/vibe-core/src/manifest/change/mod.rs`, `crates/vibe-core/src/manifest/change/rebase.rs`, `crates/vibe-core/src/manifest/change/rebase_cases.rs`

**Check.** `cargo test -p vibe-core --lib manifest::change::rebase_cases 2>&1 | grep -F "test result: ok. 2 passed"`

**Test.** crates/vibe-core/src/manifest/change/rebase_cases.rs: `rebase_rewrites_only_origin_tokens` and `rebase_refuses_unknown_origin_pin`

**Read first.**
- crates/vibe-core/src/manifest/change/migration.rs (plan_change_migration and the ChangeMigrationPlan doctest: toml_edit span -> MigrationTokenEdit -> replay_authored_tokens -> reparse)
- crates/vibe-core/src/manifest/change/mod.rs (ChangeError, invalid, ChangeErrorKind, ChangeContext, ChangeManifest::parse_str, ChangeManifest::origins)
- crates/vibe-core/src/manifest/migration.rs (MigrationTokenEdit fields field/before_span/before_token/after_token; replay_authored_tokens)
- formats/corpora/change-manifest/e1/valid/representative.toml
- crates/vibe-core/src/manifest/change/m_02_b_2_cases.rs (test module style: TEXT via include_str!, ChangeContext::for_node fixture)

**Steps.**
1. Create rebase.rs. Line 1: `//! Pure origin rebase proposal over the exact token replay law; it grants no write.` Line 2: `specmark::scope!("spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-MIGRATION-REPLAY");`. Then `use super::*;` and `use crate::manifest::MigrationTokenEdit;`.
2. Add `#[derive(Debug, Clone, Copy)] pub struct OriginRebase<'a> { pub package: &'a str, pub new_package: &'a str, pub new_manifest_sha256: &'a str, pub documents: &'a [(&'a str, &'a str)] }` (each documents pair is (address, new sha256)) and `#[derive(Debug, Clone)] pub struct ChangeRebasePlan { pub before: String, pub after: String, pub edits: Vec<MigrationTokenEdit>, pub checked: ChangeManifest }`.
3. Add `#[specmark::spec(implements = "spec://org.vibevm.core/vibevm/common/PROP-071#CHANGE-MIGRATION-REPLAY")] pub fn plan_origin_rebase(text: &str, context: &ChangeContext, directory: &Path, rebases: &[OriginRebase<'_>]) -> Result<ChangeRebasePlan, ChangeError>`. 1) `ChangeManifest::parse_str(text, context, directory)?;` 2) `let document = toml_edit::Document::parse(text).map_err(|_| invalid(ChangeErrorKind::Syntax, "rebase.document", "invalid TOML"))?;` 3) `let origins = document.get("origins").and_then(toml_edit::Item::as_array_of_tables)` (None maps to invalid(ChangeErrorKind::Origin, "rebase.package", "no origin row has this package pin")). For each rebase, find the table in `origins.iter()` whose `get("package").and_then(Item::as_value).and_then(Value::as_str) == Some(rebase.package)`; none returns invalid(ChangeErrorKind::Origin, "rebase.package", "no origin row has this package pin"). Push an edit for that table's `package` value (field "origins.package", new value rebase.new_package) and its `manifest_sha256` value (field "origins.manifest_sha256", new value rebase.new_manifest_sha256). For each (address, sha) in rebase.documents, find in that table's `get("documents").and_then(Item::as_array_of_tables)` the table whose `address` string equals address; none returns invalid(ChangeErrorKind::Origin, "rebase.documents.address", "origin has no document with this address"). Push an edit for its `sha256` value (field "origins.documents.sha256", new value sha).
4. Each edit is built by one private helper `fn token_edit(text: &str, value: Option<&toml_edit::Value>, field: &'static str, new_value: &str) -> Result<MigrationTokenEdit, ChangeError>`: a missing value or `value.span()` of None returns invalid(ChangeErrorKind::Migration, "rebase.span", "token has no exact span"); before_token is `text[span]` taken with `text.get(span.clone())` (None maps to the same error); after_token is `format!("\"{new_value}\"")` (a basic double-quoted TOML string; the span of a parsed string covers its quotes). After collecting, drop every edit whose before_token equals its after_token, sort by `before_span.start`, call `crate::manifest::replay_authored_tokens(text, &edits)` mapping an error to invalid(ChangeErrorKind::Migration, "rebase token replay", "exact token edit refused"), then `let checked = ChangeManifest::parse_str(&after, context, directory)?;` and return `ChangeRebasePlan { before: text.into(), after, edits, checked }`.
5. Doc examples (vibe-core is in gated_pub_doctest, so every new pub item needs one). On OriginRebase: ```
use vibe_core::manifest::change::OriginRebase;
let docs = [("spec://org.example/intent/FEAT-001#DELIVERY", "sha256:00")];
let r = OriginRebase { package: "feat:org.example/intent@=1.0.0", new_package: "feat:org.example/intent@=1.1.0", new_manifest_sha256: "sha256:00", documents: &docs };
assert_eq!(r.documents.len(), 1);
```. On ChangeRebasePlan and on plan_origin_rebase the same hidden setup as the ChangeMigrationPlan doctest (include_str! of representative.toml, Manifest::parse_str of `[project]\ngroup='org.fixture'\nname='alpha'\nversion='1.0.0'\n`, ChangeContext::for_node(&node, ".", Some(Path::new("docs/custom-specs"))), ChangeId::parse("chg-0123456789abcdef0123456789abcdef"), context.directory(&id)), then: ```
let plan = plan_origin_rebase(text, &context, &directory, &[OriginRebase { package: "feat:org.fixture/reusable-intent@=1.0.0", new_package: "feat:org.fixture/reusable-intent@=1.1.0", new_manifest_sha256: "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", documents: &[] }]).unwrap();
assert_eq!(plan.edits.len(), 1);
assert!(plan.after.contains("@=1.1.0"));
```
6. In change/mod.rs: add `mod rebase;` on the line after `mod validation;`; add `pub use rebase::{ChangeRebasePlan, OriginRebase, plan_origin_rebase};` after the existing `pub use migration::{...};` block; at the end of the file add `#[cfg(test)]` and `mod rebase_cases;`.
7. Create rebase_cases.rs: `use super::*;`, `const ID: &str = "chg-0123456789abcdef0123456789abcdef";`, TEXT via the same include_str! as m_02_b_2_cases.rs, and `fn context() -> ChangeContext` = `ChangeContext::for_node(&Manifest::parse_str("[project]\ngroup='org.fixture'\nname='alpha'\nversion='1.0.0'\n").unwrap(), ".", Some(Path::new("docs/custom-specs"))).unwrap()`. `rebase_rewrites_only_origin_tokens`: let c = context(), d = c.directory(&ChangeId::parse(ID).unwrap()), m = format!("sha256:{}", "c".repeat(64)), s = format!("sha256:{}", "d".repeat(64)), docs = [("spec://org.fixture/reusable-intent/FEAT-001#DELIVERY", s.as_str())]; call plan_origin_rebase(TEXT, &c, &d, &[OriginRebase { package: "feat:org.fixture/reusable-intent@=1.0.0", new_package: "feat:org.fixture/reusable-intent@=1.1.0", new_manifest_sha256: &m, documents: &docs }]).unwrap(); assert `plan.after == TEXT.replacen("@=1.0.0", "@=1.1.0", 1).replacen(&"a".repeat(64), &"c".repeat(64), 1).replacen(&"b".repeat(64), &"d".repeat(64), 1)`, `plan.edits.len() == 3` and `plan.checked.origins()[0].package.ends_with("@=1.1.0")`. `rebase_refuses_unknown_origin_pin`: same call with package "feat:org.fixture/other@=1.0.0"; `let err = ... .unwrap_err();` assert `err.kind == ChangeErrorKind::Origin` and `err.field == "rebase.package"`.

**Decisions already made.**
- The function is pure: it reads and writes no files. It creates no new spec anchor and cites the existing CHANGE-MIGRATION-REPLAY law it reuses.
- New values are written as basic double-quoted TOML strings; pins and digests contain no characters that need escaping, and the final reparse validates them.
- change/mod.rs is 391 lines and grows by 5; all new code lives in the new rebase.rs (well under 600 lines).
- No generated file, wire type or codegen step is involved; toml_edit is already a vibe-core dependency.
- Clippy -D warnings applies. Do not add #[allow].

**Not in this entry.** Not in this entry: the CLI command, reading the installed world, adding or removing origins or documents, a generic TOML editing helper, and any change to plan_change_migration.

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
