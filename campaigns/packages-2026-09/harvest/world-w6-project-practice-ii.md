# W6 — licensing, source-mirrors, spec-genres, dev-runtime-docs: the three sources

_Captured 2026-07-28 at the W6 opening. Every number below is the output of the
command printed above it._

W6's four flows govern four host artefacts that exist on disk and can be opened:

- **`licensing`** — `LICENSE.xml`, the workspace manifest, nineteen crate manifests.
- **`source-mirrors`** — `mirrors.toml` and `cargo xtask mirror`, which this
  repository runs as its standard rollout. The flow describes a procedure the host
  executes, not one it aspires to.
- **`spec-genres`** — the genre map against a spec tree that has 42 PROP documents
  and **zero** FEAT documents.
- **`dev-runtime-docs`** — `DEV-GUIDE.md` and `RUNTIME-GUIDE.md`, and the
  same-commit rule, which is checkable in `git log` per commit.

## Source 1 — the package agreeing with itself {#source-1}

```console
$ python campaigns/packages-2026-09/tasks/source1-join.py \
    vibevm/vibepacks/org.vibevm.world/licensing \
    vibevm/vibepacks/org.vibevm.world/source-mirrors \
    vibevm/vibepacks/org.vibevm.world/spec-genres \
    vibevm/vibepacks/org.vibevm.world/dev-runtime-docs
source-1 join over 22 file(s) under vibevm/vibepacks/org.vibevm.world/licensing, vibevm/vibepacks/org.vibevm.world/source-mirrors, vibevm/vibepacks/org.vibevm.world/spec-genres, vibevm/vibepacks/org.vibevm.world/dev-runtime-docs
  relative .md citations resolved: 22
  broken: 0
```

**Twenty-two relative citations, none broken.** Clean.

## Source 3 — the installed reality {#source-3}

```console
$ python campaigns/packages-2026-09/tasks/source23-boot-join.py | grep -A3 'world/dev-runtime-docs'
  org.vibevm.world/dev-runtime-docs  [INSTALLED NO-SOURCE]
    installed: vibedeps/flow-dev-runtime-docs/0.1.0/boot/58-flow-dev-runtime-docs.md
    source   : <none found>
```

**Three of four are clean** — `licensing`, `source-mirrors` and `spec-genres` do
not appear on the join's problem list, so each is INSTALLED, SOURCED and
word-identical to what the host boots.

### `dev-runtime-docs` is installed at a path the package no longer ships {#stale-path}

```console
$ find vibedeps/flow-dev-runtime-docs -type f
vibedeps/flow-dev-runtime-docs/0.1.0/boot/58-flow-dev-runtime-docs.md
vibedeps/flow-dev-runtime-docs/0.1.0/LICENSE
vibedeps/flow-dev-runtime-docs/0.1.0/README.md
vibedeps/flow-dev-runtime-docs/0.1.0/spec/flows/dev-runtime-docs/DEV-RUNTIME-DOCS-PROTOCOL.md
vibedeps/flow-dev-runtime-docs/0.1.0/vibe.toml
$ find vibevm/vibepacks/org.vibevm.world/dev-runtime-docs -type f
vibevm/vibepacks/org.vibevm.world/dev-runtime-docs/v0.1.0/LICENSE
vibevm/vibepacks/org.vibevm.world/dev-runtime-docs/v0.1.0/README.md
vibevm/vibepacks/org.vibevm.world/dev-runtime-docs/v0.1.0/vibevm/vibespecs/boot/58-flow-dev-runtime-docs.xml
vibevm/vibepacks/org.vibevm.world/dev-runtime-docs/v0.1.0/vibevm/vibespecs/flows/dev-runtime-docs/DEV-RUNTIME-DOCS-PROTOCOL.xml
vibevm/vibepacks/org.vibevm.world/dev-runtime-docs/v0.1.0/vibe.toml
```

The installed copy carries the boot snippet at **`boot/`**; the package ships it at
**`vibevm/vibespecs/boot/`**. That is the pre-DRIFT-039 layout, so the join's path-matching
finds no source and reports NO-SOURCE — the same shape W2 recorded for
`sync-from-code`. The **prose is the same**: a direct diff shows only the Phase B
markup (`<status/>`, `##ANCHOR`, `@impl/done`) and the heading anchors the package
gained afterwards.

**So this is a fact about the install, not about the rule**, and it belongs to
whichever fact asserts a path or an install layout — not to the rule facts. Note
also that this package ships `LICENSE` where its siblings ship `LICENSE.xml`.

### The sibling-pointer family {#dangling}

```console
$ ls spec/
WAL.md  boot  common  design  manual-tests  modules  terraforms
```

**The host has no `spec/flows/` directory**, so every `../flows/<name>/<file>.md`
pointer in W6's boot snippets resolves nowhere in the consuming project — W1's
69-dangling finding. It is a fact about the pointer, not about the rule the
pointer sits under.

## Source 2 — the host's observed conformance {#source-2}

### licensing — one posture, one exception, and no automated gate {#s2-licensing}

```console
$ head -3 LICENSE.xml
Copyright (c) 2026 Oleg Chirukhin

The Universal Permissive License (UPL), Version 1.0
$ grep -nE '^license' Cargo.toml
55:license-file = "LICENSE.xml"
$ grep -hoE '^license(-file)?[^=]*=.*' crates/*/Cargo.toml xtask/Cargo.toml | sort | uniq -c
     18 license-file.workspace = true
$ for f in crates/*/Cargo.toml xtask/Cargo.toml; do grep -qE '^license(-file)?\s*(\.workspace)?\s*=' "$f" || echo "NO LICENSE FIELD: $f"; done
NO LICENSE FIELD: crates/vibe-index/Cargo.toml
```

**One product licence, stated once and inherited by eighteen of nineteen
manifests — and `crates/vibe-index/Cargo.toml` states none.** The flow's rule is
that every sub-package states the same licence in its manifest; this is the one
that does not. Report it as the single exception it is, with the count on both
sides.

`CLAUDE.md`'s operating-facts ledger carries the host's own licence-state record
(«our shipped surface is fully UPL-1.0», the relicensing commits, and the
enumerated off-limits set) — a durable citation target for anything about the
posture's history.

**The permissive-only dependency rule has no automated gate:**

```console
$ ls deny.toml about.toml
ls: cannot access 'deny.toml': No such file or directory
ls: cannot access 'about.toml': No such file or directory
```

Neither `cargo-deny` nor `cargo-about` is configured, and `Cargo.lock` carries no
licence field at all — so a grep of the lockfile measures package *names*, not
licences. **Do not count `Cargo.lock` hits as licence evidence.** If a fact claims
the rule is enforced, say precisely what enforcement you looked for and did not
find; if it merely states the rule, that is a prescription.

### source-mirrors — the host executes this flow, not merely holds it {#s2-mirrors}

```console
$ cat mirrors.toml
schema = 1

[[target]]
name = "gitverse"
url = "git@gitverse.ru:vibevm/vibevm.git"
mode = "push"
refs = ["main", "tags"]
region = "ru"

[[target]]
name = "github"
url = "git@github.com:vibevm/vibevm.git"
mode = "push"
refs = ["main", "tags"]
region = "us"
```

The file's own header states the flow's model in the host's words — «benevolent
dictator / hub-and-spoke, no primary … Mainline is the maintainer's integrated
local `main` — single-writer … Every target below is a downstream read-replica»
— and documents the three verbs (`mirror`, `--check`, `--from NAME`) with
«fast-forward-only, fail-loud, never `--force`» spelled out. It also states that it
carries **no credentials**, which is the secrets-hygiene composition.

**A live run from this session, for source 2 at its most direct:**

```console
$ cargo xtask mirror
mirror: fanning main @ 8b2a896 out to 2 target(s)
  ok     gitverse main
  track  origin/main -> 8b2a896
  ok     gitverse tags
  ok     github main
  track  github/main -> 8b2a896
  ok     github tags
mirror: all push targets synced.
```

Two targets, both fast-forward, no `--force`. The host's own boot snippet
(`vibevm/vibespecs/boot/90-user.xml`, `##CMD-MIRROR` and `##SRC-MULTI-HOMED`) prescribes
`cargo xtask mirror` over a bare `git push origin`, and `vibevm/vibespecs/common/PROP-016-source-mirrors.xml`
is the governing host contract. **All three are durable citation targets.**

Check separately rather than together: the fan-out mechanics, the daily loop, the
web-UI-merge rule («not integrated until brought home into mainline first»), and
the drift-handling rule. Each has a different observable, and `xtask/src/mirror.rs`
is where the implemented half lives.

### spec-genres — the map names seven genres and one of them has zero instances {#s2-genres}

```console
$ ls spec/
WAL.md  boot  common  design  manual-tests  modules  terraforms
$ ls spec/common/PROP-*.md vibevm/vibespecs/modules/**/PROP-*.md | wc -l
42
$ find . -name 'FEAT-*.md' -not -path './vibevm/vibedeps/*' -not -path '*/.vibe/*'
   (no output)
```

**Forty-two PROP documents and not one FEAT document anywhere in the repository.**
The genre map's «Module contracts (here: PROP / FEAT)» names both, and the tooling
supports both — `vibevm/vibespecs/modules/vibe-workspace/PROP-035-spec-compiler.xml:107` says
«`PROP-NNN` / `FEAT-NNN` in a URI resolve to `PROP-NNN-<slug>.md`», so the resolver
was built for a genre the tree never used.

The rest of the map has instances to check against: boot files (`vibevm/vibespecs/boot/`),
foundational decisions (`vibevm/vibespecs/common/PROP-000.xml`), module contracts
(`vibevm/vibespecs/modules/**`), **design docs** (`vibevm/vibespecs/design/` — 6 files), research docs
(`legacy-spec/research/`, an archive), campaign plans (`vibevm/vibespecs/terraforms/`), and the
checkpoint (`vibevm/vibespecs/WAL.xml`). The flow's two-way-link law — «a contract section that
has lore links to it; the lore names the section it explains» — is checkable in
both directions across `vibevm/vibespecs/design/` and `vibevm/vibespecs/modules/`.

### dev-runtime-docs — the same-commit rule, and a commit that keeps it {#s2-runtime}

```console
$ wc -l DEV-GUIDE.md RUNTIME-GUIDE.md
  344 DEV-GUIDE.md
   70 RUNTIME-GUIDE.md
$ git log -1 --format='%h %ad %s' --date=short -- DEV-GUIDE.md
cb14fe5c 2026-07-26 fix(vibe-core): there is one per-user home, and one place a credential lives
$ git show --stat --format='' cb14fe5c
 DEV-GUIDE.md                                  |   2 +-
 RUNTIME-GUIDE.md                              |   4 +-
 campaigns/progress-2026-08/tasks/DRIFT-021.md | 108 ++++++++++++++++++++++
 crates/vibe-cli/src/commands/aiui/control.rs  |  49 ++++------
 crates/vibe-core/src/settings.rs              |  59 +++++++-----
 crates/vibe-publish/src/github.rs             |   5 +-
 crates/vibe-publish/src/lib.rs                |   6 +-
 crates/vibe-publish/src/token.rs              | 128 ++++++++++++++------------
 8 files changed, 235 insertions(+), 126 deletions(-)
```

**This is the flow's rule executed.** A change to the per-user home
(`vibe-core/src/settings.rs`) and the credential path
(`vibe-publish/src/token.rs`) — squarely «paths» and «environment variables» —
carries `DEV-GUIDE.md` and `RUNTIME-GUIDE.md` in the *same commit*. Both guides are
last touched by exactly that commit.

**Do not stop at one instance.** The rule is a per-change obligation, so the honest
measurement is over a window: how many commits touching the toolchain,
prerequisites, env vars, paths or bootstrap steps also touched a guide, and how
many did not. Say what window you measured and how you selected the commits.

## The nineteen files and their anchor counts {#files}

Measured from `campaigns/packages-2026-09/run/mirror/`; the total agrees with
`tasks/PHASE-C-BATCHES.json` (`W6 … 19 files, 700 markers, 572 anchors`).

```
dev-runtime-docs (30)
   9  vibevm/vibepacks/org.vibevm.world/dev-runtime-docs/v0.1.0/README.md
   6  …/spec/boot/58-flow-dev-runtime-docs.md
  15  …/spec/flows/dev-runtime-docs/DEV-RUNTIME-DOCS-PROTOCOL.md
licensing (137)
  24  vibevm/vibepacks/org.vibevm.world/licensing/v0.1.0/README.md
  17  …/spec/boot/60-flow-licensing.md
  48  …/spec/flows/licensing/LICENSING-PROTOCOL.md
  30  …/spec/flows/licensing/dependency-licenses.md
  18  …/spec/flows/licensing/eula-template.md
  14  …/spec/skills/draft-eula/SKILL.md
source-mirrors (200)
  23  vibevm/vibepacks/org.vibevm.world/source-mirrors/v0.1.0/README.md
  16  …/spec/boot/62-flow-source-mirrors.md
  52  …/spec/flows/source-mirrors/SOURCE-MIRRORS-PROTOCOL.md
  52  …/spec/flows/source-mirrors/daily-loop.md
  57  …/spec/flows/source-mirrors/fanout-mechanics.md
spec-genres (191)
  26  vibevm/vibepacks/org.vibevm.world/spec-genres/v0.1.0/README.md
  23  …/spec/boot/17-flow-spec-genres.md
  59  …/spec/flows/spec-genres/SPEC-GENRES-PROTOCOL.md
  41  …/spec/flows/spec-genres/design-docs.md
  42  …/spec/flows/spec-genres/when-to-write-what.md
```

**`licensing` ships a `SKILL.md` with 14 anchors** (`draft-eula`), and the same two
questions apply as to `health-audit`'s: F-092 says a `SKILL.md`'s YAML frontmatter
cannot carry a fact anchor (already filed, 9 files); and `.claude/skills/` holds
five skills, none of them `draft-eula`.

**One standing rule for the `licensing` slice: relicensing is an owner decision and
this campaign does not take it.** Phase C files findings; it does not repair the
subject it measures.

**Scope:** §3.1 sources 1, 2 and 3 for the four flows of batch W6.
