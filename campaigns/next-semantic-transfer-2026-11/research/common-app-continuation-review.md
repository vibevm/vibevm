# Common app continuation — bounded source review

Observed main `8f84aefb9cf94d7e875c6d25eb26b90a1d22a319`. Read root's `common-app-vertical-decisions.md`, the named standing/spec files and the prior secret-resume report. This review inspects the current owners below; it adds no runtime reproduction, code/spec/Git changes, builds or workers. Recommendations incorporate root's later choice of a dedicated `vibe app resume <exact pending key>` verb.

## Verdict

The dedicated home-aware verb is cleaner than the earlier raw-argv formatter proposal. Once the complete original public request is durably owned by pending work, copying `--set`, profile/feature flags or a mutable configuration-input filename into a shell command is unnecessary. Resume can select the pending operation and accept only fresh ephemeral secret resupply. It also supplies the missing route to a project-home dependency that has parked before acquiring an installation index record.

The draft needs the following concrete continuation details. None requires another database, scheduler, generic transaction runtime or nested secret-input protocol.

## 1. Write parent pending input before descending into requirements

Observed: `operations/mod.rs::install_entered`, `operations/update.rs::update_locked` and `operations/modify.rs::modify` compute a plan, then call `deps::install_required`, and only afterwards enter apply/Run. `steps/run.rs::Run::open` currently creates and saves steps.json. A dependency can therefore park before the parent has persisted any original answers.

Required sequence: complete the selected root preflight; create its typed pending record in the existing work/steps2 owner; bind its actual operation/home/request context; then enter requirements. This is a planned/awaiting-dependency phase, **not a fake Done step or index publication**. Later Run::open must adopt that prepared state instead of its present Fresh clear-and-recreate behavior, which would erase the checkpoint. On resume, load this state before choosing an option, resolving new defaults or using the current index as the input authority.

A pending key must identify one operation generation, not just package@version or the newest run. Work directories are reused by package key and Fresh clears them. An old hosted task's resume command must refuse if the path now belongs to another pending operation. One existing-style operation/run ID in steps2 and the canonical command is enough; it is a stale-continuation discriminator, not an execution grant.

## 2. Minimal retained data belongs to the existing pending record

Use the same generated common input/evaluation types intended for index/plan, not handwritten JSON or replay argv:

| Retained data | Why it is needed |
|---|---|
| Pending generation, closed operation kind, exact package/selected option/platform/arch and actual app-home binding | Dispatch install/update/modify/import continuation to the correct existing owner; never derive the operation from current index shape. |
| Complete **public original input**, including absent/empty/exact/additive/dormant/profile/delta outcome | Public answers may originate from a prompt, stdin or a file that later disappears. Re-reading those sources is a new request. |
| Nonsecret evaluated result/origins, canonical path realizations, review basis and reliance | Recheck current applicability without silently renewing changed default/profile consent or re-resolving a relative path under another CWD. |
| Value-free secret provided/resupply IDs, scoped to this selected installer, with origin sufficient to distinguish fresh supplied input from a declaration default | A previously supplied optional secret cannot be satisfied by omission or fallback. Check **fresh explicit/prompt input presence**, not merely whether final parameter resolution found some value. |
| Actual dependency continuation references, and predecessor identity/operation progress needed by maintenance | Parent must find its exact child work and must not repeat already-completed removal or choose a different predecessor. |
| Bound execution/agent-envelope context described below | Configuration equivalence alone does not make an old delegated answer applicable to a new task. |

Secret values and their reusable digests do not belong in any record, fingerprint or error. Resupplying a secret authorizes a new ephemeral binding; without retaining the old value/hash, the program can check presence and declaration/origin, **not equality with the previous secret**. Say that explicitly instead of claiming exact secret replay. Clearing/changing public intent or accepting a changed basis is fresh reconfiguration, not permission to keep old Done steps under `resume`.

Version selection must follow the data required, not merely rich-versus-legacy. A new legacy operation with a supplied optional secret or otherwise necessary retained continuation input also needs steps2. Existing protocol1 can remain readable with effective-only/weaker provenance, but must not be described as carrying original intent or a secret-presence proof. `steps/state.rs::{load,to_wire}` is currently strictly protocol1; dispatch by version before decoding the closed schema.

## 3. Bind execution continuation separately from configuration meaning

Observed: `steps/resume.rs::start_point` compares only package/option/features; it neither checks public parameters nor step definitions. `steps/agent.rs::deliver` currently expands the newly planned prompt, writes the envelope, then checks whether an old delegated result.json exists. A configuration fingerprint would not detect a changed prompt or shell/step declaration.

Bind the selected step IDs/order/kinds/phases/run decisions and relevant public execution context to the pending operation. For a delegated step, preserve/compare the **actual prepared prompt literal and the prompt/context bytes that the agent received** in the existing envelope/step owner. Check old result applicability before rewriting its prompt/context or accepting result.json. A changed definition or task context must leave the old answer/envelope intact and return a named mismatch; a fresh reconfiguration/restart may deliberately replace it.

This does not establish whole-source authentication. ResolvedPackage currently contains root/coordinate/application/options, not a guaranteed package-tree content identity. The accepted template work captures a file only within one plan. Recipe bytes under the same path and arbitrary mutable payloads are not authenticated by a configuration digest or a step-path string. Retain/check bytes actually used to bind the old delegated task; do not advertise broader source replay guarantees that acquisition does not provide.

## 4. Resume linked children before treating them as satisfied dependencies

Observed: `operations/deps.rs::install_required` constructs child InstallRequest with Default/Fresh. Passing the parent's `--resume` unconditionally would also be wrong: a newly encountered child may have no pending work. Instead use the parent's exact pending child references to choose Resume for the linked unfinished child and Fresh only for a genuinely new child.

More importantly, `resolve_requirement` first accepts an indexed child. Authored finish-step parking can already commit an index record: `Run::phase(Finish)` returns the parked failure for reporting after `apply` and install/update/modify commit. Thus **indexed does not imply completed**. Consult the linked pending state before installed-candidate short-circuiting. Project `install_pending` similarly filters out any indexed row; pending finish work must be considered before that filter.

Root's preferred secret flow works with the new verb: give explicit child pending key/home/option guidance, let the caller resupply that child's secret through its own continuation, then resume the linked parent. A root public parameter and a child secret with the same ID are different requests. Do not union their names or pass one root `--set` into the child. Parent continuation must validate that the settled child still fulfills its actual recorded requirement/source route; no blind fallback to Fresh or same-name user-home installation.

## 5. Keep actual source routes and maintenance phases

`vibe_app::from_manifest` already verifies exact coordinate against manifest; `from_local_registry` admits a root below the selected local registry; `from_global` acquires a cached tree. CLI `global::Packages` carries registry/embedded/offline policy, while project requirements resolve strictly from the selected project's lock slots via `from_slot`.

Pending continuation should retain the actual acquired source locator and exact coordinate/option plus its existing source route kind: local registry root, global acquired store locator or project lock/slot. Reuse those owners to re-admit the source and compare configuration/execution basis. Do not re-resolve an unversioned package to today's newest version, silently fall through to a different registry/home, or serialize credential values. If the retained source is unavailable, recover through the recorded route and recheck; otherwise report source unavailable. A path/coordinate is not whole-tree authentication. Project resume must remain anchored to the selected project home/lock context and must not call project install just to manufacture a root dependency declaration.

Maintenance needs minimal phase data beyond individual new installer steps. Modify performs recorded old-choice removal **before** apply; a prepare-step park after removal must not repeat it on resume. Preserve predecessor identity and whether removal has not started/completed/has an uncertain outcome. A crash inside arbitrary removal commands cannot be certified complete; that needs truthful recovery, not automatic double execution. Update must retain its actual prior/supersede target: re-running `previous_installation` after a finish park may select the newly indexed version and change rollback bookkeeping. Preserve existing placement/finish resume behavior instead of reinstalling or deleting a placed payload.

## 6. Exact grammar and formatter consequence

Recommended new syntax remains root's `vibe app resume <exact pending key>` under the existing app home selector (`-g` or explicit `--path PROJECT`), with ephemeral resupply through agreed existing `--set`/hidden prompt inputs and normal operational flags. It dispatches the recorded operation kind. The original review did not propose `modify --resume` or `import --resume` aliases because those main grammars did not support them. The import part of that recommendation is superseded by the later actual owner-collision reproduction: an independent cohort now requires the deliberately added `app import --resume ID` route. Application resume remains exact-key/generation; no command is generated against an unsupported old grammar. See common-app-vertical-decisions.md for the current target and ownership contract.

AgentHooks should carry a typed pending target/root reference or a formatter over that value-free target, rather than original argv. Render a canonical command for the pending root (and separate child guidance when needed). Do not retain/replay a complete-input filename/stdin marker, feature/profile/public-parameter flags, or expected_definition as a new submission: the record is the input authority. Formatter failure cannot fall back to the old literal argv. Root/cohort binding still matters for **capturing** input, but is no longer a string-redaction problem.

If shared batch `--set` reaches multiple selected root options, classify its actual recipients before persisting it as public. A future root may mark the same shared assignment secret. The dedicated command alone does not cure that capture ambiguity. Either complete the actual cohort preflight first with an explicit rule for mixed classification, or refuse an incompletely classified batch before effects and direct the caller to explicit per-root inputs. Do not invent a public value by examining only the first root or a same-named dependency.

## 7. Import needs a finite-loop continuation, not just a row installation

Observed: `commands/app/portable.rs::import` treats a Parked error as a refused row, continues later rows, then restores arrangement; `import_action` can skip an already indexed same-choice row even when a finish step is still parked. `install_row` currently uses Fresh.

For complete import continuation, stop/propagate delegated at the first parked row and retain the existing loop's **classified public cohort, current row/completed rows, hidden/held/skip policy and pending arrangement action** in its pending owner. Resume the pending row/dependency before same-choice skip logic, then continue remaining rows and restore arrangement only at the defined completion point. Keep the cohort owner alive until that finite import finishes. This is a continuation of the existing importer, not a general workflow engine. Merely resuming the current app row cannot be advertised as resuming the whole import. Capturing a raw import file or hashing unclassified answer bytes is not a substitute for classified public input.

## Discriminating public cases (proposed, not executed here)

1. Parent chooses nondefault/dormant common input via callback or stdin; child parks before parent Run::open. New-process root resume retains all answers with the original source gone and no new default consent.
2. Supplied optional secret: missing fresh input refuses even if a current default exists; fresh resupply continues; no raw/canonical value or digest appears in index/steps/plan/envelope/command/errors. Include a legacy operation that therefore requires protocol2.
3. Root public ID and child secret same ID: child-target resume takes only child resupply, parent input unchanged. Repeat in project home before child index exists; no user-home install or new workspace requires row.
4. Child parks in finish after its index is written. Parent must not treat it complete; answer+child resume finishes before parent proceeds.
5. Modify parks after old removal: resume never runs removal twice. Update finish-park keeps its original predecessor/rollback metadata. Simulate uncertain removal separately and assert explicit recovery, not invented success.
6. Change prompt/step context after a delegated answer, leaving configuration identical: refuse before envelope rewrite/result consumption. Changed defaults or supplied public deltas likewise do not silently continue old Done steps.
7. Fresh replacement at the same package@version creates another pending generation; an old task command cannot resume it. Wrong -g/project home and changed source route refuse without cross-home mutation.
8. Import with a later row and arrangement parks at the first row: no later mutation before delegation; dedicated resume finishes the correct pending row and remaining finite loop once, including already-indexed finish parking.
9. `app resume` generated command parses for each recorded operation, contains no original input/secret args and works with input-file removal. Mixed-secrecy shared batch input is handled by the chosen cohort rule before any serialization/effect.

The existing draft's original-intent/default-basis/path-origin decisions are sound. The items above make their durable ordering and continuation semantics concrete; source inspection alone is not acceptance of the new behavior.

## Concrete items still for root/writer decision

- Exact wire field names and pending-key encoding are not frozen by this review. The accepted requirement is one home-scoped exact package pending key plus operation-generation ID, with versioned state and no lookup by “latest”.
- The existing operations must choose their narrow persisted phase transitions around dependency descent, modify removal and index/finish publication. The normal parked path must resume; an uncertain external-removal outcome must remain explicitly uncertain.
- The whole-import cohort/cursor owner and mixed-classification shared batch-input rule remain to be implemented/reviewed. Row-only continuation must not be reported as whole-import completion.
- Legacy protocol1 resume behavior needs an explicit compatibility label and refusal where stronger provenance is required; new operations requiring secret-presence/input metadata use2 even if their installer syntax is legacy.
- Existing acquired-source re-admission is the supported freshness boundary. No serializer-field choice can supply missing whole-tree/recipe authentication; the chosen execution/prompt binding and its limit must remain visible in acceptance.

These are bounded implementation/review decisions inside the active selected-installer parent, not reasons to replace it with parser/index-only completion.
