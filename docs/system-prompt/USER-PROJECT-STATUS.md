# USER-PROJECT: practical delivery status

Verified on 2026-11-17. This reports runnable behavior against PROP-074 and its
owner-approved CURRENT-SCOPE, MEMORY-DELIVERY, TRANSPORT-DELIVERY and
CLOSURE-ACCOUNTING refinements. It is not whole-feature/release acceptance.
The previously accepted bounded slice remains the base. See the [quickstart](README.md#personal-specification-boot)
and [offline walkthrough](user-project-onboarding.py).

## Finished user journeys

| Original obligation | Delivered scenario and evidence boundary |
| --- | --- |
| Personal root and package lifecycle | Explicit `user init/status`, then `install/update/uninstall -g --user-project`, `list -g`, `check -g`. One isolated walkthrough executes the entire lifecycle through the next launch. Ordinary manifest, lock, registry and compilation owners remain authoritative; MCP stays separate. |
| Late personal boot, shared cache stability | Actual argv-recording launcher receives complete project continuation followed by personal content. An indexed authored edit is visible at next launch without rebuilding the project. Shared SYSTEM/AGENT bytes remain identical through personal update/uninstall. Existing focused tests also cover static content, conditions, missing/unsafe optional layers and project-only fallback. |
| User memory | Authorized authored personal record is indexed by ordinary installation and loaded next launch; live edit survives package removal. Shared guidance distinguishes personal facts from project facts, excludes credentials and honors durable opt-out. Existing memory lifecycle tests cover opt-out. This is not an autonomous model-writing demonstration. |
| Per-project overrides | Independent root, package installation, authored preference, explicit alias binding to another checkout, enable/disable and next-launch effects. Override comes after personal; disabling preserves authored files. Existing tests cover Git worktree identity and invalid bindings. |
| Layered composition | `prompt compose` produces private local text with the exact shared project prefix as leading bytes and ordered personal/override content. No provider call. Existing tests cover no-layer identity and disabled-layer exclusion. |
| Layered tree | Plain and JSON show separate project/user/override descriptors. Existing console and hosted-PTY evidence covers view-only hide/show versus persisted override enable/disable, including next capture. No graphical Electron smoke. Attribution is additive; prose is not analyzed to invent a rule-replacement graph. |
| Classic discovery | Generated no-template entrypoints carry late file-reading instructions and the same memory/override guidance. Existing tests verify generated text and no implicit initialization. Model compliance with those instructions is not measured. |
| Privacy and AI-native boundary | Existing publisher tests exclude reserved boot/local before descent. Canonical specmap guard, synchronized shipping CLI/MCP consumers and real local consumer probes refuse configured private-layer roots; legitimate external dependencies remain supported. This is not a universal declared-dependency authority implementation or secret/history scanner. |

The new walkthrough is one connected scenario, not a fresh per-helper matrix:
27 successful CLI invocations and 24 behavioral checks on Linux, with a synthetic
local registry and hermetic argv recorder. It records binary and scenario hashes,
all command outcomes, and explicit limits in its scratch `RESULTS.json`. It
initializes an isolated home/settings directory rather than reading user secrets.
Its fixture launcher configuration exists only there. The first development run
failed an incorrect test assumption about INDEX order; the assertion was corrected
to require complete project-before-personal ordering. No product defect or runtime
code change was needed to complete the journey.

## Honest bounded limits

- **Automatic source-policy parity:** automatic global personal installation
  refuses an existing declared/locked personal context. The recommended explicit
  `--user-project` route uses ordinary personal policy and supports the finished
  lifecycle. Inferred global `--stop-file` requires that same explicit selector.
  These remain contract gaps, not silently waived requirements, but are not a
  reason to redesign routing merely to shorten the documented command.
- **Legacy/remoter modes:** global remap installation now carries its original
  resolver and selected source candidates from kind preflight into acquisition,
  then uses the ordinary offer-adoption owner. Focused regressions cover a newer
  version appearing after selection and refusal when selected metadata changes.
  Global remap preview explicitly refuses without adoption. Broader solver/mode,
  remote preflight cache/publication and in-place acquisition parity have not
  been established by this local journey.
- **Memory automation:** authorized explicit records and next-boot loading are
  demonstrated. Autonomous model fact-writing and absent-root first-write behavior
  are not. No harness-memory migration or automatic synchronization is provided.
- **AI-native authority:** current private roots and reserved local output are
  guarded. General direct-unresolved/dependency-authority semantics beyond that
  bounded guard are not accepted as complete.
- **Consumers:** launch carrier transport is demonstrated, not model compliance.
  Compose is a caller-consumable file; there is no established product session API
  sender to wire. Installer lifecycle chat must not receive personal boot merely
  to create an API checkbox. Graphical Electron smoke and other-OS runtime smoke
  are unrun.
- **Verification:** prior applicable 153 tests, affected orchestrator 171 plus
  routing/personal/root tests, strict host lint, zero-new full conform and codegen
  idempotence remain the accepted base evidence, not an exhaustive workspace run.
  Canonical AI-native strict lint still has the recorded pre-existing
  `explain.rs:99` redundant-borrow failure; its scoped allowance is not strict-clean.

## What is actually needed for a practical release

The recommended release scope is the documented **explicit personal workflow**,
not unrestricted automatic routing or autonomous memory management. That workflow
has a runnable local end-to-end path; no additional runtime blocker was found.

The owner-approved scope is now reconciled in PROP-074 and the finite
UP-FINISH-01 brief; scoped product/onboarding commits and local integration are
authorized and performed. External release/push remain separate owner actions.
The final requirement dispositions below preserve the broader original promises
rather than presenting every one as implemented. Review scoped changes without
staging unrelated dirty bytes. A release to additional
operating systems or a claim of graphical/provider integration requires the
corresponding targeted smoke on that consumer, rather than another universal
prerequisite ladder. Retain the existing baseline lint finding honestly under
ordinary release policy.

Broader original guarantees, including unrestricted automatic routing and general
AI-native authority, need separately completed scenarios before they can be
claimed. The current approved release boundary is narrower. Autonomous memory
writing is not required by that boundary and is not demonstrated here. The removed
retained/native/history-policing framework is not a follow-up or release
prerequisite and must not be revived.

This status does not authorize commits, push, publication, provider calls or
changes to credentials, harness configuration, toolchains or peer checkouts.

## Final original-requirement disposition

| Requirement family | Disposition | Evidence / exact residual |
| --- | --- | --- |
| ROOT, INIT, STATUS, root grammar/admission/publication/report | Demonstrated | Root29, core15, workspace51 tests; walkthrough init/status. No implicit boot/read init. |
| Explicit INSTALL, installed lifecycle, EMPTY-AND-ALL, list/check | Demonstrated | Personal lifecycle6/read4; walkthrough install/list/check/no-arg regeneration/update/uninstall. Own manifest/lock/source policy. |
| Automatic INSTALL-ROUTING and inferred stop | Explicit unsupported convenience at its documented boundary | Existing personal source context refuses with explicit-selector remedy; inferred stop does likewise. No supported explicit-flow blocker found. |
| Application/MCP compatibility, one destination before mutation | Demonstrated within named test subjects | Existing legacy routing gates and global mixed-destination/preview regressions. Not every remote/solver/materialization mode was measured. |
| Remap source selection through global installation | Demonstrated bounded repair | Same live resolver/candidate from preflight through acquire; catalog-growth and selected-manifest-drift regressions, plus ordinary remap/preview compatibility tests. No new source authority. |
| Late lane, outside shared prefix, optional failure, protected precedence | Demonstrated delivery contract and transport | Boot3 and actual recorder journey; shared bytes unchanged, next-launch edits/toggles. Model obedience is not measured. |
| Memory default/classification/opt-out | Demonstrated guidance and authored-record delivery | Shared instruction constant; boot3 opt-out/index/live-edit test and connected authored-memory journey. Read4 proves absent-root initialization remedy and invalid-root refusal without mutation. No memory-write command is invented. |
| Absent/unavailable first memory-write destination | Delivered instruction/refusal boundary; autonomous behavior excluded | Guidance requires explicit user init/action and forbids silent harness fallback; ordinary reads/lifecycle refuse safely. There is no autonomous writer to exercise; no claim of observed model behavior. |
| Override purpose/shape/location/controls/additive/separate resolution | Demonstrated | Override5, independent package roots and locks, worktree identity test, actual next-launch toggles and explicit new-checkout bind. |
| LAYERED-PREFIX and LOCATION | Demonstrated | Compose1 plus walkthrough exact leading bytes, protected local publication/ignore and separate launch carrier; no-layer identity tested. |
| LAYERS-IN-TREE additive attribution and controls | Demonstrated | Layers3, frozen legacy projection, console/hosted-PTY hide versus enable controls. |
| Original explicit supplement/replacement visualization and general cross-layer resolution | Unimplemented original promise, not a supported explicit-flow blocker | Current view carries additive layer-qualified identity, not explicit replacement relations or merged cross-layer locks. Independent compilation is deliberate; no prose/NLP conflict graph is claimed. |
| OVERRIDE-NO-AI-NATIVE-ANCHORS | Demonstrated bounded guard; broader original authority promise remains unimplemented | Canonical isolation tests and12 shipping CLI/MCP probes reject configured current private roots and omit local units from index/coverage. General direct-unresolved or every undeclared external reference is not proven; no universal dependency-authority claim. |
| Privacy responsibility and filesystem-copy export | Demonstrated product boundary | Existing payload/staging exclusion tests, launch shared-byte isolation, local ignore checks. Deliberate Git publication/history policing is owner-excluded scope, not a missing scanner. |
| Classic, launch, capture, compose | Demonstrated respective reading/transport/artifact boundaries | Generated classic router tests, actual recorder, capture tests, compose artifact. Product session API sender does not exist; creating one or modifying installer authority is excluded. |
| Platforms and actual consumers | Validation unrun where named | Linux hermetic CLI/PTY measured. Other-OS runtime, graphical Electron and provider/model-compliance smoke unrun; not silently disabled product modes. |
| Retained/native transaction framework, migration and sync automation | Owner-excluded scope | Rejected framework removed; no revival. Autonomous memory extraction/migration/sync and NLP preference parsing are not prerequisites of current delivery. |

No concrete supported explicit-workflow data-loss, privacy or source-integrity
blocker remains identified by this finite assessment. The documented workflow is
ready within its stated boundaries. Original promises explicitly marked
unimplemented above are not thereby accepted, erased or reclassified as tests.
