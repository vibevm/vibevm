# `flow:atomic-commits` — one commit, one idea

A vibevm `flow` package that installs the **atomic commits** Git
discipline into a project. One commit carries exactly one logical
change, and the commit message explains *why* in
[Conventional Commits](https://www.conventionalcommits.org/) format.

In a pure-human team, atomic commits are a quality-of-life feature
(easier review, cleaner bisects, viable cherry-picks). In a human-AI
team they are load-bearing: the human's primary verification mechanism
is reading the diff, and a commit that mixes three concerns across
eight files is not verifiable in one pass.

This package ships three pieces of content plus a boot snippet:

- `spec/flows/atomic-commits/ATOMIC-COMMITS-PROTOCOL.md` — full
  protocol: what an atomic commit is, why it matters more in a
  human-AI team than elsewhere, when to split, when to batch, and the
  rule that pushed history is frozen.
- `spec/flows/atomic-commits/conventional-commits.md` — message
  format: header shape, allowed types, scope convention, body
  structure, worked examples, anti-patterns.
- `spec/flows/atomic-commits/splitting-large-changes.md` — mechanical
  procedure for turning a messy working tree into a sequence of
  atomic commits using `git add -p`, including a prompt for
  delegating the split to the agent.
- `spec/boot/30-flow-atomic-commits.md` — boot snippet loaded at
  session start, pointing the agent at the protocol and the never-do
  list.

## Install

```bash
vibe install flow:atomic-commits
```

## Uninstall

```bash
vibe uninstall flow:atomic-commits
```

Uninstalling removes every file the package wrote, including the boot
snippet. User-owned files (`00-core.md`, `90-user.md`, `WAL.md`) are
never touched.

## Composition

- Works with `flow:wal` (`10-…`) and `flow:sync-from-code` (`20-…`):
  numeric boot-snippet prefixes are distinct by design.
- `flow:sync-from-code`'s final step is a `docs(spec)` commit; the
  format of that commit message is pinned by this flow.
- End-of-session WAL rewrite (from `flow:wal`) ends in a commit;
  atomic-commits is how that commit is shaped.

## Philosophical background

The rule is extracted from *AI-native development*, chapter 2
(*"Shared state: файлы как IPC"*, subsection "Атомарность"), together
with the Conventional Commits specification. Short version: humans
hate splitting messy trees, AI is happy to — delegate the split,
verify the plan, and use the commit log as the project's decision
archive.

## License

EULA. See the surrounding registry for distribution terms.
