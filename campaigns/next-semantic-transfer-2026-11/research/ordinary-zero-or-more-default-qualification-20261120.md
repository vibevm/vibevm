# Zero-or-more defaults — omitted versus explicit empty qualification

Contract NST-ZERO-OR-MORE-DEFAULT-QUALIFICATION-01 (`a2e7106fb4bf11c9888f173a5217a4337a6ccd6a`), integrated at `2f7d4d169`, after accepted weak implementation `c81d7d6cc`. Test/report only. No new feature implementation, basis writer, schema or parent-state promotion.

## Four separate actual consumer worlds

Each fresh local ordinary flow project has one real workspace consumer, an unguarded zero-or-more `extra` group with nonempty declared default `[enabled]`, and an optional dependency activated only by that member. All four authored inputs use `default_features=false`. The test reads actual source/input, checks normalized omitted versus explicit member maps, runs the shared pure evaluator, and invokes the public offline installer even when the pure result is already known. `--assume-yes` is an operation flag, never the source of default consent.

| Authored input | Pure result | Actual public install |
| --- | --- | --- |
| Omitted answer, use-declared-defaults | enabled; declared-default use | Explicit unsupported refusal |
| Exact empty members, use-declared-defaults | Empty, no default use | Success, optional target absent |
| Exact enabled matching default, use-declared-defaults | enabled, no default use | Success, optional target installed |
| Omitted answer, require-selection | Empty, no default use | Success, optional target absent |

The success controls retain exact original consumer inputs, including presence of an explicit empty answer versus absence; automatic_basis remains absent and lock minimum remains epoch8. Explicit-match publishes target slot, boot marker and hook effect. The other successes publish no optional target. All success worlds cold replay without changed package/boot/lock state, extra hook runs or authored rewrite.

## Earliest missing ordinary behavior

Omitted/use-declared-defaults reaches the shared package configuration automatic-reliance guard in `crates/vibe-core/src/manifest/configuration/package.rs`, before slots, lock, boot or hooks. Exact public diagnostic:

`only unguarded exactly-one single-member defaults are connected`

The pure evaluator already distinguishes omission, explicit empty, exact matching selection and no-default policy. Public negative PASS therefore does not establish automatic zero-or-more delivery. The unsupported boundary is ordinary automatic default-basis admission/persistence, not a failure of core cardinality evaluation or explicit empty semantics.

On refusal, complete project byte/path snapshots permit only `.vibe` plus an empty `lifecycle.lock`; no blanket coordination filtering is used for that complete check. Registry bytes are unchanged in every world. Semantic snapshots used for success replay explicitly exclude coordination, which is printed separately.

## Smallest proposed next boundary

Extend the existing ordinary automatic-basis owner to retain a lossless default member set for one unguarded zero-or-more group, preserving original omitted input separately from effective selection and explicit empty. The existing consumer_choices string represents a single selected exactly-one member, not an arbitrary set; do not flatten multiple members into an invented string, use an explicit answer as a reliance surrogate, or weaken the current guard without a representation/compatibility decision. Reuse the current evaluator and basis review/update owners. The architect must bound the representation, prior reader/writer behavior and default-drift handling before implementation. No such implementation is authorized by this test-only packet.

## Evidence boundary

Selector: `default_review::zero_or_more_defaults_omitted_empty_matching_and_require_selection_public_boundary`.

Initial four-world public runtime PASS is retained in `zero-or-more-qualification-public-r1.log`; strengthened final assertions additionally pin pure selected/default-use results and complete refusal state. The r2 build caught test-only PathBuf lookup keys passed as str; corrected to Path without changing product behavior or assertions. Build RED is retained. Final build/runtime/lint/format and exact source/CLI/fixture hashes are sealed in `zero-or-more-qualification-candidate-r1.json`. Independent acceptance remains required.

Runtime uses the finite `python3 -I` bwrap supervisor with cleared environment, isolated filesystem/process/network, synthetic HOME/settings and exact read-only CLI/test/system runtime mounts. No credentials, auth stores, host homes, network, sockets or provider execution. No NST/REMAINDER/IDE/USER-PROJECT completion follows from these four worlds.
