# M-01-E.1 candidate — Root review pending

## Boundary

Current HEAD/base `23dce2e7185aa6b30c34d29f0fee54a7f15e1961` (accepted D2). Sole writer; no staging/commit or next atom. Product candidate has exactly five paths listed in SOURCE-SHA256.json. Root-owned FIXED-ISSUES.md is a separate documentary delta and was not edited here. No production reader/resolver behavior or dependency/schema changes were necessary: this is characterization plus test registration and governing clause `PROP-070#SOURCE-OBSERVATION-FIXTURES`.

## Evidence

- Early actual resolver/held-buffer consumer: **1/1**, exit 0 after correcting wrong selector. Its earlier exit-0/zero-test log is retained and not counted.
- New SafeFS family: **5/5**, exit 0, zero ignored.
- New resolver family aggregate: **6 distinct cases passed**, zero ignored. Five unchanged cases pass in spec-e1-final.log (overall exit 101, 5/6), and the only changed failed assertion passes in spec-e1-anchor-corrected.log (exit 0, 1/1). This is an aggregate, **not a claimed single green 6/6 invocation**. The corrected body merely binds the exact incumbent ambiguity/path diagnostic. No broader optional repeat required.
- All **11 new cases** plus one representative are nonignored Linux execution. Full paths/types/bytes/link targets, permissions and Unix inode/device/link counts are compared across queries. Atime intentionally excluded. Race injections themselves alter owned fixtures; tests compare the captured post-injection state, not falsely claim no race mutation.
- Narrow final rustfmt: exit 0. XML parse and unique anchor check: exit 0. git diff --check: exit 0. Rust owner lengths: file.rs 230, m_01_e_1_cases.rs 199, resolver.rs 576, m_01_e_1_cases.rs 222, all below 600.
- Full workspace fmt: exit 1, **13 pre-existing diff entries** across 8 untouched files; no E1 finding. No global green claim or baseline cleanup.

Exact commands, cwd/env, exits/counts and raw log SHA256: COMMANDS.json. Complete changed-source hashes: SOURCE-SHA256.json; source/evidence files are sealed by SHA256SUMS. Host: Linux x86_64, uid 1000; rustc 1.101.0-nightly (c36f14571 2026-10-01). Cargo uses one existing private target, locked/offline; final deciding runs use /tmp and absolute manifest. The fixture-correction run from repository cwd is explicitly recorded as procedure error.

## Native and portable inventory

BOUND-PLAN.md maps every case to incumbent helper/consumer and expected refusal. Linux native: regular nested source/held digest and embed, symlink+hardlink ownership refusal, equal-byte distinct-object rename after held open, cap growth, real read_dir(file) failure, and cancellation of authorized 4 MiB held stream. Cancellation yields exact Uncertain(dropped-before-completion); complete control reads exact digest/size. It proves local reader/drop behavior, not remote delivery or service cancellation transport.

Portable on Linux: traversal/drive/UNC/ADS/device/trailing-dot/control lexical refusals; actual differently spelled case/NFC fixture names; deterministic existing final-identity mismatch hook. These are **not native Windows/macOS execution**. Windows reparse/ADS/device native and macOS native primitives remain unperformed/open for appropriate hosts. Native permission-denied enumeration was not run: ENOTDIR is the bound actual enumeration failure, distinct from chmod/permission proof.

## Failures and limits

INCIDENTS.md preserves shell redirection before Cargo, zero selector, E0277 anchor-vector fixture errors, two measured diagnostic-assertion failures (one missed multiline replacement), truncated read attempts and cwd procedure error. Duplicate anchor already refused correctly; no observed production defect was hidden or fixed by weakening source semantics. Existing raw/proof/platform limits persist. Finite held pre/post checks do not prevent hostile ABA or mutation after the last check; query creates no lock/directory/COW publication. B227/B229/B230/B231 and historical formatting/orphan debt remain deferred. No codegen/format ABI changed; no conditional generator/wire oracle needed. Root owns final specmap and acceptance; managed review remains known ZAP009, not reprobed or replaced by private worker.

## Next action

Root reviews this unstaged five-file candidate and hashes, runs its final specmap, and decides acceptance/atomic commit. No E2 work started.

Current read-only Git snapshot:
```text
 M FIXED-ISSUES.md
 M crates/vibe-safefs/src/file.rs
 M crates/vibe-spec/src/resolver.rs
 M vibevm/vibespecs/common/PROP-070-resource-resolution.xml
?? crates/vibe-safefs/src/file/m_01_e_1_cases.rs
?? crates/vibe-spec/src/resolver/m_01_e_1_cases.rs
```
