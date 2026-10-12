# M-01-C.2 candidate — independently verified original-source membership

Base HEAD `6b487cbdff13165540218bdda3760601beac40cc`. Product diff is unstaged for Root review; no worker commit, public push, release, provider fetch, cache fill or rebuild. `FIXED-ISSUES.md` is a disjoint Root-owned WIP.

## Boundary implemented

PROP-070 now defines `#CAPTURED-SOURCE-MEMBERSHIP` and `#SOURCE-STRENGTH-LAW`. The original package source is a separate capture from the installed slot outputs. `vibe-install::verify_original_source_capture` derives canonical expected rows from retained `StableFileSnapshot` bytes and calls the **existing** `vibe_registry::compute_content_hash_legacy0_bounded_exact` over the complete original tree. That owner enforces no-follow replay, exact package pin/manifest, all listed children, two observations and its fixed limits (8 files, 2 MiB/file, 4 MiB total, depth 4, 12 children/directory). Only equality with the independently supplied selected full `ContentHash` mints `VerifiedSourceMembership`, whose fields are private. Recipe `sha256-tree/1:` is explicitly unavailable on this bounded route; no new recipe was invented.

The workspace `SourceMembershipIndex::from_captured` is now an **inert** byte catalogue: it validates snapshot and manifest shape but carries no expected hash and has no source-proven promotion API. The workspace B2 authority can report `Observed` or `RecordConsistent` after revalidating its current selected world and local slot record/output. Only install's opaque verified wrapper can return `SourceProven`, after rechecking that its exact package pin/hash equal the B2 selected lock and that the returned Mixed output's source path and bytes are members of its verified original index. Original-source membership is a separate method; transformed output remains at most record-consistent until C.3 proves conversion/overlay derivation. Self/development output without independent capture stays observed; strict missing capture refuses offline.

## Deciding public-route correction and matrix

Root/Astra found that my first public `from_verified_replay(pin,H,H,captured)` took a **caller-supplied** replay value. A fresh selected B2 world with coordinated slot payload+record tampering, plus a public catalogue made from those tampered bytes and forged `H,H`, returned false `SourceProven`: `public-seal-red.log`, **0/1**, exit 101, assertion showed `left: SourceProven`. This was a normal API composition defect. The same exact route is now 1/1 green in `install-c2-final.log`: the raw catalogue cannot be passed to the private verified wrapper, local output remains `RecordConsistent`, and the genuine original proof from pristine source cannot promote the coordinated tamper.

| Case | Measured result |
| --- | --- |
| Pristine complete original capture replayed against preselected H; Mixed slot unchanged | `SourceProven`; verified original DOC is a member |
| Self/development output; no independent capture | `Observed`; strict request unavailable |
| Selected slot with no original proof | `RecordConsistent`; strict request unavailable |
| Record-only SHA edit after preparation | B2 `StaleWorld("selected descriptor record")` |
| Payload-only byte edit after preparation | B2 `StaleWorld("selected output")` |
| Fresh B2 world after **coordinated** payload+record digest edit; selected H and original verified index fixed | Local `RecordConsistent`; strict source-proven request unavailable |
| Unrecorded slot document | Absent from original index; query has no prepared capture and cannot be source-proven |
| Original capture changed after its snapshots or independently selected H wrong | Canonical replay unavailable or `SourceHashMismatch` |
| Tree1 selected hash on the current bounded Legacy0 route | Explicit unsupported-recipe unavailable, no fallback/fetch |

The pristine H was computed once from the original source fixture before any slot mutation. Neither a slot-record source_hash nor a digest recomputed from the tampered candidate was used as the expected authority. Existing `RegistrySlotVerifier::verify_record` remains an installation-local record/payload check; its `SlotCheck::Verified` is not relabeled source-proven.

## Exact source files

| File | SHA-256 |
| --- | --- |
| `vibevm/vibespecs/common/PROP-070-resource-resolution.xml` | `07d42f024d79a544fbd304ed1aac8a39442212065cc18307ae9cf029fe875947` |
| `crates/vibe-workspace/src/source_query/mod.rs` | `a5938f478f81d058154d630c8376ff438726148f938a1b1b5c3a62cad7a0f0b5` |
| `crates/vibe-workspace/src/source_query/capture.rs` | `4c4f39262b5ee7c562e16cae23bf0e4c9ca5a452aba990e2806094d47069962a` |
| `crates/vibe-workspace/src/source_query/capture/continuity.rs` | `3aa045e09b8072148f3eefd1fc73ab47168c41d9fe32b840921826c64e2d47db` |
| `crates/vibe-workspace/src/source_query/capture/continuity/membership.rs` (new) | `101e647bc8406eeeeee217a12f3e0758e8be1477b2a6d05a587d198a37c45525` |
| `crates/vibe-workspace/src/source_query/capture/continuity/m_01_c_2_cases.rs` (new) | `e3f9cb51c61bb32f015033b090bb8207b118b19d141b5fd442c93b94b80fe81c` |
| `crates/vibe-install/src/lib.rs` | `7e16ee95d93cded18d7ba8bd55c4074a378398e472dba394b32a4b3b74fd8a57` |
| `crates/vibe-install/src/slot_verify.rs` | `2a8db2d17b1ff7594bfe962e270a1ca9344a9f9e4630c046f686288d0265fc6c` |
| `crates/vibe-install/src/slot_verify/m_01_c_2_cases.rs` (new, Root-granted test-only child) | `da0984fe78b5fdc2f8097ac8ac04b2e08c367961be8a6bb447663364b887722b` |

All touched Rust files are below 600 lines; the largest is `slot_verify.rs` at 542. No Cargo dependency graph or lockfile version/hash changed.

## Selected checks

All Cargo runs used `/fast/git/v/vibevm-next`, `CARGO_TARGET_DIR=/fast/git/v/.build/c4-first-agent-target`, `+nightly --locked --offline`. `vibe-install` has `lib.test=false`, so its new source-owner tests run through the existing `--test slot_integrity_verify` target that includes `slot_verify.rs`. Every selected run below executed nonzero tests with zero ignored.

| Target / selector | Result | Log SHA-256 |
| --- | --- | --- |
| `vibe-install --test slot_integrity_verify production_slot_verify::m_01_c_2_cases` | 4/4 selected-world matrix | `install-c2-final.log` `d3a89cef1a23d78391023d536162609b8ad0e19e6d3de59202e121541511277f` |
| `vibe-install --test slot_integrity_verify production_slot_verify::source_membership_cases` | 3/3 canonical replay, wrong H/source, unsupported recipe | `install-replay-final.log` `638b1151bf70e8d99630288c6ebe93b302cfad2ffbc6906af4ea3311a4d152b6` |
| `vibe-workspace --lib source_query::capture::continuity::membership::m_01_c_2_cases` | 1/1 inert catalogue shape | `workspace-c2-sealed.log` `b7c5fc1f32d67665eef40b8903e353241cc56067c1dba375ac344a441acd8399` |
| `vibe-install --test slot_integrity_verify verify_accepts_untouched_slots_without_copying -- --exact` | 1/1 local slot compatibility | `install-slot-compat-final.log` `50db452f2981fb2ab1339cacb91805218c1a47fe5caacbd970433e6365fd6e28` |
| `vibe-workspace --lib source_query::capture::continuity::m_01_b_2_cases::unrelated_broken_member_does_not_poison_selected_point -- --exact` | 1/1 B2 compatibility | `workspace-b2-compat.log` `18985ef075e613e7ca9acae63c682eaa585aea5d2d424dd1b8232d5fb0ff065b` |
| `cargo +nightly fmt --package vibe-workspace --package vibe-install -- --check` | exit 0 | `package-fmt.log` |
| `git diff --check` | exit 0 | terminal check |
| `cargo +nightly fmt --all -- --check` | exit 1, same 15 unrelated hunks | `full-fmt.log` `24acc9dabcadbeeb2ce4bd2b6acd0cdabc70f59e492ed613b7da7a39c1a3eba1` |

All actual failures are retained here: `public-seal-red.log` is the reproduced product failure, and `full-fmt.log` is the pre-existing unrelated baseline. No setup/compile failure or zero-test pass was hidden. Root owns final specmap and FIXED-ISSUES integration.

## Limits and Root next action

The bounded replay currently supports only the frozen Legacy0 `sha256:` recipe and its explicit small-tree limits. Tree1 and larger original packages remain unavailable for strict source proof; non-strict output can still report its honest weaker strength. The proof trusts the existing selected-world preparation owner and the canonical replay; it is not cryptographic attestation of arbitrary Rust callers. SafeFS/B2 observations are detection-bound, not atomic or hostile-ABA proof. Source-proven output is limited to exact original Mixed bytes. A transformed slot may have a verified original member, but its returned bytes remain at most record-consistent until C.3. Native Windows/macOS and broad release qualification remain unperformed; full workspace fmt is not green. Root should review the real public-route red→green and matrix, integrate incidents, then accept/commit atomically if satisfied. No C.3 or later atom was started.
