# Universal dependency ownership: executable recipe decision

Date: 2026-11-19. Addendum to CONSUMER-BOOT-VIBEDEPS-01.md.
Architectural decision, not permission to change execution lane. Builder implements in its assigned checkout; architect reviews. No additional architecture-selection/PlanMode cycle is needed for this bounded task. Actual harness approvals still bind.

## Decision

Preserve `render_contract(1, supplied)` byte for byte, including its existing golden. Add recipe 2 and make it the default (`CURRENT_RECIPE = 2`, `KNOWN_RECIPES = &[1, 2]`). Unknown recipes still refuse. No manifest grammar or sidecar schema addition.

Recipe 2 includes the exact compact instruction:

> Edit sources, not materializations. Never hand-edit `vibedeps`; change its canonical inputs and let Vibe regenerate it.

An explicit recipe 1 pin preserves recipe 1 transport-contract prose, NOT an exemption from the newly owner-required universal boot policy. For that case the composer adds the same instruction separately as generated policy text adjoining the frozen rendered contract. This is an intentional composed-prefix change: record it plainly, never claim historical whole-prefix identity. Do not silently ignore the recipe pin, rewrite its golden, or refuse otherwise valid recipe 1 projects.

## Owners and order

1. Inspect the actual task base/branch and preserve unrelated dirty candidates; record full base hash in the implementation receipt. Use the applicable approved isolated coding lane; this brief does not authorize direct product coding or a substitute launcher.
2. Introduce ONE internal constant/helper for the exact instruction, accessible to the existing router and system-prompt owners. Prefer a small existing shared boot owner or a narrow private module; no new public framework/package dependency. Keep literal wording single-owned.
3. `crates/vibe-workspace/src/boot_artifacts/redirect/router.rs::render_router`: include the rule within the selected-root managed boot body, after the existing root/quiet scoping and before optional personal boot. Cover static-lane and no-static-lane branches identically. Do not alter neutralized dependency copies into active routers.
4. `crates/vibe-workspace/src/system_prompt/contract.rs`: retain the recipe 1 implementation unchanged; dispatch 1/2 explicitly, add recipe 2 with its correctly identified generated header and the shared instruction. Avoid search/replace over already-rendered prose. Defaults flow through the existing `PrefixSettings` owner.
5. `crates/vibe-workspace/src/system_prompt/compose.rs::compose_prefix`: for explicit recipe 1 only, construct the replacement generated text from frozen `render_contract(1, ...)` plus the adjoining shared policy paragraph. Feed the combined owned text through existing `entrypoint_parts`; the generated range must cover BOTH parts. For recipe 2 use its render directly, without adding a duplicate. Preserve entrypoint=false and absent-managed-block handling. No new segment id or metadata field is necessary.
6. Native launch already loads the instruction entrypoint and `AGENT-LANE.md` carries source files, not the router. Verify the actual launch owner in `crates/vibe-cli/src/commands/agent/launch.rs` and supported carrier paths retains the updated router or a policy-equivalent composed prefix. Do not inject a fake source row into `system_prompt/lane.rs`, change source marker identities, or add router text to the source-only lane merely to satisfy a string test. If a carrier actually substitutes/discards the router, carry this same owned policy at that substitution owner and test the final delivered input. No second full boot generator.
7. Append a new unused SP-FMT id in `campaigns/sys-prompt-2026-11/FORMAT-CHANGES.md` AFTER checking the builder tree for concurrent reservations. Explain default 2, pinned-1 contract preservation versus intentional prefix change, classic-router change for non-opted-in consumers, and actual registered-format/epoch disposition. The old standing no-opt-in-no-byte-change note remains historical and is explicitly superseded ONLY for this universal router instruction. Record recipe behavior in the existing PROP-076 transport-contract fact; refer to PROP-009#CONSUMER-BOOT-DEPENDENCY-OWNERSHIP. Do not rewrite old evidence as though recipe 2 already shipped then.
8. Run focused gates below, review exact diff, integrate under existing scoped commit permission. Publish accepted SYS-TAILS consumer routers through ordinary Vibe regeneration only AFTER generator acceptance; reuse valid graph/tool proofs. Update authoritative source-plan evidence before dashboard refresh.

## Finite acceptance and tests

- `system_prompt/contract/tests.rs`: old recipe 1 golden remains identical; new recipe 2 golden; known/default/unknown diagnostics match 1,2/default2; both retain no volatile fields/imports/boot commands.
- `system_prompt/compose` tests: default/explicit2 rule present once in owned transport text; explicit1 frozen contract substring unchanged plus one adjoining policy; entrypoint present/absent/false; authored prefix/suffix preserved byte-for-byte, including CRLF; no escaping of generated-range recursion verification. Authored text independently repeating a rule is not a reason to strip it.
- Router tests: all supported instruction filenames; zero dependencies and dependency-bearing consumers; with/without static lane; no SYSTEM opt-in; root/quiet scope intact; authored bytes untouched; repeat regeneration preserves bytes and mtimes.
- Publication/launch focused tests: actual final instruction input retains policy when managed-block substitution occurs, for read and launch delivery and explicit1/default2. Assert applicable ownership, not only unrelated on-disk string presence. Existing marker/source identities and source-only agent-lane tests remain valid.
- No model/provider calls or rerun of historical cache/live-agent matrix is required for static wording. No builds in architect checkout or peer writes by architect. Builder runs affected crate tests and existing format/conform gates appropriate to changed owners; report exact commands, results and unrun checks.

## Exclusions / stopping condition

No lock schema change, materialized dependency hand-edit, whole-host forced reinstall, optional-layer redesign, provider/auth/config mutation or new launch framework. Finish once universal actual delivery, frozen contract compatibility, deterministic regeneration and focused gates are demonstrated. Execution-lane unavailability is a blocker, not permission to bypass it.
