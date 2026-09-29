# Existing-only external store API candidate

Writer stopped. Only `crates/vibe-safefs/src/transaction/external.rs` and `crates/vibe-safefs/src/transaction/tests/external.rs` changed (575 and 461 lines). Existing test macro body is byte-identical to HEAD; additions are four tests and their fixture/snapshot helpers. Diff is 85 additions/15 deletions in the API owner and 173 additions/zero deletions in tests.

Added signatures:

- `ExternalStore::open_existing_disjoint(path: &Path, project: &Project) -> Result<Option<Self>>`
- `ExternalStore::lock_existing_project(project_key: &str) -> Result<Option<ExternalProjectLock>>`
- `ExternalStore::identity_token() -> Result<String>`

The existing-only capability walk creates nothing. `None` denotes an absent component outside retained project ancestry; missing inside-project suffixes, existing overlap, links, non-directories, and unavailable I/O refuse. Completed selections retain the physical disjointness proof. The lock opens an existing directory and named file without creation, preserving regular/single-link checks, held blocking `File::lock`, Windows sharing restrictions, and named-object identity rechecks. A narrow common helper keeps create-enabled acquisition behavior. The opaque store token uses the existing identity owner with domain `vibe-safefs-external-store-identity-e1\0`, distinct from the project token.

The actual governing filename is `vibevm/vibespecs/common/PROP-056-scraped-project-export.xml` (`SEC-NO-FOLLOW`). No dependency, raw syscall, unsafe block, migration journal, or apply behavior was added.

Four exact root-run bindings (`-p vibe-safefs --lib`, each `-- --exact`):

- `transaction::tests::existing_store_absence_has_no_creation`
- `transaction::tests::existing_disjoint_store_opens_and_invalid_scope_refuses`
- `transaction::tests::existing_project_lock_absence_has_no_creation`
- `transaction::tests::existing_project_lock_reacquires_rechecks_and_store_token_is_stable`

Tests compare fixture names/kinds/bytes before and after inspection; cover overlap including missing inside suffix, file/link refusal, missing lock directory/file, stable distinct tokens, identity retry, and hardlink/symlink lock refusal. Root additionally selected existing `transaction::tests::external_lock_rechecks_identity_and_durable_writes_report_parent_support` because create acquisition shares the helper.

Targeted rustfmt returned zero; its unrelated macro reindent was restored while keeping formatted new Rust bodies in the original macro style. No Cargo, compilation, tests, user-path operation, Git mutation, or public operation ran. This is an unverified implementation candidate, not platform or runtime acceptance.
