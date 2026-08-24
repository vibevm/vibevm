# The `world` evidence brief — what a delegated search returns, and what it must not

_Written 2026-07-28 at the W1 opening, from what the `ai-native` cluster's fifteen
delegated tables cost and bought. Read this whole file before starting._

## Your job, and the one thing that is not your job {#job}

You gather **evidence**. You do not write verdicts, and you never guess.

For every marked fact in the files you are given, return a row saying what the fact
claims, where in this repository that claim can be checked, and what you found
there. A reviewer reads every row and decides `confirmed` / `drift` /
`unverifiable`. **A row that argues for a verdict is worth less than a row that
says precisely what it searched** — because the reviewer's disagreement with a
well-searched row costs one lookup, and the same disagreement with an
under-searched row costs the whole search again.

## The three sources a `world` fact rests on {#sources}

This corpus is `org.vibevm.world` — prompt-only behavioural flows. A flow has no
crate to point at, so its facts are checked against three sources, and **every row
records which of them it used**, in a `src` field holding a non-empty subset of
`[1, 2, 3]`:

**1 — the package's own shipped artifacts.** A boot snippet that claims a rule must
contain it; a protocol document a snippet cites must exist and say what the snippet
says it says. This is the weakest source: the package agreeing with itself.

**2 — the host's observed conformance.** This repository is a living consumer of
every flow it installs. `CLAUDE.md` / `AGENTS.md` / `GEMINI.md`, `vibevm/vibespecs/boot/**`,
`vibevm/vibespecs/WAL.xml`, `CONTINUE.md`, `spec/common/PROP-*`, `vibevm/vibespecs/modules/**`, the
`campaigns/**` zones, `AUDIT.md`, `TASKS.md`, `BACKLOG.md`, the crates — and, for
the git flows, **this repository's own `git log`**. If a flow promises a behaviour,
the host either behaves that way or does not, and that is checkable.

**3 — the installed reality.** What a consumer actually receives: `vibedeps/<slot>/`
on disk, plus the generated `vibevm/vibespecs/boot/STATIC.xml` and `vibevm/vibespecs/boot/INDEX.md`, which
are COMPILED from the installed packages and carry a provenance marker per
contribution. (`files_written` in `vibe.lock` is `[]` for all 36 packages, so
`vibedeps/` is the sanctioned substitute — batch plan §2.3.)

**Say which sources you actually used.** A row whose evidence is only the package's
own text is `src: [1]`, and the campaign counts those separately as
self-referential. Do not pad the list: `src` records what you looked at, not what
you could have.

## The perimeter — it is almost never one package {#perimeter}

Searching only the package a fact lives in produced five wrong absences in the last
cluster, and not one of them was a worker's error: each brief pointed the search at
the wrong place. So the default perimeter is all of this:

```
vibevm/vibepacks/org.vibevm.world/**   vibevm/vibepacks/org.vibevm.ai-native/**   vibedeps/**
spec/**   crates/**   xtask/**   schemas/**   campaigns/packages-2026-09/harvest/*.md
CLAUDE.md  AGENTS.md  GEMINI.md  MEMORY.md  README.md  AUDIT.md  TASKS.md  BACKLOG.md
CONTINUE.md  ROADMAP.md  DEV-GUIDE.md  RUNTIME-GUIDE.md  SPECSPACES.md  CHANGELOG.md
conform.toml  specmap.toml  specmap.json  vibe.toml  vibe.lock  progress.toml
.claude/skills/**   research/rust-demo/**   research/ts-demo/**   research/go-demo/**
```

Two rules follow, and they are the two that matter most:

- **A `not-found` is a fact about the search perimeter until the perimeter has been
  checked.** It is never, on its own, evidence of absence. Before you write
  `not-found`, widen once and say in `searched` where you widened to.
- **When you find something OUTSIDE the nominal package, SAY WHERE.** That
  relocation is the single most valuable thing a delegated search returns.

**Exclude `node_modules/`, `.vibe/cache/` and `vibedeps/` when counting a demo
project's own practice.** A count that includes them is a count of somebody else's
code — that mistake put ten wrong verdicts into the last cluster. (`vibedeps/` is
still in the perimeter for source 3, where it is the *subject*, not the sample.)

**`legacy-spec/` is an archive.** Nothing living may rest on it as a normative
source; if a fact's only support is there, say so — that is itself the finding.

## The row format {#format}

Return a JSON array. One object per marked fact, in document order:

```json
{
  "file":   "vibevm/vibepacks/org.vibevm.world/<pkg>/<ver>/<path>.md",
  "anchor": "THE-ANCHOR-ID",
  "marker": "@impl/done",
  "claim":  "one line, in your own words, of what this fact asserts",
  "evidence": [
    "path/to/file.ext:123  the exact text at that line, quoted",
    "path/to/other.md:45  ..."
  ],
  "src": [1, 2],
  "found": "located",
  "searched": "what you looked for, where, and what came back — including the widening"
}
```

- **`evidence`** — each entry is `<repo-relative path>:<line>  <verbatim snippet>`.
  The line number must be right; a checker resolves every one of them before the
  reviewer reads a word, and a ref that does not resolve discredits the row. Quote
  verbatim; if you shorten, use `…` and keep the segments in order.

  **The grammar is exactly that and nothing else.** W1 returned 43 refs the
  checker could not resolve and **not one of them was a fiction** — every one was
  a notation the grammar does not admit. Avoid all four:

  - **No bare paths.** `path/to/file.md  [installed; present in the payload]` has
    no line number and cannot be checked. Cite a line — even line 1 — or put the
    observation in `searched`.
  - **No ranges.** `AUDIT.md:1-458  [458 lines; no attribution line]` is an
    *absence*, and an absence has no line to point at. Absences belong in
    `searched`, stated as the command that found nothing.
  - **No added markup.** If the file's line reads `type(scope): subject`, quote it
    that way. Wrapping it in a code span — `` `type(scope): subject` `` — is your
    formatting, not the file's text.
  - **No re-escaping.** A line ending in a shell continuation `\` must be quoted
    with ONE backslash. JSON-escaping it twice produces `\\`, which is a different
    string from the one in the file.

  A ref that will not fit the grammar is not evidence you should drop — it is an
  observation that belongs in `searched`, where prose is welcome and nothing is
  parsed.
- **`found`** — exactly one of:
  - `located` — the evidence settles the claim, one way or the other;
  - `partial` — you found related material that does **not** settle it. This is the
    class that carries drift, so `searched` must say precisely what is missing;
  - `not-found` — nothing on the widened perimeter bears on the claim.
- **`searched`** — the terms, the paths, the counts, and the widening. Write it for
  a reader who will disagree with you.

## Never {#never}

- Never write a verdict, a recommendation, or the words «probably» / «likely» /
  «should be». Report what is there.
- Never invent a line number or a path. If you cannot cite it, say so in
  `searched`.
- Never classify a group of facts by one rule to save time. **138 rows were sorted
  by filename in the last cluster and every one of them had to be read again.** A
  row you did not actually search is worse than a row you marked `not-found`.
- Never edit any file in the repository. You read and report; the boss writes.
- Never leave `src` empty, and never claim a source you did not consult.
