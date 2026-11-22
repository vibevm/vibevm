# ADV-CONTROL-CODES-4

Planned at commit `9d8428d07`. Line numbers below are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** Under --json, an ambiguous short name exits 7 with code `ambiguous_package` and remedy.candidates listing the qualified package names to choose from.

**Write.** `crates/vibe-cli/src/exit_code.rs`, `crates/vibe-cli/src/commands/short_name.rs`, `crates/vibe-cli/src/commands/app/short_name.rs`, `crates/vibe-cli/src/commands/app/short_name/slots.rs`, `crates/vibe-cli/src/commands/app/short_name/render.rs`, `crates/vibe-cli/tests/cli_registry_alias.rs`, `VIBEVM-SPEC.md`

**Check.** `bash tools/cargo-test-ran.sh -p vibe-cli --bin vibe --test cli_registry_alias remedy_candidates`

**Test.** Unit test in the existing `mod tests` of crates/vibe-cli/src/exit_code.rs: (1) remedy_candidates_of_an_ambiguous_package: InstallError::AmbiguousPackage{message:"m".into(), candidates: vec!["com.acme/wal".into(), "org.vibevm/wal".into()]} gives .remedy() == Some(json!({"candidates":["com.acme/wal","org.vibevm/wal"]})), and classify() of it gives code "ambiguous_package", exit 7, refusal true and the same remedy. CLI test in tests/cli_registry_alias.rs: (2) remedy_candidates_of_a_collision_under_json: uses the same Site setup as two_registries_issuing_one_spelling_for_two_packages_is_exit_seven, then runs site.vibe(&["app","installers","qwen","--json"]). It asserts exit 7, and that the last non-empty stderr line parses with code=="ambiguous_package" and refusal==true. remedy.candidates, sorted, must equal ["org.vibevm.qwen/qwen-code","org.vibevm.qwen/qwen-fork"].

**Read first.**
- crates/vibe-cli/src/exit_code.rs: items `enum InstallError`, `impl InstallError`, and `mod tests` fn `ambiguous_package_maps_to_seven`
- crates/vibe-cli/src/commands/short_name.rs:55-140,250-305
- crates/vibe-cli/src/commands/app/short_name.rs:120-270
- crates/vibe-cli/src/commands/app/short_name/render.rs:25-51
- crates/vibe-cli/src/commands/app/short_name/slots.rs:65-85
- crates/vibe-cli/tests/cli_registry_alias.rs:1-120,470-510

**Steps.**
1. In exit_code.rs, change the variant `AmbiguousPackage(String)` to `AmbiguousPackage { message: String, candidates: Vec<String> }`. In its existing `#[error(...)]` attribute replace only the leading `{0}` with `{message}`; every other character of the attribute (the backslash line continuations, the spec:// URI and the fix text) stays exactly as it is, so the rendered text is byte-identical. Update the exit_code() match to `AmbiguousPackage { .. }`. Make `remedy()` return `Some(json!({"candidates": candidates}))` for this variant and None for UserDeclined. Update the existing ambiguous_package_maps_to_seven test to the new shape.
2. In commands/short_name.rs, change `ShortNameOutcome::Ambiguous(Vec<String>, String)` to `Ambiguous(Vec<String>, String, Vec<String>)`, where the third element holds the qualified candidates. The lockfile branch (near line 123) passes `qualified(&locked, name)`. The alias branch (near line 136) passes `many.iter().map(|c| c.target.to_string()).collect()`. The match near line 260 builds `AmbiguousPackage { message: render_collision(..), candidates }`, and qualify_locked (near line 302) builds candidates from `qualified(many, &pkgref.name)`.
3. In commands/app/short_name/render.rs add `pub(super) fn collision_candidates(name: &str, lookup: &Lookup) -> Vec<String>`, which returns `lookup.eligible()` mapped to `format!("{}/{}", c.group, c.package_name(name))`. Use it at the two app construction sites, app/short_name.rs (Outcome::Ambiguous, near line 263) and app/short_name/slots.rs (near line 77), with message = render_collision(..) as today.
4. Write the unit test and the CLI test named in `test`. The CLI test file gets a private helper that parses the last non-empty stderr line as JSON.
5. Apply the spec_edit to VIBEVM-SPEC.md.
6. Run `cargo fmt --all`, the check command, `cargo test -p vibe-cli --bin vibe exit_code`, and `cargo clippy -p vibe-cli --all-targets -- -D warnings`.

**Decisions already made.**
- remedy for an ambiguous name is `{"candidates": [qualified group/name, …]}`, not `args`, because the fix replaces the positional package rather than adding a flag.
- Candidates are plain `group/name` strings with no `(alias in registry …)` or `(untrusted group)` decoration, and only eligible candidates are listed.
- The human message and exit code 7 are unchanged.
- The code and refusal already come from ADV-CONTROL-CODES-1's AMBIGUOUS_PACKAGE row. This entry adds only the remedy.

**Not in this entry.** Not in this entry: changing render_collision's wording, merging the project and app short-name renderers, and adding candidates to the not-issued refusal.

**Depends on.** ADV-CONTROL-CODES-1

**Spec edit (apply as a step of this entry).**

VIBEVM-SPEC.md §9.3 {#output-format}: append to the paragraph that begins "Under `--json`, a failed command's last stderr line is one JSON error envelope" this sentence:

An ambiguous short name carries `remedy.candidates`, the qualified package names to use instead.

Entries that write the same file run one after another, never in parallel.
