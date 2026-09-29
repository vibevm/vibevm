# `flow:git-atomic-commits` — one commit, one idea {#root}

<status stage="doc" state="done" audience="user"/>

@fact:PACKAGE-INSTALLS-THE-ATOMIC-COMMITS-DISCIPLINE A vibevm `flow` package that installs the **atomic commits** Git
discipline into a project. @status:impl/done

@fact:ONE-COMMIT-ONE-LOGICAL-CHANGE-EXPLAINED-BY-ITS-MESSAGE One commit carries exactly one logical
change, and the commit message explains *why* in
[Conventional Commits](https://www.conventionalcommits.org/) format. @status:impl/done

@fact:in-a-pure-human-team-atomic-commits-are-quality-of-life In a pure-human team, atomic commits are a quality-of-life feature
(easier review, cleaner bisects, viable cherry-picks). @status:spec/done

@fact:in-a-human-ai-team-they-are-load-bearing In a human-AI
team they are load-bearing: the human's primary verification mechanism
is reading the diff, and a commit that mixes three concerns across
eight files is not verifiable in one pass. @status:spec/done

@fact:package-contents-lead This package ships the **atomicity** discipline (the message **format** is the separate
`flow:git-conventional-commits` package): @status:impl/done

- @fact:CONTENT-THE-FULL-PROTOCOL `spec/flows/atomic-commits/ATOMIC-COMMITS-PROTOCOL.xml` — full
  protocol: what an atomic commit is, why it matters more in a
  human-AI team than elsewhere, when to split, when to batch, and the
  rule that pushed history is frozen. @status:impl/done
- @fact:CONTENT-THE-SPLITTING-PROCEDURE `spec/flows/atomic-commits/splitting-large-changes.xml` — mechanical
  procedure for turning a messy working tree into a sequence of
  atomic commits using `git add -p`, including a prompt for
  delegating the split to the agent. @status:impl/done
- @fact:CONTENT-THE-BOOT-SNIPPET `spec/boot/30-flow-atomic-commits.xml` — boot snippet loaded at
  session start, pointing the agent at the protocol and the never-do
  list. @status:impl/done

## Install {#install}

```bash
vibe install flow:git-atomic-commits
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:git-atomic-commits
```

@fact:UNINSTALL-REMOVES-EVERY-FILE-THE-PACKAGE-WROTE Uninstalling removes every file the package wrote, including the boot
snippet. @status:impl/done

@fact:USER-OWNED-FILES-ARE-NEVER-TOUCHED User-owned files (`00-core.xml`, `90-user.xml`, `WAL.md`) are
never touched. @status:impl/done

## Composition {#composition}

- @fact:COMPOSES-WAL-AND-SYNC-FROM-CODE-BY-DISTINCT-PREFIXES Works with `flow:wal` (`10-…`) and `flow:sync-from-code` (`20-…`):
  numeric boot-snippet prefixes are distinct by design. @status:impl/done
- @fact:COMPOSES-SYNC-FROM-CODE-FOR-THE-COMMIT-MESSAGE `flow:sync-from-code`'s final step is a `docs(spec)` commit; this
  flow is why the sync lands as its own commit and not folded into the
  code change. The *format* of that message is pinned by the sibling
  `flow:git-conventional-commits`, not here. @status:impl/done
- @fact:COMPOSES-WAL-FOR-THE-SESSION-END-COMMIT End-of-session WAL rewrite (from `flow:wal`) ends in a commit;
  git-atomic-commits is how that commit is shaped. @status:impl/done

## Philosophical background {#background}

@fact:extracted-from-the-books-second-chapter The rule is extracted from *AI-native development*, chapter 2
(*"Shared state: файлы как IPC"*, subsection "Атомарность"), together
with the Conventional Commits specification. @status:spec/done

@fact:short-version-delegate-the-split-and-keep-the-log-as-an-archive Short version: humans
hate splitting messy trees, AI is happy to — delegate the split,
verify the plan, and use the commit log as the project's decision
archive. @status:spec/done

## License {#license}

@fact:license-line UPL-1.0 — The Universal Permissive License, Version 1.0. See `LICENSE` and the surrounding registry for distribution terms. @status:impl/done

