# `flow:git-practices` — a repository's commit-and-push discipline, as a family

The **git-practices** family aggregates the distinct disciplines a repository applies to its
Git history. Each is its own installable `flow` package — adoptable alone — and this umbrella
names a tested set of them; requiring `flow:git-practices` pulls the whole family through the
dependency closure, each member contributing its own boot snippet.

They are deliberately **separate packages** because they are different things:

- **`conventional-commits`** — the message *format*: `type(scope): subject`, a why-not-what body,
  the allowed-type set, scope convention, worked examples, anti-patterns.
- **`atomic-commits`** — the *atomicity* discipline: one commit = one logical idea; when to split
  a mixed working tree, when to batch, why it matters more in a human-AI team.

A message can be valid Conventional Commits and non-atomic (`feat: add foo, bar, baz`), or atomic
without the format — so each is adoptable on its own, and the family is how a project takes the
whole posture at once. The family grows to include **human-authored attribution** and **commit
autonomy** as those members land.

Content-minimal by design (PROP-028): no boot snippet of its own — the members ship theirs.

## Install

```bash
vibe install flow:git-practices
```

## License

UPL-1.0 — see `LICENSE`.
