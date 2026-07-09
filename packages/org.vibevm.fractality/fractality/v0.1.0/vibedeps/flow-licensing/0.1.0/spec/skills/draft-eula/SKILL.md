---
name: draft-eula
description: Draft or review a project's license posture — the placeholder EULA with relicense intent, the permissive-only dependency check, and the third-party carve-out. Use when setting up a new project's LICENSE.md or auditing an existing one. Guidance, not legal advice.
---

# Draft or review a licence posture {#root}

You are drafting or reviewing a project's licensing posture from the
`flow:licensing` protocol. Produce a draft for the owner; the licence
choice and any relicensing are the owner's decision, never yours.
State plainly that this is guidance, not legal advice.

## Procedure {#procedure}

1. **Determine the posture.** Ask (or infer from context) which the
   owner wants: permissive open, proprietary/source-available, or the
   placeholder with relicense intent. If unclear, ask — do not
   assume.
2. **Draft the `LICENSE.md`.** For the placeholder, fill the skeleton
   in `spec/flows/licensing/eula-template.md`: product name,
   copyright, the interim grant, the relicense-intent clause (name the
   target licence; keep it only if opening is genuinely intended), and
   the third-party carve-out. For a permissive posture, use the target
   licence's official text verbatim and set the manifest field to its
   SPDX id.
3. **Check dependencies.** Run the ecosystem's licence listing over
   the FULL resolved graph (not just direct deps). Classify each
   against the allow/deny table in
   `spec/flows/licensing/dependency-licenses.md`. Flag anything
   copyleft, unclear, or missing as an owner decision.
4. **Sync the statements.** Ensure the `LICENSE.md`, every manifest
   `license` field, and any README badge agree. A disagreement is a
   finding.
5. **Point the carve-out at the generated list**, and name any
   study-only material for removal before redistribution.

## Output {#output}

Present as a draft: the proposed `LICENSE.md`, the manifest field
value, the dependency-check result (with any violations called out),
and the sync status. Do not apply the licence choice or any
relicensing without the owner's explicit approval — both are
irreversible-threshold decisions.
