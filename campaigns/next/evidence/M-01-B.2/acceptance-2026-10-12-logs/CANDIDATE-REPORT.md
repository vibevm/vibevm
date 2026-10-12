# M-01-B.2 candidate — selected world continuity

Base HEAD `e695bc76e2d5cb063742c26b6809247c19c85b74`. This is one unstaged product candidate for Root review; no commit, push, release, provider fetch or hydration was performed. `FIXED-ISSUES.md` is Root-owned and was already modified separately.

## Changed owner files

| File | SHA-256 |
| --- | --- |
| `vibevm/vibespecs/common/PROP-070-resource-resolution.xml` | `0c6ad774f425f45544429abfde5c75f4682019ab565a37cf3c76b073154a62f4` |
| `crates/vibe-workspace/src/source_query/mod.rs` | `8f27fcbe5441e1f6c44acdde752b1a22073edc3cd9d9b2381a9476d710b9ef07` |
| `crates/vibe-workspace/src/source_query/capture.rs` | `42454063160aed0637140507aa6c9a7d0589ec8065dc62d5cdbc7029e532898b` |
| `crates/vibe-workspace/src/source_query/capture/continuity.rs` (new) | `fc173c269223b1e36f8b17bb0ea4a34139085fe85d097f748be4e735a73a3f7e` |
| `crates/vibe-workspace/src/source_query/capture/m_01_b_2_cases.rs` (new) | `682adf895283e8f39cc9c7aa8ca7c6c7cc371cdf546b05b8f4651c100f2b838a` |
| `crates/vibe-workspace/src/source_query/capture/m_01_b_2_final_window.rs` (new) | `4948eaac4a59daf27ab83452c4804702cbe72988c499c7b9cb267603eb233ae9` |

PROP-070 now owns `#SELECTED-WORLD-CONTINUITY`. `WitnessedSourceWorld` joins B1's prepared selected binding to caller-prepared lock, root-to-selected-node membership, reachable package-manifest and selected descriptor-record witnesses without I/O in construction. It validates the lock's selected rows against the epoch's typed version/kind/hash/ordered exact pinned edges and materialization-derived root. Query-time revalidation uses bounded SafeFS snapshots for those selected witnesses and captured outputs, including a final governing-file and lock check after the last output read, and returns B1's prepared bytes only after agreement. It does not discover unrelated member slots or inspect unselected package payloads. Missing/read-failed selected evidence returns a coarse `WorldUnavailable` boundary; changed evidence returns `StaleWorld`, without an absolute path in the new error variants.

## Independent review counterexamples

Root's read-only `CONTINUITY-REVIEW.md` identified three source risks; none was presented as executed proof until tested here.

1. **Copy epoch + InPlace lock:** exact fixture changed the genuine selected lock's materialization while preserving coordinate/version/hash/edges and old copy-slot captures. Constructor returned `Ok` before fix: `materialization-red.log`, 0/1, exit 101. The join now checks the retained package manifest materialization and incumbent materialization-specific slot path against provider root. The same exact case passes in `b2-family-ten-final-window.log`.
2. **Map O←A + record O←B, same output hash:** valid authored CommonMark A/B declarations and a valid transformed slot record returned `Ok` before fix: `source-map-red.log`, 0/1, exit 101. The join now compares each captured descriptor-map output's source with the record's effective source (output path for Mixed, recorded source for transformed). The same negative passes, and a matching transformed O←B map passes as a control inside that test.
3. **Member/descriptor change inside the observation window:** a second full selected-file pass still checked member/record before its last output, then checked only lock. The exact final-output fixture changed the selected descriptor record at that boundary and received `Ok` (`final-window-red-confirmed.log`, 0/1, exit 101, `accepted stale world`). The fix separates governing files from outputs and rechecks membership, selected manifests and records after the final output, then the lock. The same fixture passes 1/1 in `final-window-fixed.log`; the earlier between-pass mutation remains green in the 10-case family. A change after the last applicable check remains possible; no atomic filesystem snapshot or native race-free guarantee is claimed.

Subsequent source review found a fourth exact join: a lock dependency edge could pin leaf@2 while the retained epoch and selected leaf row held leaf@1; coordinate-only comparison accepted it (`edge-pin-red.log`, 0/1, exit 101). The join now compares each ordered edge against the selected row's incumbent exact `LockedPackage::as_package_ref()`; the matching leaf@1 control and wrong leaf@2 negative pass in `b2-family-ten-final-window.log` (10/10, 0 ignored). This stays within the selected closure and does not inspect unrelated members.

The separate valid authored `[[package.sources]]` edit case passes as stale selected manifest. A broken unrelated member manifest leaves a proved selected member+dependency query ready and returns its original captured bytes. Removal of a selected manifest reports `WorldUnavailable("selected manifest")`; changed membership/output reports `StaleWorld`, with no temp root in the displayed error.

## Exact checks

All Cargo invocations used `CARGO_TARGET_DIR=/fast/git/v/.build/c4-first-agent-target`, `+nightly --locked --offline` from `/fast/git/v/vibevm-next`.

| Check | Result | Log |
| --- | --- | --- |
| `cargo test -p vibe-workspace --lib source_query::capture::continuity::m_01_b_2_cases -- --nocapture` | 10 passed, 0 failed, 0 ignored | `b2-family-ten-final-window.log` SHA-256 `cd03df28c44b7036bee972f62411f359bc6282f294fb082cbf769a0ca3e51273` |
| `cargo test -p vibe-workspace --lib source_query::capture::continuity::m_01_b_2_final_window::final_output_window_rechecks_selected_descriptor -- --exact` | 1/1, 0 ignored | `final-window-fixed.log` SHA-256 `c31cf2db3e7b5db7702d20b59fc439266abbd10002bfbf69e6d3b12022719327` |
| `cargo test -p vibe-workspace --lib source_query::prepared_cases::member_self_and_admitted_dependency_bind_exact_captured_roots -- --exact` | 1/1, 0 ignored | `b1-compat-final-window.log` SHA-256 `190fefb98a6b2bb9567ee5e3f277196d92b02b2f5fe8f8ed9e5eecd33ddedd56` |
| `cargo test -p vibe-spec --lib resolver::m_01_a_2_cases::descriptor_rows_use_the_same_portable_component_and_identity_law -- --exact` | 1/1, 0 ignored | `spec-compat.log` SHA-256 `3098bcf29f24d964a2ceaa28e1e739172146c8272294dbf07fe7699104249fce` |
| `cargo +nightly fmt --package vibe-workspace -- --check` | exit 0 | `package-fmt-final-window.log` |
| `git diff --check` | exit 0 | terminal check |
| `cargo +nightly fmt --all -- --check` | exit 1: same 15 hunks in unrelated CLI/MCP/orchestrator/publish/trace/wire files | `full-fmt.log` SHA-256 `24acc9dabcadbeeb2ce4bd2b6acd0cdabc70f59e492ed613b7da7a39c1a3eba1` |

No test selected zero cases. New source files are 513, 592 and 38 lines, all below 600. The first family build failed before test execution from my private `slot_record` import and shadowed fixture name (`b2-family-first.log`, exit 101). After correcting those, a slot-record fixture violated its sorted-row invariant (`b2-family-second.log`, 3 passed/1 failed, exit 101); I corrected fixture order. The Copy/InPlace, O←A/O←B, edge-pin and last-output red logs are real product failures. All logs remain in this directory; Root should merge these incidents into its own ledger.

## Limits / next action

Current-world revalidation is a bounded detection step over SafeFS snapshots; it is not an atomic multi-file read, native Windows/macOS qualification or M-01-C retained source-read provenance. B1's trusted preparation boundary remains: no public API attests arbitrary Rust caller provenance. A previously loaded `Workspace` is supplied; the positive unrelated-member case proves B2 does not re-read its broken member, not that a fresh global `Workspace::load` could parse it. The lock does not pin target representation: the incumbent citation descriptor reader explicitly uses `expected_format=None`, so this candidate does not infer or impose a package-manifest/record `spec_format` equality absent a trusted target-format witness. Source path, digest, selected manifest and record snapshots are checked as described above. No general filesystem/kernel protection or broad campaign gate was run. Root should inspect the candidate and the four reproduced counterexample dispositions, integrate the incident ledger, then accept/commit atomically if satisfied. Do not begin another atom from this worker report.
