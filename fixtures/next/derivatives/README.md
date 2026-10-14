# Local derivative design fixtures

These five scenarios specify required behavior; they do not implement a derivative service. `base/` is immutable captured example data, `local/` is separately owned authored content, and `expected/` is a reviewable output oracle. None is an installed package. The JSON vocabulary is fixture metadata, not production configuration or permission syntax.

All code-bearing payload bytes are inert: never run any base/local/expected file. No source grants transfer. Every case has a sealed base and absent demand evidence yet requires separate-derivative eligibility. `manifest.json` lists later executable service obligations and the exact missing owner behavior.

Run `python3 fixtures/next/derivatives/validate.py` from the repository root for inventory, hash, XML and scenario checks. `pure_seams.rs` is a standalone validation driver (not a Cargo test module); compile it against the already-built `vibe-specdoc` library and run it from the repository root. It only passes captured fixture bytes to existing pure parsers and preservation APIs. It never executes fixture payloads or publishes output. These validations do not satisfy future service lifecycle cases.

The tree inventory fixture hash is SHA-256 of compact, sorted-key JSON rows including path, byte count and file SHA-256. It is deliberately not a production content-hash recipe. Fixtures use regular files and portable paths; future path/collision refusal tests remain implementation obligations.

M-10-A.2 adds the bounded Rust service harness in
`crates/vibe-workspace/src/derivatives/m_10_a_2_cases.rs`. Run
`cargo test --offline --locked -p vibe-workspace --lib m_10_a_2_cases` with the
session's authorized target directory. Six cases cover the common lifecycle for
whole-file CommonMark and opaque trees, retained authored provenance after output
and cache deletion, stale bases, invalid inventories, and passive effects.
The harness mounts each inventory under `payload/` using the existing M-16
SourceDescriptor and captures regular files through the M-01 CapturedSourceFile
carrier. Its typed example package pins do not authenticate these fixtures.

`DerivativeService::apply` returns data for a downstream consumer; it does not
install, hydrate, deploy, execute, grant authority, or write a receipt. The
separate existing projection reconciliation test checks neighbor preservation;
it is not evidence of derivative-to-projection integration. The service owns no
filesystem writer or persistent store. Caller-owned authored storage is a
precondition, and output borrows those independent inputs. Semantic export edits,
production capture-proof promotion, all-five-family feature completion, durable
publication and consumer integration remain later work. The design validator's
`future_service_cases_executed: 0` describes that Python validator alone.

M-10-C.1 adds actual local publication through
`vibe_workspace::derivatives::MaterializedDerivative`. Its `derive` validates
retained base/local captures against current no-follow bytes and complete selected
membership, exclusively creates a disjoint result directory, stages all files
under `tree/`, and publishes the existing strict `result.json` record last using
SafeFS create-new publication. The reader requires that record and revalidates
the full output inventory. This is cooperative local publication, not a sealed
filesystem transaction or production source proof. An interrupted directory is
reported as residue and is never adopted; rebase/apply recovery remains M10C2.

The authored binding TOML and local files live independently from that disposable
directory. Deleting it permits recreation with the same result identity. Show
retains both complete raw inventories and exact SHA-256 digests; Vibe inspection
adds a separately labelled canonical Markdown projection and digest or diagnostic.
Diff carries both sides of every addition/removal/replacement, including binary
data. Ordinary XML and scripts are copied without conversion or execution.

`service_fixture.rs` mounts every manifest resource on disk for the selected
`--lib m_10_c_1_cases` families in vibe-workspace and vibe-agent-projection.
The vibe-spec family verifies independent upstream/local addresses and projection
bases. These Rust tests are distinct from the design-only Python validator.
Projection preparation consumes the complete materialized result without
hydration; tests invoke receipt reconciliation separately to check ownership and
unowned-neighbor preservation. Derivation itself never installs or activates.
