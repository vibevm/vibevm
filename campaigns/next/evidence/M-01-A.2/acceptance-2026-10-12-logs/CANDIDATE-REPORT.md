# M-01-A.2 candidate for Root review

2026-10-12. Base HEAD `b7cf75a72b16b637fe4e45b25d7af29f5ee930e5`; source candidate remains unstaged. Root owns acceptance, FIXED-ISSUES.md and commit. Root's current `FIXED-ISSUES.md` edit is disjoint and was not changed by this executor.

## Bounded source and behavior

- `vibe-spec` now uses the already existing `vibe-safefs` component authority. `Cargo.toml` adds that workspace edge; `Cargo.lock` adds exactly `"vibe-safefs"` to the `vibe-spec` dependency row, with no version/hash changes. No `vibe-safefs` production source was edited.
- New `resolver/portable.rs` wraps `split_relative`, `identity_key`, exact sibling checks and `ensure_no_follow_walk` for physical source lookup. It rejects hostile relative components before directory traversal, returns typed `UnsafeSourcePath`/`PortableSourceAlias`, checks exact nested directory spelling and final regular file without following a symlink, and never rewrites the requested logical name. It is a read-side preflight, not a captured or race-free read grant.
- `resolver/lookup.rs` uses that seam for both direct and abbreviated ID lookup. It retains Markdown/XML and active custom-format collision behavior. The ID path now detects case-folded extension and terminal-dot aliases instead of silently reporting not-found. New-layout probes use no-follow metadata, avoiding a `.is_dir()` follow of a symlinked `vibevm/vibespecs` root.
- `resolver/source_map.rs` replaces its local relative-path blacklist and lowercase output collision key with shared `split_relative` and `path_identity_key`. `resolver/resource.rs` checks the classified physical Vibe output before returning a strict logical identity; raw named export construction remains an identity/declaration operation without a source read.
- `PROP-070#PORTABLE-SOURCE-COMPONENTS` records this portable read-side rule and its limits. New `resolver/m_01_a_2_cases.rs` has seven cases. Parent resolver registration/errors and crate manifest are the only other product edits. All touched Rust files remain below 600 lines.

## Evidence with exact checked subjects

Using `cargo +nightly test --locked --offline`, private `CARGO_TARGET_DIR=/fast/git/v/.build/c4-first-agent-target`:

- `verified-a2.log`: `vibe-spec --lib resolver::m_01_a_2_cases::`, 7/7, zero ignored. Nested valid XML ID path retains its exact physical name. Drive, UNC, ADS, device, trailing dot, control and dot traversal input refuse. Directory/file case and NFC aliases refuse. Linux symlinked child/output and layout-root fixtures refuse or do not select the outside source. Descriptor rows share the portable component/case/NFC law.
- `verified-a1.log`: accepted A1 corpus, 7/7, zero ignored after final A2 source.
- `verified-id.log`, `verified-custom.log`, `verified-pair.log`, `verified-selected.log`: four exact affected incumbent `vibe-spec --lib` cases, each 1/1, zero ignored. The descriptor-map incumbent case passed 1/1 after source-map change in `final-spec-descriptor_map_indexes_only_selected_vibe_and_refuses_forged_or_ambiguous_rows.log`; its checked body was not changed afterward.
- Four exact `vibe-safefs --lib` cases passed 1/1 each, zero ignored: `the_shared_table_refuses_every_hostile_spelling`, `split_relative_judges_every_component_not_only_the_last`, `identity_folds_every_case_a_simpler_key_would_miss`, `complete_sets_share_the_identity_and_literal_path_laws` (`safefs-*.log`). No shared source changed.
- `id-extension-before.log`: the physical `.MD` ID-stem alias regression failed 0/1, exit 101. `trailing-physical-before.log`: terminal-dot physical alias failed 0/1, exit 101. Both now pass in `verified-a2.log` after only the ID lookup comparison changed.
- `spec-fmt-final2.log`: package-scoped `cargo +nightly fmt -p vibe-spec -- --check` exit 0. `git diff --check` exit 0. `xml-check.log`: PROP-070 parses and the new anchor exists. `Cargo.lock` diff is only the expected graph edge.

## Actual failures and limits

- The first attempted test command could not open its log because I had not created `E/M01-A2` yet (shell exit 1). I created the directory and reran; `first-spec-a2.log` then passed 4/4. This was my setup error, not a compile/test verdict.
- `safefs-component-family.log`: the broader existing shared family is **red**, 15 passed / 1 failed / 0 ignored, exit 101. The failing existing case is `component::tests::overlap_and_containment_use_the_same_component_boundaries`: its Windows-style `Path` overlap assertion is false under this Linux native `Path::components` behavior. `vibe-safefs` source was unchanged. A2 uses `split_relative` and `identity_key`, whose exact deciding tests above passed. Do not call this family green or repair `paths_overlap` as incidental A2 work.
- `workspace-fmt.log`: full workspace format check remains exit 1 with the same 15 unrelated hunks outside `vibe-spec`; package check is green. No unrelated formatting was applied.
- Linux lexical and symlink fixtures are not native Windows/macOS qualification or syscall tracing. The preflight does not close namespace races or capture/recheck source bytes. Full source-read failure distinctions and caller propagation remain A.3; public query/capture remains later work. Ambient legacy slot selection in old callers was not redesigned.
- No broad panel, mutation, public Git/release, stopped Zap+Lean, PodBay or external Aeneas work ran. Root should regenerate/review specmap for the new source scope after accepting the contract; no specmap gate is claimed here.

## Final source identifiers and next step

SHA256: `lookup.rs` `ced4a59a2b75fa7958452f599c7a13c00a3cecd2f19978ec638d69b843032a21`; `portable.rs` `6a3d82e61d10183af6ff100baac59c063bf55561e0d3ced59e3bbb5c5c061f7b`; `m_01_a_2_cases.rs` `d380b90d42c7ff5c35e87ecdae479d98082471b7b1c7509ed41ade9e03cb3084`; `source_map.rs` `751bb4c5e9a2968498844004de87569655b3d1dfad2306859720700ba3e15b90`; `resource.rs` `54887819d9f372d4813f2809bc8af2106736a8707b6bafe1fed7ad43e2a175ea`; PROP-070 `3a27031146db9f27d205035196dee5a0b7877d4717b9a49558cd2d76cdd55fcb`.

Root reviews this finite source/evidence boundary, handles the incident ledger and specmap, and commits only if the A2 contract is accepted. Do not start A.3 from this candidate report.
