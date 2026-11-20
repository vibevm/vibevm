# Ordinary host-dependent Path: base and cold-replay contract gap

Read-only source qualification at `618b0e636c588e5fb988e2d32d878d0cafee8e46`, under the architect's next NST obligation. No implementation, arbitrary parameter-target observation, synthetic runtime substitute or external agent execution.

## Answer from the admitted sources

**The inspected ordinary contract does not select declaring-consumer directory, selected project root or process cwd as the base for `Path must_exist=true`, nor define its host observation/revalidation policy on cold replay.** This is a missing architectural decision, not a failed existence test or permission to infer a base. Current conservative refusal remains correct until that decision is made.

| Exact source / owner | What is established | What it does not establish |
|---|---|---|
| PROP-069#PARAMETERS-ARE-VALUES | Host-dependent path validation belongs to the actual planning host; shared library must not inspect ambient environment for choices/dependencies | No relative base, containment, symlink/existence policy, observation capability or ordinary replay rule |
| PROP-069#AUTHORED-INTENT / PROFILE-INPUTS / PERSISTENCE / RECONFIGURATION | Real declaring workspace consumers supply identity; originals/origin and consumer agreement are preserved; changed definitions validate before effects | A consumer diagnostic string is not a filesystem root, access grant or persisted absolute path realization |
| `NST-ORDINARY-PUBLIC-PATH-QUALIFY-01.md` | Do not infer base/access from cwd; tests synthetic only; identify actual host owner before proposing support | No authority to pick a base or inspect real user paths |
| `NST-ORDINARY-PUBLIC-PATH-LEXICAL-FIRST-01.md` | Only `must_exist=false`; preserve exact spelling without cwd/project join, expansion, canonicalization, IO or relative→absolute persistence. Cold replay after target disappearance must remain lexical | Explicitly leaves `must_exist=true` unsupported until host base and separate check are specified |
| `manifest/package/application/wizard.rs:ParameterDecl::canonical` | Path validates nonempty/control-free text; lexical result is the authored spelling | Does not perform existence/host-root validation, including when must_exist is declared |
| `manifest/configuration/package.rs` Path branch | First ordinary evaluator refusal explicitly requires a separately specified planning-host base/check | Present target, current cwd or project-relative coincidence cannot waive it |
| `vibe-install/src/plan/roots.rs:RootRequirements::configuration_inputs` and `plan.rs:configured_workspace_inputs` | Actual declarations yield package→consumer→original input, using relative manifest occurrence keys, not made-up owner labels | Keys are not parsed into trusted filesystem capabilities; they carry no host witness or ordinary absolute realization |
| `vibe-resolver/src/configuration.rs:ConfigurationSnapshot` | Candidate/original/previous state and source observations are retained for solving | No Path host-base/capability carrier. Source authority is not arbitrary parameter-target authority |
| `vibe-package-source/src/project_local.rs:project_packages_root` | Surface resolves a project root for discovery of the project's package registry | Existing project-root availability is not approval to use it for parameter realization |
| `vibe-workspace/src/freshness/configuration.rs:configuration_freshness` | Original input and validated retained configuration participate in freshness | No ordinary host-path observation or missing-target replay semantics |
| PROP-069#APP-CONFIGURATION-BASIS; `vibe-app/src/configuration/parameters.rs:canonical`; `steps/plan.rs:HostFacts` | Separate app owner has host resolve_path, retained previous realization and must_exist check. Unchanged source/origin may retain old realization; portability does not grant target readiness | Does not prescribe the ordinary dependency owner's base, store a missing ordinary witness, or authorize copying app state/exists behavior into the pure kernel |

Accepted earlier reports `ordinary-public-path-qualification-20261120.md` and `ordinary-public-path-lexical-first-20261120.md` already distinguish these limits. Existing actual lexical/current-vs-cwd/target-disappearance/old-reader evidence is reused, not rerun. No report of ordinary host-readiness success exists in this bounded source set, and no new implementation absence outside that set is inferred.

## Required architectural decisions before a bounded implementation

Recommendation for the architect: keep the current lexical branch unchanged and select the first host-checked slice explicitly, rather than add ambient `exists` in shared normalization.

1. **Relative base and identity:** choose the actual declaring consumer directory or selected project root explicitly. Prefer declaring-consumer origin for a future multi-consumer model, but this is a recommendation, not an accepted rule. A root-only first slice can avoid the ambiguity only by explicitly refusing member consumers; it must not imply the two bases are generally equivalent.
2. **Observation authority/perimeter:** specify whether absolute/outside-project paths are admitted, traversal/symlink/ancestor/nonregular handling, which host observation owner supplies no-follow capability, and whether only existence/type is observed. A public Path value alone grants neither content read nor materialization.
3. **Same-instance merge:** lexical originals currently agree by typed value. Decide whether equal spellings from distinct consumer bases must resolve to the same host target, conflict, or remain outside the first slice. Do not silently replace original spelling with canonical physical equality.
4. **Replay/update host law:** decide when unchanged install/reinstall/cold replay must revalidate existence, what happens when a previously observed target disappears or changes kind, and whether a moved checkout rebases relative input or requires new input. Keep pure show/history inspection distinct from planning readiness. Never treat a historical source pin as a target-host witness.
5. **Retention and compatibility:** decide whether the first slice reobserves on every planning operation without persisting host realization, or needs a typed owner-bound observation/realization. Existing epoch8 lexical data does not automatically authorize a new host-bound field or app-record copy; require actual old-client behavior if representation/meaning changes.
6. **TOCTOU and failures:** specify final pre-effect recheck, missing/changed target refusal and admitted guarantee (cooperative revalidation, not atomic immunity against a hostile process). No generic rollback claim for arbitrary external files follows.

After that contract, finite synthetic tests can distinguish consumer-root/project-root/cwd, missing/present/disappeared/rebound targets, explicit lexical preservation, fresh/cold/update behavior, source drift, supported prior clients and no-effects refusals under enforced isolation. This report does not run or pre-accept them.

## Checks and disposition

Inspected only the named repository contracts/code/reports. No parameter value was dereferenced, no HOME/auth/config path searched, no build/runtime/network/provider operation was needed. Source hashes are sealed in `ordinary-path-host-contract-candidate-r1.json` under `/data/git/vibevm-keyring-storage-20261120T0100Z/`; whitespace check passed. Architectural choice remains with the architect before product edits. No parent-state changes, USER-PROJECT execution, publication, REMAINDER or IDE work; NST remains active.
