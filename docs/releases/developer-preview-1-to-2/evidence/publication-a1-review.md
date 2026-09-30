# M-07-A.1 exact publication subject and plan

Accepted local boundary: the prepared subject binds the selected committed
source/tree and bounded Cargo.lock bytes, all five platform distribution
triples, aggregate and installer bytes, smoke observations, and fixed
`github.com/vibevm/vibevm` destination. A pure effect plan binds that subject
digest to an owner-supplied complete remote observation, its generation and
replaced release ID, explicit draft/publish choice, and every intended tag,
release, asset, cleanup, and finalize partition. An already published mutable
1.1.0 release requires a separate explicit replacement-plan choice; this is
plan data and grants no effect authority.

The `vibe-publish` subject and effect-plan constructors have no filesystem or
network handle. The `xtask` capture only reads a selected Git commit and a
pinned local output root. Its tests compare output and source bytes before and
after capture/plan construction. No remote client or publication writer is
called here; observation is typed input. A live remote observation, approval,
journal and effect fence remain later M-07 work.

Focused evidence at `b80998759`:

- `cargo test --locked --offline -p vibe-publish --lib publication_`: 22 passed,
  including independent raw valid/invalid plan corpora, changed lock/asset/
  target/release generation, incomplete capture and smoke readiness refusal.
- `cargo test --locked --offline -p xtask --bin xtask dist::capture::tests`:
  7 passed, including byte-stable local outputs and published-release planning.
- `cargo fmt -p xtask -- --check` and `git diff --check`: exit 0.
- Schema registration/code generation was checked after the schema change;
  the generated format modules are used by the above decoding cases. No
  `vibe-wire` test name matches `publication_` (filtered selection: 0 tests).

This accepts pure planning only. No public Git push or remote publication was
performed. The campaign's broader specmap check currently has three unrelated
M-14 citation suspects and is not claimed green by this review.
