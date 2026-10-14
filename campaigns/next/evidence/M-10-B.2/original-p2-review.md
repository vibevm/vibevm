# M-10-B.2 independent candidate review

**Changes requested: one P2 admission-decision defect.** No builds/tests, provider/model/service calls, source edits or commits. HEAD remains `8b096070cd348c51cdf8b3bcb2c494895d40f9fd`; unrelated Root BACKLOG is excluded. Root owns acceptance.

## P2 M10B2-P2-001 — effective provider is not rechecked when consuming admission

Location: `crates/vibe-lifecycle/src/mechanism/deploy/derivative.rs:130-138`.

`DerivativeAdmission::bind` resolves the target against the supplied registry/routes and requires the builtin handler (`:80-89`). Its carrier stores result identity, the authored DeployTarget and artifact, but no selected provider/basis. Later `decide` receives a separate DeployExecution; it checks selected target ID and structural descriptor equality, then host preflight checks roots, principal and generation. Neither operation rechecks registry/routes.

Concrete fixture-level counterexample: bind the ordinary target (`provider=None`) with default builtin `deploy:vibe-bin`; then pass an execution with identical target, result, roots and grant but route that key to an uninstalled provider. The existing resolver would refuse this current selection, yet decide follows all its checks and returns AdmittedAtFixtureStop. An installed native override is the corresponding outside-builtin example. Resolver precedence is explicit in `vibe-extension-registry/src/mechanism/resolve.rs:270-281`: target pin, host route, builtin default. `binding.rs:417-457` does not retain the omitted selection check.

This violates the decision's own builtin-only boundary and the M10B2 requirement for exact current descriptor/admission. It is a false positive **at the promised fixture safe-stop**; no live provider launch is demanded. The existing4 lifecycle cases vary result bytes, artifact identity, descriptor ID, binding, roots, principal and generation, but do not vary effective provider selection.

Smallest remedy: re-resolve through the existing owner when deciding, retain/compare the approved provider identity where needed, and refuse unavailable/disabled/replaced/non-builtin selection before acquiring destination infrastructure. Add a same-target changed-route negative case. No duplicate registry, trust boolean or new grant engine is needed. This finding follows the direct code path; I did not execute a reproduction.

## What is sound in the inspected candidate

- Creation remains independent: BoundDerivative is retained on compatibility refusal; actual local inventory and CommonMark parser headings determine Compatible/Incompatible/Unknown. Explicit alternative heading selection works; absent/unsupported targets remain Unknown.
- All five families have byte-inventory inspection using actual retained bytes. The spec and report explicitly avoid claiming semantic XML/Vibe/native ABI qualification from inventory presence.
- Package-skill inspection joins the result coordinate and complete selected bytes to an actual lowered binding, then uses existing selection/frontmatter validation. The receipt writer still refuses an unowned destination and preserves FOREIGN.txt. Inspection performs no hydration/projection.
- Lifecycle positive case uses incumbent typed `Fixture`, engine `build_record`/`write_record`, `resolve_bound_project_artifact` and non-cloneable `AdmittedLocalDeployBinding`. Current artifact bytes are measured and compared with retained result bytes; original/old-result identity, stale artifact, copied host DTO, principal/root/generation changes refuse in covered paths. This is real typed fixture admission, not invented production capture proof.
- Result admission is consumed as a diagnostic decision. No payload/provider is run or deployed. Lock infrastructure in disposable roots is explicitly disclosed. Signatures/install state cannot construct this wrapper without the incumbent host carrier.

## Integrity and evidence

Candidate report matches its published artifact digest/length:163355 bytes, `1d80ed0595bef6755f3711e5dfd3df355e545708e32f41758210fd60fa2d1d11`. Patch matches46342 bytes, `a899ef727c01afff3c418cf9e92dab33aa329fd071c8a8b7d0fd55cd2ba5ad43`.

There was no separate final source-hash manifest. I reconstructed each SHA-bound patch output in memory against HEAD, including all six new files, and compared exact bytes with all10 current task files. All matched. Captured source SHA256 values were checked again unchanged after review and are retained in the JSON.

Final raw logs corroborate workspace3/3, projection2/2 and lifecycle4/4, all0failed/0ignored. Full test bodies and actual assertions were inspected. Earlier lifecycle compile failure and workspace role-fixture2pass/1fail remain disclosed; no rerun here and no failed epoch was promoted. The successful log families do not cover the changed-route defect.

Full PROP074 was read. In-memory XML comparison confirms every existing section unchanged and exactly one additive COMPATIBILITY-AND-ADMISSION section. No five-family obligation or previous safety boundary was removed; no anchor-count-only inference. All10 files are within task perimeter. No schema/wire/manifest/dependency change triggers codegen here. New production owners are95,70,168 lines; tests212,144,265. Large existing registration owners only gain module glue; they were not newly enlarged into a large implementation.

The positive safe-stop is meaningful, but the missing current effective-provider check blocks accepting the claim that this exact fixture admission is complete. Native/plugin/skill activation and the full general derivative lifecycle remain separate work, not new demands imposed by this review.
