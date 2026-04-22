# Manual tests

This directory holds **human-runnable** smoke-tests for vibevm —
scripts a person reads top-to-bottom and executes on their own
machine. They cover scenarios the automated `cargo test` harness
cannot reach: anything that needs a real remote git registry, a real
SSH identity, a real `~/.vibe/` tree on a real filesystem, or a
human-judgement "does this look right?" check.

These are not a substitute for `cargo test`. They are the *last mile*.

## When do you run these?

- **Before tagging a milestone.** Walk the tests for every feature
  the milestone claims to ship.
- **After touching an integration surface** (git backend, CLI arg
  parsing, lockfile format, registry layout) even if `cargo test`
  stays green — unit tests use fakes and tempdirs; the real world is
  messier.
- **When a user reports a reproducer.** Add their steps here so the
  next session can replay them.

`cargo test` stays the fast feedback loop for refactors; these
scripts are for the slower, higher-confidence pass.

## Running any test

Every test file in this directory is self-contained and **starts
from a clean slate**. The pattern is:

1. Read the file top to bottom before touching anything.
2. The file's "Preflight" section lists prerequisites (tools, network
   access, credentials) — verify them first.
3. The "Setup — clean slate" section creates an isolated scratch
   directory and sets `VIBE_REGISTRY_CACHE` to point into it, so the
   test never touches your real `~/.vibe/registries/`.
4. Work through the numbered steps. After each step, the file tells
   you what output to expect; if something differs, stop and
   investigate before moving on.
5. The "Cleanup" section removes the scratch directory.

Do not skip steps, do not improvise. The point of a manual test is
reproducibility — if you change the recipe, update the file.

## Conventions for new tests

Each test is a single Markdown file named
`<milestone>-<slug>.md` — e.g. `M1.1-git-registry-smoke.md`,
`M1.2-update-diff.md`. The filename is the index; there is no
separate registry.

A test file carries these sections in this order:

1. **Purpose.** One-paragraph description: what feature, what
   scenario, why this needs a human.
2. **Preflight.** Tools / credentials / network access required.
3. **Setup — clean slate.** Exactly how to isolate the run. Always
   use `VIBE_REGISTRY_CACHE` and a fresh scratch `$PROJECT` directory
   under `mktemp -d` (or `$(mktemp -d)` in Git Bash — works on
   Windows too). Never write into the user's `~/.vibe/`.
4. **Steps.** Numbered. Each step has a command in a fenced block
   and an "Expected" subsection describing the observable outcome.
5. **Cleanup.** One copy-pasteable block that tears down the scratch
   state.
6. **What to file if it fails.** What artifacts to collect
   (`VIBE_LOG=debug` output, lockfile contents, cache tree) so a
   follow-up session can diagnose.

Keep commands POSIX-shell compatible. The author's primary machine is
Windows running Git Bash; macOS and Linux must work too. When
platform behaviour diverges (path separators, executable suffix),
show the Windows form first with a note.

Keep each file under ~300 lines. If a test is larger than that, it is
really two tests — split them.

## Index

| File | Milestone | Feature |
| --- | --- | --- |
| [`M1.1-git-registry-smoke.md`](M1.1-git-registry-smoke.md) | M1.1 | Install from the real GitVerse registry, `vibe registry sync`, lockfile `source_uri` shape. |
| [`M1.5-gate-multi-package-smoke.md`](M1.5-gate-multi-package-smoke.md) | M1.5-gate | Install three flows from the same registry in one project; distinct boot-snippet prefixes; one shared clone; symmetric uninstall. |

Add a row to this table when you add a test. Keep the table sorted by
milestone.

## Why here and not in `docs/`

`docs/` is for end-user documentation — it has to stay discoverable
and browseable. `manual-tests/` is a contributor-facing checklist; it
documents how we verify the product, not how to use the product.
Separate audiences, separate trees.
