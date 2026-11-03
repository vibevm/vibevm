# F01 — resolver lexical containment

Implemented in `/fast/git/wt/next-semantic-transfer`, based on main `6d2f6e59be3bf55ac85fb2db67ad4e80b6925d79`. Product contract is root-owned `PROP-035#URI-LOOKUP-PATHS`; I read its decision section and did not modify it. Read the named standing file and Rust guide/cards. Cards were located under `vibevm/vibespecs/cards/`, rather than `rust/cards/`; the initial attempted read of the latter failed before any edit.

## Change and rationale

Four source files changed/added:

- `crates/vibe-spec/src/resolver.rs`: `InvalidLookupPath` carries the input field, spelling and existing typed `DeclarantPathFault`; its Display uses the existing fault's reason and permanent REQ/fix-surface grammar. `resolve_source_file` validates document and active suffix spellings before selecting or probing a source root. `package_slot` validates the complete joined `group.name` component and explicit version before joining/probing filesystem paths.
- `crates/vibe-spec/src/resolver/glob.rs`: validates pattern document path before enumeration, including when dependency slots are absent. The existing early identity return for non-pattern expansion remains.
- `crates/vibe-spec/src/resolver/paths.rs`: small internal adapter over existing `vibe_core::manifest::declarant_path` and `declarant_path_component`; no new path grammar or normalization. A custom frontend suffix must form one safe `source.<suffix>` component. Validating the joined slot instead of each package name separately preserves `org.demo.con`.
- `crates/vibe-spec/src/resolver/path_tests.rs`: ten regressions/compatibility cases. Includes actual public `FsSectionSource` read of an outside sentinel before the fix, absolute document/slot and version escapes, custom suffix traversal, absent-root pattern validation, portable invalid spelling matrix, and selected-root/non-pattern/Unicode/percent/con-slot controls. Three behavioral tests carry explicit verifies edges.

No changes to parser, crate manifests/dependencies, `lookup.rs`, source-query/proof machinery, machine framework, specs, generated maps or commits. `resolver.rs` is 499 lines; no unrelated split/refactor was needed.

## Observed verification

Commands ran in the worktree with its existing local `target`, without environment/global configuration changes. Durations below are `/usr/bin/time` wall seconds; counts come from test output.

| Exact command | Result | Wall seconds |
| --- | --- | ---: |
| `/usr/bin/time -f 'ELAPSED_SECONDS=%e EXIT=%x' cargo test -p vibe-spec --lib resolver::path_tests::` (tests only, pre-fix) | exit 101; 7 tests, 1 passed and 6 behavioral failures | 51.92 |
| same command (first implementation compile) | exit 101; compilation E0599, no tests ran | 1.99 |
| same command (fixed diagnostic, expanded ten-case family) | exit 0; 10 passed, 0 failed, 966 filtered out | 6.59 |
| `/usr/bin/time -f 'ELAPSED_SECONDS=%e EXIT=%x' cargo test -p vibe-spec --lib resolver::` (final annotated/formatted code) | exit 0; 52 passed, 0 failed, 924 filtered out | 3.28 |
| `/usr/bin/time -f 'ELAPSED_SECONDS=%e EXIT=%x' cargo test -p vibe-spec --lib embed::` | exit 0; 26 passed, 0 failed, 950 filtered out | 0.21 |
| `/usr/bin/time -f 'ELAPSED_SECONDS=%e EXIT=%x' rustfmt --check --edition 2024 --config skip_children=true crates/vibe-spec/src/resolver.rs crates/vibe-spec/src/resolver/glob.rs crates/vibe-spec/src/resolver/paths.rs crates/vibe-spec/src/resolver/path_tests.rs` | exit 0 | 0.05 |
| `/usr/bin/time -f 'ELAPSED_SECONDS=%e EXIT=%x' git diff --check -- crates/vibe-spec` | exit 0 | 0.00 |

Formatting was applied with `rustfmt --edition 2024 --config skip_children=true` to those same four files before the final checks; exit 0. No full workspace/package test suite, clippy, conform or specmap execution was performed. The targeted lib tests compiled the affected code; root owns conform/specmap. Cargo emitted the existing `crates/vibe-cli/Cargo.toml: unused manifest key: build` warning each time.

The failed first implementation compile was my mistake: I assumed `DeclarantPathFault` implements Display. It exposes `reason()` instead; the final thiserror variant calls that method while retaining the typed fault.

### Behavioral RED details

Before production changes:

- `FsSectionSource::section_text` returned `Ok("# Secret\nOUTSIDE_SENTINEL")` for `spec://org.demo/self/../SECRET`.
- Custom suffix `/../../outside/SECRET` returned `Ok(ResolvedSourceFile { path: ".../spec/NOTE./../../outside/SECRET", ... })`.
- Manually constructed absolute document/slot and `../outside` explicit version all resolved instead of refusing.
- Invalid pattern document path with no slots returned success/empty expansion instead of refusing.
- Existing valid nested Unicode/percent-literal and `org.demo.con` slot control passed.

Final resolver family also passed active custom exact/ID-prefix lookup, custom/builtin collisions, scoped canonical suffix behavior, selected-world version agreement, optional newest-version selection and all existing pattern tests. Embed family passed overlay, XML projection, recursive expansion, cycle/refusal and compiler embed closure cases.

## Proven scope and limits

Proven locally: lexical path spellings cannot use the exercised existing resolver boundaries to leave the chosen document root or joined package/version slot; invalid pattern document spelling is refused before absent-slot enumeration; old valid routing and custom frontend tests remain green. Percent sequences remain literal and no public parsing compatibility changed. A caller-supplied selected root may legitimately lie outside the workspace; it remains its authority and is not reinterpreted.

Not claimed: no-follow/symlink containment, hardlink ownership, case/NFC alias equivalence, filesystem race/ABA prevention, held-byte integrity, ambiguity/pattern-error propagation, public source proofs or broader source-query semantics. Those are distinct future atoms. Platform-independent lexical cases ran on this Linux host; no native Windows/macOS behavior was executed. Existing diagnostic messages for unrelated errors remain unchanged.

Root-owned spec/campaign files were already dirty before work and remain outside this change. No commit or cleanup was attempted; root owns acceptance and worktree lifecycle.

## Central review and additional verification

Reviewed the complete four-file patch. The new helper delegates to the existing portable path law and validates the joined slot, so it does not introduce a second filename table or reject a safe `org.demo.con` directory merely because the package name is `con`. The public parser remains unchanged; filesystem use is the validation boundary. Selected roots and literal percent sequences retain their authority/meaning.

Central clippy `cargo clippy -p vibe-spec --lib -- -D warnings` passed. Scoped conform found0new findings. Two exact compiler frontend consumers passed independently: `compiler::pass_tier::frontend_discovery_tests::worklist_reaches_a_raw_sentinel_and_inactive_formats_read_nothing` and `compiler::pass_tier::frontend_execution_tests::unselected_custom_format_refuses_before_alternate_source_read` (1case each). The existing broad test families are reused from the verified worker run; no whole-product pass is claimed.

The initial map rebuild exposed baseline XML dialect/scoping defects, repaired separately in c990b2900. F01 source bytes were retained with SHA256 while that baseline was committed, then restored byte-for-byte; only its spec status changes from specified to implemented. Final map check is recorded in the campaign log. Broader safe reads, ambiguity, held-byte proof and the complete source-query user feature remain open.
