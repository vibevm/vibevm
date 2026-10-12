# M-01-C.1 candidate — retained source observation

Base HEAD `f60dd007b8f0c019abe21e8bae39e52b69280867`. Product changes are unstaged for Root review. No commit, public push, release, provider fetch, cache hydration, query lock, directory creation or COW publication was performed.

## Selected behavior and source

`vibe-safefs::Project::read_file_snapshot_bounded` already holds no-follow root/parent/file capabilities through two bounded passes and the final capability-relative name/identity check. It returns one `StableFileSnapshot` containing the exact bytes, raw SHA-256, size and opaque file identity. Its existing single-link law stays intact; no SafeFS reader implementation changed.

The actual consumer was `FsSectionSource` in `crates/vibe-spec/src/embed.rs`: its incumbent `FileResolver::read_projected_source`/`read_preflighted_source` used two pathname `fs::read` calls. `resolver/read.rs` now uses the existing SafeFS snapshot after the incumbent portable selected-root preflight. The Markdown/XML projection consumes `snapshot.bytes`; `read_observed_projected_source` retains that same snapshot beside projected text, so no second pathname read supplies the parser or raw digest. Custom source reads use the same observed source method. A missing selected file after preflight is unstable; failure to establish a held observation reports `SourceObservationUnavailable` with the actual SafeFS reason and publishes no bytes. The old injected two-read helper remains test-only for accepted A3 error characterization. Physical output above the explicit 64 MiB source cap is unavailable rather than truncated.

PROP-070 now defines `#RETAINED-SOURCE-OBSERVATION` and updates `#INTERNAL-LOOKUP-FAILURES` to distinguish portable preflight from the production held read. Its A3 file scope remains registered; the new read method carries the C1 contract citation. Overlay bytes supplied in memory are a separate caller-provided input, not claimed as a physical held-file observation.

The deciding same-byte namespace case now exercises both the SafeFS snapshot and the **actual** `FsSectionSource::section_text` consumer. The existing SafeFS hook fires after its source handle opens; it renames that object and installs a distinct regular file with identical bytes at the original name. Both held reads refuse with `final-name race`, and each fixture confirms equal bytes but different file identities. The historical test-only two-pathname-read helper accepts that same-byte rebind, demonstrating why content equality alone was insufficient. `vibe-spec` enables the existing `vibe-safefs/inject-failures` hook only as a dev dependency feature for this exact case; Cargo.lock versions/hashes and shipped feature behavior are unchanged.

| Changed file | SHA-256 |
| --- | --- |
| `vibevm/vibespecs/common/PROP-070-resource-resolution.xml` | `f798d6a0355e245c384173fef8d17a9e01151f920b9c167a072a8580a5d5ac09` |
| `crates/vibe-safefs/src/file.rs` | `2a89c0eb54511cb47df696d4514a0c167c561d843cedb9745d5a2b4750e6b3a2` |
| `crates/vibe-safefs/src/file/m_01_c_1_cases.rs` (new) | `20b48e395dd9c053dad4897c541646a76b64a6fdd21aa85a960bc32c57d451da` |
| `crates/vibe-spec/Cargo.toml` | `36af3167625426b8e6399b219c44a9d8b660284a7c51cce5120dd65806d23368` |
| `crates/vibe-spec/src/resolver.rs` | `70b9b9ff61995ad75a135e78e70cd61dd56f5b0b50810f8f87d36ffef9eb7845` |
| `crates/vibe-spec/src/resolver/read.rs` | `953eb8eba10fd73e644bdd372dad05b7d37012993d45fc02f0d9766fe67a6c92` |
| `crates/vibe-spec/src/resolver/m_01_c_1_cases.rs` (new) | `01e20120cbb7e4f1b5d78588f8bf6e21ac36b20de9c6d209240742ea34e3bb0c` |

## Exact checks

All Cargo tests ran from `/fast/git/v/vibevm-next` with `CARGO_TARGET_DIR=/fast/git/v/.build/c4-first-agent-target`, `+nightly --locked --offline`. Log paths below are relative to this report. Every selected family ran nonzero tests, zero ignored.

| Target / selector | Result | Log SHA-256 |
| --- | --- | --- |
| `vibe-safefs --lib file::m_01_c_1_cases` | 4/4: stable bytes/digest/identity, hardlink, Unix symlink, identical-byte distinct-object replacement | `safefs-c1-final.log` `afa57c2f3501286b4341b5b210ef84100bb39239107725e78455567f892dff3b` |
| `vibe-safefs --lib file::m_01_c_1_cases::same_byte_replacement_during_held_snapshot_refuses -- --exact` | 1/1 | `safefs-same-byte.log` `bcc8c59ff0a80b7e8b64843b982557e252ea3eeb021057bb2dc23cd9af409c67` |
| `vibe-spec --lib resolver::read::m_01_c_1_cases` | 4/4: stable digest/projection/embed, shared source unavailable, historical same-byte rebind accepted, actual consumer rebind refused | `spec-c1-same-byte.log` `1a35331c4e4e50df78d1d90efa0ca65e956264324971c63a78898cb1d7553ce5` |
| `vibe-spec --lib resolver::read::m_01_c_1_cases::same_byte_replacement_during_actual_embed_read_refuses -- --exact` | 1/1 after distinct-identity assertion | `spec-same-byte-final.log` `11e06fe7cc6debae3ec8f1091e433cbe6bb826f7b6ad17db369709c91f8008d4` |
| `vibe-spec --lib resolver::read::tests` | 2/2 accepted A3 read-error characterization | `spec-a3-read.log` `3ca8c684e4ae7dbc901365a2a4231bbc351b536cd75c18cc09c6f3ae6580a36e` |
| `vibe-spec --lib embed::tests::a_section_of_an_xml_dependency_reads_as_its_canonical_twin -- --exact` | 1/1 | `spec-embed-xml.log` `a7cc98d7a0093d3a039d8db61a3a6399327dc12d607be2a0880d2145c0e801a0` |
| `vibe-spec --lib resolver::m_01_a_3_cases::compiler_facing_source_preserves_content_and_distinct_refusals -- --exact` | 1/1 | `spec-a3-compiler.log` `e99f6fe12a4babf07057b3be0faf74959266801d0904e267b917837ccb9414b4` |
| `vibe-spec --lib resolver::m_01_a_3_cases::descriptor_owned_embed_read_refuses_symlinked_output -- --exact` | 1/1 | `spec-a3-symlink.log` `d68dbdda9dea755850a72456944d8765c1ef1daefebbd77c46e01d65b06f6bbe` |
| `cargo +nightly check --locked --offline -p vibe-spec --lib` | exit 0 without dev feature | `spec-lib-check.log` `cc61e1708025c27d7c8a1271b769f070b496ee19fbaa5a824130c82d4643d06f` |
| `cargo +nightly fmt --package vibe-safefs --package vibe-spec -- --check` | exit 0 | `package-fmt-post-same-byte.log` |
| `git diff --check` | exit 0 | terminal check |
| `cargo +nightly fmt --all -- --check` | exit 1, same 15 unrelated hunks in CLI/MCP/orchestrator/publish/trace/wire | `full-fmt-post-same-byte.log` `24acc9dabcadbeeb2ce4bd2b6acd0cdabc70f59e492ed613b7da7a39c1a3eba1` |

New source files are 95 and 110 lines; touched Rust owner files stay below 600. No setup, compile or selected-test failures occurred. The full-workspace format failure is the pre-existing 15-hunk baseline, not a green gate. Root owns any final specmap generation and the incident ledger.

The A3 `resolver::read::tests` two-read cases now execute only the `#[cfg(test)]` historical helper. They preserve error classification characterization; they do **not** prove the new production held reader. The two same-byte held-read cases and the stable consumer case above are the deciding C1 proof.

## Limits / Root next step

The verified window begins with the SafeFS no-follow held observation and ends at its final file/name check. A replacement before that observation can legitimately become the object observed; a change after its final check, including hostile ABA or an external alias, is not ruled out. Linux ran the physical rename/symlink cases; native Windows/macOS behavior and broad platform qualification are unperformed. The separate development overlay route has no physical-file identity. Source membership authentication and conversion provenance belong to later atoms; this C1 candidate assigns neither. Root should inspect the incumbent consumer integration and the exact logs, then accept/commit atomically if the boundary matches C1. No C.2 or other atom was started.
