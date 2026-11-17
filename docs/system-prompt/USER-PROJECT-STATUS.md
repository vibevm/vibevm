# USER-PROJECT: practical delivery status

Verified on 2026-11-17. This reports runnable behavior against PROP-074, not a
rewrite of its requirements or whole-feature/release acceptance. The previously
accepted bounded slice remains the base. See the [quickstart](README.md#personal-specification-boot)
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
- **Legacy/remoter modes:** remap offer probing and installation still resolve
  independently; global remap preview explicitly refuses without adoption.
  Broader solver/mode, remote preflight cache/publication and in-place acquisition
  parity have not been established by this local journey.
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

Before release, the owner must decide that scope against the still broader
PROP-074 wording, have the corresponding specification/status reconciled by its
owner, and authorize fixation/integration and release separately. Review the
scoped changes without staging unrelated dirty bytes. A release to additional
operating systems or a claim of graphical/provider integration requires the
corresponding targeted smoke on that consumer, rather than another universal
prerequisite ladder. Retain the existing baseline lint finding honestly under
ordinary release policy.

If unrestricted automatic routing, general AI-native authority, or autonomous
memory writes become release promises, select the relevant user scenario as a
separate deliberate task. They are not implied by this delivery. The removed
retained/native/history-policing framework is not a follow-up or release
prerequisite and must not be revived.

This status does not authorize commits, push, publication, provider calls or
changes to credentials, harness configuration, toolchains or peer checkouts.
