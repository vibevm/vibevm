---
name: draft-eula
description: Draft or review a project's license posture — the placeholder EULA with relicense intent, the permissive-only dependency check, and the third-party carve-out. Use when setting up a new project's LICENSE.md or auditing an existing one. Guidance, not legal advice.
---

<status stage="impl" state="done"/>

# Draft or review a licence posture {#root}

@fact:DRAFTING-OR-REVIEWING-A-LICENSING-POSTURE You are drafting or reviewing a project's licensing posture from the
`flow:licensing` protocol. @status:impl/done

@fact:PRODUCE-A-DRAFT-THE-DECISION-IS-THE-OWNERS Produce a draft for the owner; the licence
choice and any relicensing are the owner's decision, never yours. @status:impl/done

@fact:STATE-PLAINLY-GUIDANCE-NOT-LEGAL-ADVICE State plainly that this is guidance, not legal advice. @status:impl/done

## Procedure {#procedure}

1. @fact:DETERMINE-THE-POSTURE **Determine the posture.** Ask (or infer from context) which the
   owner wants: permissive open, proprietary/source-available, or the
   placeholder with relicense intent. If unclear, ask — do not
   assume. @status:impl/done
2. @fact:DRAFT-THE-LICENSE-FILE **Draft the `LICENSE.md`.** For the placeholder, fill the skeleton
   in `spec/flows/licensing/eula-template.md`: product name,
   copyright, the interim grant, the relicense-intent clause (name the
   target licence; keep it only if opening is genuinely intended), and
   the third-party carve-out. For a permissive posture, use the target
   licence's official text verbatim and set the manifest field to its
   SPDX id. @status:impl/done
3. @fact:CHECK-DEPENDENCIES **Check dependencies.** Run the ecosystem's licence listing over
   the FULL resolved graph (not just direct deps). Classify each
   against the allow/deny table in
   `spec/flows/licensing/dependency-licenses.md`. Flag anything
   copyleft, unclear, or missing as an owner decision. @status:impl/done
4. @fact:SYNC-THE-STATEMENTS **Sync the statements.** Ensure the `LICENSE.md`, every manifest
   `license` field, and any README badge agree. A disagreement is a
   finding. @status:impl/done
5. @fact:POINT-THE-CARVE-OUT-AT-THE-GENERATED-LIST **Point the carve-out at the generated list**, and name any
   study-only material for removal before redistribution. @status:impl/done

## Output {#output}

@fact:present-as-a-draft-lead Present as a draft: @status:impl/done

- @fact:OUTPUT-THE-PROPOSED-LICENSE-FILE the proposed `LICENSE.md`, @status:impl/done
- @fact:OUTPUT-THE-MANIFEST-FIELD-VALUE the manifest field
  value, @status:impl/done
- @fact:OUTPUT-THE-DEPENDENCY-CHECK-RESULT the dependency-check result (with any violations called out), @status:impl/done
- @fact:OUTPUT-THE-SYNC-STATUS and the sync status. @status:impl/done

@fact:DO-NOT-APPLY-WITHOUT-THE-OWNERS-EXPLICIT-APPROVAL Do not apply the licence choice or any
relicensing without the owner's explicit approval — both are
irreversible-threshold decisions. @status:impl/done
