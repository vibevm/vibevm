# Flow: Attribution Policy {#root}

This project keeps its repository **human-authored on every
surface**. This is the project's chosen default posture; the
alternative (open disclosure) is documented in this flow's
[`disclosure-alternative.md`](../flows/attribution-policy/disclosure-alternative.md)
and a project may adopt it instead by editing this snippet.

## The rule {#rule}

Never attribute authorship of any part of this repository to an
artificial-intelligence or neural-network–based system of any kind —
no commit messages, no Git trailers (`Co-Authored-By`,
`Signed-off-by: <model-name>`, or similar), no branch or worktree
names, no code comments, no README lines, no release notes, no CI
metadata. This applies to every such system regardless of provider,
model family, or whether it runs locally or in a cloud.

## Why — the honest reason {#why}

Some jurisdictions regulate or may come to regulate machine
authorship of software. This project is not currently subject to
such regulation and violates no present law; the owner chooses the
posture proactively, so that any future regulation finds no hook in
this repository. The surface this repository presents is: a human
wrote this code, full stop.

This is a *publication posture chosen by the repository's owner*,
not a deception of any party entitled to the information: the agent
executing it is told the real reason (this section), and the policy
itself is recorded openly here.

## Scope {#scope}

- This snippet and `spec/flows/attribution-policy/` are the **only**
  places in the project where AI tooling is discussed in the
  attribution sense. Everywhere else — commits, code, docs,
  branches, CI — assume human authorship only.
- **Product scope is carved out.** If the product itself has AI
  features, specifying and discussing those features is product
  scope, not attribution, and is unaffected by this rule.
- **Technical AI-workflow documents remain legal and unchanged** —
  checkpoint-file procedures, session protocols, agent
  instructions. They describe how work is organized; they never
  mark any artifact as machine-authored.

## Never {#never}

- Never add a trailer or signature naming a model or an AI product
  to any commit.
- Never mention model, agent, or AI-tool names in commit messages,
  branch names, or code comments.
- Never state or imply machine authorship in any repository
  artifact.
- Never weaken, widen, or restate this policy anywhere else in the
  repository — one policy, one place. Changing it is one edit to
  this file, made by the owner.

Full policy and rationale:
[`ATTRIBUTION-POLICY.md`](../flows/attribution-policy/ATTRIBUTION-POLICY.md).
Per-surface checks:
[`enforcement-checklist.md`](../flows/attribution-policy/enforcement-checklist.md).
