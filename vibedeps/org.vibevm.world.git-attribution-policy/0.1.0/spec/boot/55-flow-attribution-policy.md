# Flow: Attribution Policy {#root}

<status stage="impl" state="done"/>

@fact:THE-REPOSITORY-IS-HUMAN-AUTHORED-ON-EVERY-SURFACE This project keeps its repository **human-authored on every
surface**. @status:impl/done

@fact:THE-ALTERNATIVE-IS-ADOPTED-BY-EDITING-THIS-SNIPPET This is the project's chosen default posture; the
alternative (open disclosure) is documented in this flow's
@spec://org.vibevm.world/git-attribution-policy/flows/attribution-policy/disclosure-alternative#root
and a project may adopt it instead by editing this snippet. @status:impl/done

## The rule {#rule}

@fact:NEVER-ATTRIBUTE-AUTHORSHIP-TO-ANY-SUCH-SYSTEM Never attribute authorship of any part of this repository to an
artificial-intelligence or neural-network–based system of any kind —
no commit messages, no Git trailers (`Co-Authored-By`,
`Signed-off-by: <model-name>`, or similar), no branch or worktree
names, no code comments, no README lines, no release notes, no CI
metadata. @status:impl/done

@fact:THE-RULE-APPLIES-REGARDLESS-OF-PROVIDER-OR-LOCATION This applies to every such system regardless of provider,
model family, or whether it runs locally or in a cloud. @status:impl/done

## Why — the honest reason {#why}

@fact:some-jurisdictions-regulate-machine-authorship Some jurisdictions regulate or may come to regulate machine
authorship of software. @status:spec/done

@fact:the-owner-removes-the-hook-proactively This project is not currently subject to
such regulation and violates no present law; the owner chooses the
posture proactively, so that any future regulation finds no hook in
this repository. @status:spec/done

@fact:THE-SURFACE-PRESENTED-IS-A-HUMAN-WROTE-THIS-CODE The surface this repository presents is: a human
wrote this code, full stop. @status:impl/done

@fact:a-publication-posture-not-a-deception This is a *publication posture chosen by the repository's owner*,
not a deception of any party entitled to the information: the agent
executing it is told the real reason (this section), and the policy
itself is recorded openly here. @status:spec/done

## Scope {#scope}

- @fact:SCOPE-THE-ONLY-PLACES-THE-TOPIC-IS-DISCUSSED This snippet and `spec/flows/attribution-policy/` are the **only**
  places in the project where AI tooling is discussed in the
  attribution sense. Everywhere else — commits, code, docs,
  branches, CI — assume human authorship only. @status:impl/done
- @fact:SCOPE-PRODUCT-IS-CARVED-OUT **Product scope is carved out.** If the product itself has AI
  features, specifying and discussing those features is product
  scope, not attribution, and is unaffected by this rule. @status:impl/done
- @fact:SCOPE-WORKFLOW-DOCUMENTS-REMAIN-LEGAL **Technical AI-workflow documents remain legal and unchanged** —
  checkpoint-file procedures, session protocols, agent
  instructions. They describe how work is organized; they never
  mark any artifact as machine-authored. @status:impl/done

## Never {#never}

- @fact:NEVER-ADD-A-TRAILER-NAMING-A-MODEL Never add a trailer or signature naming a model or an AI product
  to any commit. @status:impl/done
- @fact:NEVER-MENTION-TOOL-NAMES-IN-COMMITS-BRANCHES-OR-COMMENTS Never mention model, agent, or AI-tool names in commit messages,
  branch names, or code comments. @status:impl/done
- @fact:NEVER-STATE-OR-IMPLY-MACHINE-AUTHORSHIP Never state or imply machine authorship in any repository
  artifact. @status:impl/done
- @fact:NEVER-RESTATE-THIS-POLICY-ANYWHERE-ELSE Never weaken, widen, or restate this policy anywhere else in the
  repository — one policy, one place. Changing it is one edit to
  this file, made by the owner. @status:impl/done

@fact:sibling-document-pointers Full policy and rationale:
@spec://org.vibevm.world/git-attribution-policy/flows/attribution-policy/ATTRIBUTION-POLICY#root.
Per-surface checks:
@spec://org.vibevm.world/git-attribution-policy/flows/attribution-policy/enforcement-checklist#root. @status:impl/done
