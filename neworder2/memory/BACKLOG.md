# Backlog — deferred items surfaced during the extraction

Owner-authorised deferrals. Move each to its proper home (a PROP / ROADMAP entry) when picked up.

## B1 — `transitive-inline` / `transitive-static` link types (owner, 2026-07-14)

**Problem found.** A family aggregator (e.g. `git-practices`) declaring a member `link = "inline"`
in its `[requires.packages]` does **not** propagate to the member's effective boot: `bootgen.rs`
resolves a dependency's `declared_link` from a **single** node's manifest (the consumer/root),
so an aggregator-declared link on a *transitive* member is not honoured — the member falls back
to its own `[boot_snippet]` suggestion or the `static` default (PROP-009 §2.4; `boot.rs`
`declared_link.or(suggested_link)`).

**Proposal (owner).** Add `link` variants that DO cross the transitive boundary — `transitive-inline`,
`transitive-static` (and the `-dynamic` twin) — meaning "apply this inclusion type to this package
**and everything it pulls transitively**". Then a consumer requiring `git-practices` with
`link = "transitive-inline"` makes the whole family load inline, without each member having to
self-suggest it. Same semantics as the existing `inline`/`static`/`dynamic`, only transitive.

**Home when built:** PROP-009 §2.4 (inclusion types) + `vibe-workspace` boot resolution + the
`vibe-core` manifest schema for the `link` enum.

**Interim (in force now).** The four `git-practices` members self-suggest `link = "inline"` in
their own `[boot_snippet]` so the commit rules land in `INLINE.md` today. When `transitive-inline`
lands, that per-member suggestion can be dropped and `git-practices` can declare it once.

## B2 — specmap should skip generated boot artifacts (found 2026-07-14)

**Problem found.** `spec/boot/INLINE.md` is the verbatim concatenation of the `inline`-linked
members' boot snippets, each of which opens with `# Flow: … {#root}`. The specmap scans
`spec/**/*.md`, so it sees N `{#root}` anchors in INLINE.md and emits `duplicate-anchor`
**warnings** (3 today, one per extra inline member). They are cosmetic — INLINE.md's internal
anchors are not citation targets (a member's real address is
`spec://org.vibevm.world/<member>/…#root`, which resolves) — and non-fatal (self-check green).

**Proposal.** The specmap should **exclude generated boot artifacts** (`spec/boot/INLINE.md`,
`spec/boot/INDEX.md`) from the anchor scan — they are machine-generated concatenations/manifests,
not authored spec units. Either a `specmap.toml` exclude for generated boot files, or the engine
skips the files the loading model marks generated. Alternatively the inline lane could namespace
each snippet's anchors, but excluding the generated artifact is simpler and correct.

**Home when built:** the `rust-ai-native-specmap` scan + `specmap.toml` (host policy).
**Interim:** accept the N `duplicate-anchor` warnings on INLINE.md as known/cosmetic (0 suspects).

## B3 — regenerate the fractality nested-project lock post-rename (found 2026-07-14)

**Problem.** The git-practices member rename (git-atomic-commits, git-attribution-policy, …)
renamed the package dirs and each member's `vibe.toml` `name`, so the fractality nested project's
`packages/org.vibevm.fractality/fractality/v0.1.0/vibe.lock` — which pins some of those members —
is stale: its `source_url` paths, `name` fields, and (because the `name` change alters package
content) `content_hash` values no longer match. The rename sed touched only the `source_url`
paths, half-updating a generated file, so that edit was reverted rather than committed.

**Fix (a fractality session):** run `vibe install` in the fractality project to regenerate the
lock cleanly. Low priority — the nested project is a specspace, not currently installed from here.
Do not hand-edit the generated lock.

## B4 — DONE (2026-07-14, commit 4720d65): trio delegation operational block thinned

The three sections below were collapsed to one "Running fractality here" note (vibevm's exact
entry points — launcher path, build, run/spawn/wait/route/gate, opencode route, the RLM-plan
reference, the ultracode/Workflow-can't-spawn-GLM caveat) that cites the package for the directive
and the ledger for the facts; the swarm-internals prescriptions were removed. Original scope, for
the record:

`delegation-first` now carries the concrete **first-level** usage (fractality run/spawn/wait,
GLM-5.2 as `big`, enable RLM via the profile flags, the fan-out hand-off, the 5/95 target). Done in
the trio so far: the general obligations removed, the four commit rules → a git-practices pointer.
Still redundant in the trio's Delegation-first block and to be thinned:

- **"How to run it, in-place"** — the general spawn commands are in the package; keep only the exact
  vibevm residue (the launcher path `packages/org.vibevm.fractality/fractality.ps1`, the build
  command) and cite the package for usage. Much overlaps the ledger's "Build / run" line — dedupe.
- **"Enable RLM"** — the enable (allow_tools / ask_boss / `fractality gate`) is the package's
  `#recursion`; keep only the vibevm pointer (the `FRACTALITY-RLM-PLAN` reference) + cite.
- **"Swarm modes"** — the package's `#swarm` covers first-level; the old section prescribed swarm
  **internals** (thin bosses, launcher arithmetic) the owner rejected — remove that, keep at most
  "swarms route through fractality (package `#swarm`)".
- Update the "What follows" framing line to match.

Keep verbatim: the operating-facts **ledger** and the **Rule 1 & 4** binding. Trio byte-identical.
The delicate part is judging the exact vibevm residue vs the package/ledger overlap — best with
fresh context. Also worth an owner re-read of the reshaped `DELEGATION-FIRST-PROTOCOL.md`.
