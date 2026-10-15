# M-08-C.3 full-task review: changes required

Provisional review of the Root-named reported snapshot; no producer termination or acceptance claim. All 317 actual subjects matched before and after review; no drift. Findings below are source-derived counterexamples, not reviewer-executed reproductions.

## C3-F1 [P1] Bind the executed skill donor to the approved donor root

Location: [crates/vibe-lifecycle/src/mechanism/deploy/generic/adoption.rs:60](/fast/git/v/vibevm-next/crates/vibe-lifecycle/src/mechanism/deploy/generic/adoption.rs:60).

Prepare/approve standalone adoption using donor A in the recipient project. Independently installed identical skill in project B has the same relative donor ID and byte-identical standalone receipt. Pass donor B to adopt_skill with the approved A plan. The execution comparison accepts ID/family/bytes, locks/validates/holds/retires B, and publishes recipient ownership of A resources. Donor A remains active and unheld. This produces two active owners in A and removes ownership in B without the grant having approved B.

Source-derived counterexample, not a runtime reproduction. TransferPlan has no donor-root field; the preparation root check is not repeated at execution. SkillDonor::hold validates relative resource/digest rows but no recipient root.

Required correction: Retain and revalidate exact donor capability/root identity in the approved transfer through execution and recovery; reject substitution before any hold/retirement. Add one two-root identical-standalone-receipt negative fixture. Audit analogous deployment donor execution binding in the same correction.

Supporting references: generic/adoption.rs:21, generic/apply.rs:56, pkgskill/receipt/adoption.rs:380.

## C3-F2 [P2] Address actual legacy deployment homes by their recorded project identity

Location: [crates/vibe-lifecycle/src/mechanism/deploy/generic/adoption.rs:90](/fast/git/v/vibevm-next/crates/vibe-lifecycle/src/mechanism/deploy/generic/adoption.rs:90).

An incumbent deployment uses DeployExecution.project="example" with target T and an otherwise representable owned file/OpenCode resource. Its DeploymentHome hashes "example", package and T. deployment_donor(T, package) instead hashes the GenericHost physical root correlation token; it looks in a different directory and returns missing donor receipt. No public argument can provide "example".

Source-derived functional failure, not reproduced. The legacy/OpenCode fixture starts with GenericHost::apply and rewrites only receipt resource spelling, so it leaves the physical-token home and cannot detect the mismatch.

Required correction: Let the donor owner resolve/validate the exact existing logical project/package/target identity without moving or rewriting old receipts. Exercise a receipt produced by the incumbent deployment identity path, then import and restore through public donor APIs.

Supporting references: crates/vibe-lifecycle/src/mechanism/deploy.rs:739, crates/vibe-lifecycle/src/mechanism/deploy/model.rs:259, crates/vibe-lifecycle/src/mechanism/deploy/state.rs:136, crates/vibe-lifecycle/src/mechanism/deploy/generic/m_08_c_3_cases.rs:802.

## C3-F3 [P2] Recheck the named explicit arena rather than its retained handle alone

Location: [crates/vibe-lifecycle/src/mechanism/deploy/generic.rs:203](/fast/git/v/vibevm-next/crates/vibe-lifecycle/src/mechanism/deploy/generic.rs:203).

Host A retains an explicit arena at /arena. Ordinary replacement moves that directory and creates a new /arena while destinations and their capabilities remain unchanged. A fresh participating host B receives /arena from the same configured coordination location. A compares its old held arena token to its old plan token; B compares the new held token to its new token. Both checks succeed and the wrappers acquire different lock files. A generic writer and nested-root coordinated skill writer can then enter the overlapping destination domain together.

Source-derived freshness gap, not a runtime reproduction or hostile-process claim. SafeFS documents root_correlation_token as held-root identity and explicitly requires reopen/compare to detect pathname replacement. GenericHost current_roots reopens project/user/settings but omits a separate explicit arena; both coordinated wrappers only compare the retained arena to itself. Deliberately choosing unrelated arenas remains outside the coordination contract; this finding is ordinary replacement of the configured common arena.

Required correction: Apply existing supported current-name/root validation to the supplied arena before locking and recheck after acquisition, retaining its expected identity. Refuse the stale host before family/destination effects. One two-host ordinary replacement case suffices; no new lock primitive or security expansion.

Supporting references: crates/vibe-lifecycle/src/mechanism/deploy.rs:893, crates/vibe-agent-projection/src/pkgskill/receipt/adoption.rs:69, crates/vibe-safefs/src/project.rs:161.

## Full-task coverage and evidence limits

The candidate substantively implements typed retained state attached to existing transaction ownership, explicit desired/before states, private pre-effect blobs, resource-set upgrade, literal JSON/TOML inverse, separate current cleanup authority, receipt-scoped revocation, donor holds/retirement/restoration, and representative interrupted recovery. These are real paths, not just a schema/helper foundation. F1 breaks exact donor authorization and unique ownership; F2 leaves actual representable legacy deployments outside the advertised supported matrix; F3 leaves the explicit common-arena freshness contract incomplete. Therefore both acceptance criteria and the complete safe-stop claim remain unqualified.

Producer logs inspected: V1 19, V2 5, V3 4 plus 1 old exact, V4 13 C2 plus 8 incumbent exact cases: **50 successful nonignored Rust executions**, not 50 new cases. Reviewer ran no tests/builds. Scoped formatting and whitespace report exit0; the separate approved SafeFS formatting command reports exit0.

V5 observed codegen exit0, check-codegen exit1, wire-diff exit0. The failed check displays generated registry/index additions; wire-diff explicitly says its green result means allowed pre-publication change, not unchanged wire. Preserve those results. After correction and source integration, Root may run the single index-aware check-codegen gate; the present snapshot is not an integrated gate pass.

V7 original wrapper remains false because its broad baseline included the authorized new C3 schema. The separately executed old-only supplement reports protected old schemas unchanged, generated old bodies unchanged, TS manifest identical, additive prior PROP054 text and valid XML. This is a reasonable bounded preservation substitute for that misclassified input; it does not erase the original false result or prove unrelated gates.

## Additional bounded observations

- Scoped wrappers implement real existing writer calls; lack of upper CLI/MCP feature is not itself a finding because the accepted contract is host-library composition.
- The named nested-roots test only checks two arena capabilities and manually creates an unrelated destination parent; it does not by itself prove two nested-root writer integrations. Existing standalone contention case exercises one actual writer.
- SafeFS optional mode is set on staged handle before sync_all/hard_link; no-clobber and resulting mode checks remain. Prior-receipt assertion uses owned post-preparation baseline; production preparation sequencing unchanged.
- Canonical PROP054 adds arena/retention/cleanup/transfer/mode contracts and preserves prior text. Correction must align actual behavior with those contracts.
- No product edits, tests/builds, Git mutations, external sends, models, process control, boot or stewardship performed.

Hash evidence: [before](source-hashes-before.json), [after](source-hashes-after.json). Exact finding details: [REPORT.json](REPORT.json). Root owns producer collection, final stability, correction commissioning, integration and acceptance.
