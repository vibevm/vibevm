# Programme of 2026-08-06 — the owner's rulings, in the order he set

**status: AUTHORED 2026-08-06 · NOT STARTED · order fixed by the owner: Б → В → А**

_This file is a **plan**, not a contract. It asserts nothing about the tree; it
records what was decided, why, and in what order. When its work is done it is
deleted and nothing breaks — that is the owner's ruling of this same day, and
this file is its first subject. Rows below carry tombstone lines as they close;
those tombstones are process support for whoever is walking the plan, not part
of the project's structure._

---

## 0. How to read this and what it replaces

This programme came out of one long owner conversation on 2026-08-06. It does
**not** replace the running campaign (`PACKAGES-ACTUALIZATION-CAMPAIGN-v0.1.md`,
Phase E). It is the work the owner authorised during that conversation, most of
which did not exist in any list before it.

**The course changed, and the change is worth stating plainly.** The standing
course was «drain the backlog first». Over that conversation the owner
authorised **eighteen work items** while the session closed **one** backlog row.
Almost all of it is better than what the backlog held — but it is a new
programme, not a drain, and the next session should know that before it opens
`BACKLOG.md` expecting the old course.

**Order, fixed by the owner:** **Б → В → А**. See §1 for what those are and the
reasoning that produced that order.

---

## 1. The three groups and the order

The eighteen items sort into three groups. The owner chose the order after the
boss recommended it and gave its reasoning; both are recorded because the
reasoning is what makes the order re-derivable if circumstances change.

### Group Б — hygiene: the work that makes everything after it cheaper {#group-b}

Do this first. **Reason (boss's, accepted by the owner):** the expensive thing
this week was never a missing feature — it was **the project lying in its own
records**. Numbers that do not reproduce, links pointing at deleted rows, an
owner guide promising a gate step that does not exist. Until that is cleaned,
every large piece of work drags an excavation behind it.

### Group В — taxonomy: the foundation the later features stand on {#group-v}

Second. **Reason:** the omnichannel package and the `lang` package kind are what
`vibe tools`, the language roster and the surface norm all rest on. Building
those first means building them twice.

### Group А — the index, including the blocker {#group-a}

Third, **even though it contains the item the owner himself called a blocker.**
**Reason (boss's, accepted):** the publication blocker does not stop us
working — it stops us *releasing*, and a day or two changes nothing there, while
groups Б and В compound.

---

## 2. Group Б — hygiene {#b-items}

### Б1. The plan-closure rule — write it down first {#b1}

**The owner's ruling, near-verbatim (2026-08-06):**

> A plan is a temporary thing. When it is executed it must be possible to delete
> it and nothing breaks.
>
> All significant content moves **into the specifications** on closure. Marked
> statements point at **concrete elements of specifications**, never at rows of a
> plan — a link to a plan row is a link to something designed to disappear.
>
> Closed an item — then rebuild the statements and the specs so that no tails
> remain. That is part of closing, not tidying afterwards.
>
> Tombstones inside a plan are temporary process support: they exist for the
> human and the agent walking the plan, and they are not part of the project's
> structure. When the plan ends they go with it.

**Where it lives.** The rule splits in two, exactly as the planning-medium norm
did earlier the same day (see the closed row B-032, whose reasoning lives in
commit `0f80a804`):

- *«content and citations move into the specs on closure»* — this is about
  addressing. Home: the `addressable-specs` flow.
- *«a plan is temporary; tombstones are process support»* — this is about a
  plan's lifecycle. Home: the `campaign-plans` flow.
- One pointer between them. **No restatement** — one law, one home.

**Why Б1 is first in its own group:** everything else in group Б is an
application of it.

### Б2. Migrate the citations, then tombstone the closed rows {#b2}

**Measured on 2026-08-06, before any deletion:**

- `BACKLOG.md` holds **48 rows**; **35** are finished work kept under the old
  convention (flip a field), **11** are live, **2** are `accepted` (recorded, no
  action planned).
- **22 citations from 11 live documents** point at rows that would be deleted:
  eight design documents under `spec/design/`, one PROP under
  `spec/modules/vibe-workspace/`, plus `TASKS.md` and `TOOLING-MAP.md`.
- **Those citations sit inside marked statements**, so editing them brings
  roughly 22 facts due for re-judgement.
- **8 citations are ALREADY dangling** — pointing at rows closed in earlier
  sessions, including two closed the day before. The law «a row takes its
  citations with it» was written and not executed on the last closures.

**What to do, per Б1 rather than per the boss's original either/or:**

1. **Re-point the 22 citations at spec elements**, not at plan rows. Where the
   row's content moved into a spec, the citation names that spec element. Where
   it did not move (the row was a finding, not a contract), the citation becomes
   text without an address — the row number stays as history, the link goes.
2. **Repair the 8 already-dangling ones** the same way, in the same pass.
3. **Tombstone the 35 closed rows** — one line each: number, title, «closed,
   the ruling lives in the commit that closed it». The file falls from ~1000
   lines to roughly 200. Tombstones are temporary and go when the plan does.
4. The ~22 re-judgements that follow are ordinary campaign work; they are
   **not** avoided by tombstoning, because the citations move regardless.

**Rejected alternatives and why:** *(a)* wholesale delete without migrating
citations — breaks 22 live links, which is the defect the law exists to prevent.
*(b)* move closed rows to a history file — the anchors break identically,
because links address `BACKLOG.md#b-NNN` specifically.

### Б3. The panel gains the markup-validation step {#b3}

**The gap, measured:** `tools/self-check.sh` runs **no** progress verb at all —
the single occurrence of the word in it is a comment about a config file. The
owner guide told the owner it was in the panel. That half-line was corrected on
2026-08-06; the step itself is not built.

**Proof it matters, from the same session:** a design document went into a
commit carrying **five statements with no markers**, and nothing said anything —
not the panel, not `vibe check`, not the quality gate, not the map
regeneration. It surfaced only because the corpus happened to be rescanned for
an unrelated reason. In that same minute the default `vibe progress check`
printed «clean, 275 files, 0 warnings», because the unmarked walk lives behind
`--exhaustive`.

**Why the false claim survived so long, and it is the more interesting half:**
it sat **inside a fenced code block**, as a comment on a command line. A fenced
block carries no anchor, so no verdict was ever about it and nothing could
falsify it. That is the second instance of one law in a week — the first was a
layout diagram drawing a directory that had not existed for months. **What to do
about claims inside fences is a larger question than this item and is named, not
answered, here.**

**The hazard the build must measure BEFORE stepping into it** (owner accepted
this caveat explicitly): the panel snapshots the operator's real `~/.vibe` at
start and compares it after the workspace test run; any `vibe` verb writing that
home inside the window fires the tripwire. A false red was already paid for this
way on 2026-08-05. `progress check` declares itself read-only with its writing
tail behind `--write-state` — **that is a claim to verify, not to assume**, and
where the step sits relative to the tripwire comparison is part of the same
build.

**Open question the build must answer:** what counts as the campaign zone when
the panel runs outside a campaign, and whether the step should read the host's
own scope config instead.

**Filed as** `BACKLOG.md` B-063.

### Б4. The verdict report grows a second column {#b4}

**The problem, measured:** of ~11 863 recorded verdicts, **4 151 (35 %)** have as
their entire evidence a paragraph shared with other verdicts. The largest single
blob covers **276** anchors. For `PROP-008` it is **90 anchors on one paragraph**,
and that paragraph contained a false clause: it asserted a `kind` check was
implemented when the type existed in no source file. The same campaign, judging
the same claim from the package side with per-fact evidence, marked it `drift`
four times. The corpus contradicted itself and the shared-evidence side was
wrong.

**The owner's ruling: variant (в).** Not (а) «leave it» and not (б) «require a
code reference per verdict», which would move ~4 151 verdicts to «unverified»
overnight and drop the headline from 98 % to about 63 %.

**What (в) means in practice — four steps, and it needs no migration to start:**

1. The summary prints **two numbers instead of one**: verified per-fact, and
   verified at document level.
2. The split is **computed mechanically today** — a verdict whose evidence blob
   is shared with other verdicts is document-level. Nothing to re-judge to begin.
3. Conversion happens by itself over time: when a document's text moves, its
   facts come due for re-judgement anyway, and at that moment they gain their own
   evidence.
4. **Re-judge the 90 `PROP-008` anchors now** — the batch already known to have
   carried a lie.

**The two terms must be defined where the numbers are printed**, in the tool's
own output and in the verdict standard, so a future agent does not have to ask
what they mean. Draft definitions:

> **Verified per-fact.** This statement has its own evidence record naming a
> concrete place in code or in another document. If the statement is false the
> evidence collapses with it. Falsifiable pointwise.
>
> **Verified at document level.** One evidence paragraph is stamped on several
> statements at once — somebody read the document whole and concluded it is
> implemented. If one of them is false, the paragraph about the rest still looks
> right, and the lie does not surface.

Plus the rule in one sentence: *a verdict stays document-level until its fact's
text moves or somebody re-judges it deliberately.*

**Measured unit cost of doing it properly** (boss, three facts on 2026-08-06):
15 evidence items, 11 of them `file:line`, from eight measurement commands over
six sources and one test — five refs per fact against the one shared blob. The
standard is payable. It does **not** scale linearly: facts in one document share
the reading, and multiplying five-by-4 151 would be exactly the unearned number
the finding objects to.

**Filed as** `AUDIT.md` `2026-08-06-01`, **P1**, still open — the three questions
it puts are now answered by this ruling except the third, which this item
executes.

### Б5. Documentation examples — the editorial policy {#b5}

**The question, reframed:** package references can be written short (`flow:wal`)
or fully (`org.vibevm.world/wal`). The docs use the short form in **234 places
across 38 files**. The row used to call these errors; they are not — **the short
form is legal input and is implemented**.

**How short and full coexist** (the owner initially believed the short form had
died with reverse-FQDN naming; it has not):

> The **full** form is what is **stored** — manifests and the lockfile always
> carry it. The **short** form is what is **typed**: it lives exactly at the CLI
> input boundary, is expanded once, and only the full form travels onward.

**What works today, measured:** `vibe install wal` resolves through the lockfile
first, then registry indexes; `vibe uninstall wal` / `vibe update wal` resolve
from the lockfile **alone, with no network and no index**; the three
registry-redirect verbs require the full form, with the reason recorded beside
the code (they act on a package that need not be installed, so there is no
lockfile to answer from).

**The caveat that decides the policy:** `vibe install <short>` for a package not
already locked **requires a configured registry index**. Without one the short
form does not resolve and the full form does.

**The policy (boss's, owner agreed):**

| context | form | why |
|---|---|---|
| a command the user types for **install** | **full** | works unconditionally; short depends on an index being configured |
| a command for **uninstall / update** | **short** | resolves from the lockfile with no conditions — it is what a person actually types |
| **contents of a file** (`vibe.toml`, `vibe.lock`, JSON output samples) | **full** | that is the only form ever stored there |
| **prose** | discretion | — |

**Scope:** about 20 files carry real examples. **Roughly ten `commands/*` pages
carry only false positives** and need no edit at all — their apparent matches are
JSON `"command"` labels (18 of them), git permission scopes (~11), SCP-URL
fragments (~12) and literal `<kind>:<name>` grammar tokens (~4), about 45 in
total of the 234.

**Filed as** `AUDIT.md` `2026-05-23-10`. **Its count has now been wrong three
times** — ~40, then 169 across 27 files, then 234 across 38 — against a
directory unchanged since 2026-07-26. Each correction came from widening the
perimeter of the measurement, not from fixing a pattern.

### Б6. Normalise `files_written` to forward slashes {#b6}

**The defect:** `vibe list --json` emits file paths with native separators;
the MCP `query_package` tool emits the same paths POSIX-normalised. **Two
surfaces of one capability print different values on Windows.**

**Owner ruling: normalise both to forward slashes.** Rationale: it is
machine-readable output, and a backslash in JSON is an escaping hazard.

**Note:** this changes published CLI JSON output. It was asked rather than fixed
silently for that reason.

### Б7. Record the facts that need no decision {#b7}

- **All seven generated wire contracts are permissive** — none carries
  «reject unknown fields» — **not by decision but because the generator cannot
  emit it**. Hand-written host types carry it in ~63 places. Write this down so
  the next reader does not mistake it for a choice.
- **The index's documentation has drifted from its code.** The docs describe a
  file layout `by-name/<kind>/<name>.json` and server routes carrying the package
  kind; the code moved to `by-name/<name>.json` with **group** in the routes
  instead of kind. Somebody building a consumer from the docs builds something
  that does not work. Same class as the docblock that promised two surfaces and
  had neither (fixed 2026-08-06, commit `9efc9293`).
- **The `vibedeps` literal leaks into the discipline engine.** The quality
  engine's directory skip-lists for TypeScript and Go hard-code the string
  `vibedeps`, next to universal names like `node_modules`, `target`, `vendor`.
  It is the only vibevm-specific name in the engines' runtime behaviour and it is
  not configurable. **Owner: put it in the backlog, fix before Phase T.** Effect
  is trivial (a directory literally so named is skipped) but it is a leak, and
  fixing it means re-vendoring into 21 copies.

---

## 3. Group В — taxonomy {#v-items}

### В1. The omnichannel package {#v1}

**The problem that produced it.** The owner's standing norm (2026-08-02): logic
shared between the MCP server and the command line belongs in a library, with
both as thin surfaces over it, instead of being nailed to one implementation.

To write that norm down it needed a home, and the backlog row proposed seating
it beside the «four-layer model» — spec, engine, driver, deployment. **That model
does not exist in the discipline package: zero occurrences of the words `DRIVER`
and `DEPLOYMENT` across the whole namespace.** It lives only in `spec/WAL.md`
(rewritten wholesale every session-end and excluded from the corpus by config),
in two campaign documents (a zone disposable by design), and retold in prose
across about ten harvest files. A load-bearing architectural idea, written twelve
times and verified nowhere.

**The owner's proposal, which is better than the boss's recommendation and
replaces it:** a new package **`org.vibevm.world/omnichannel`**.

> The boss had recommended putting the model into the discipline's manifesto.
> The owner asked why it would live in the AI-Native discipline at all — and he
> is right. The model is about **how a capability reaches a user**, not about
> writing AI-native code. The `world` group is where cross-cutting practices
> live that a project **installs if it wants**: git practices, addressable specs,
> campaign plans, source mirrors. Omnichannel is exactly that genre.

**The surface vocabulary (owner's list, boss's grouping):**

```
Library  ─── not a surface but what every surface sits on
             (the only mandatory one, once there is more than one surface)
    │
    ├── local, synchronous:   CLI · TUI · GUI
    ├── agent-facing:         MCP · LSP · IDE extension (VSCode, IDEA, Zed)
    └── networked:            REST · GraphQL · Queue (Kafka …)
```

**Why the grouping earns its place:** the class decides *what repeats*. Local
surfaces share the problem of rendering one dataset into different views;
agent-facing ones share tool description and schema; networked ones share
contract versioning and compatibility. «Logic in a library» is one law for all
three; what counts as a *thin* surface differs per class.

**What the package carries:**

1. The idea: a capability lives in a library; surfaces are thin; none is «the
   base».
2. The vocabulary above, so every project names surfaces identically.
3. The rule that a project **declares its own set** — a surface not declared is
   not a debt.
4. The rule that a new capability is born with all declared surfaces or with a
   recorded reason why one sufficed.

**Then vibevm imports it and declares its floor:** library + CLI + MCP for most
subsystems, TUI where one exists (`vibe tree` has one today). The AI-Native
language stacks import it and declare theirs. **LSP and IDE extensions are
deliberately NOT declared** — the owner said he will launch that work himself,
and by rule (3) an undeclared surface is not a debt.

**The boss's addition, accepted in principle:** the table of «which capability
has which surfaces» **must not be hand-maintained** — it would rot exactly like
everything else this week caught. It can be derived: the map now knows every CLI
command by name (56 command nodes landed 2026-08-06), MCP tools are registered in
one place, TUI screens are enumerable. **So the vocabulary must be
machine-readable, not prose only**, even though deriving the table is a separate
build.

**Measured surface state of the host** (the census that motivated all this,
`campaigns/packages-2026-09/harvest/g6-b047-surfaces-census.md`): of 29 top-level
commands, **19** keep their substance in a separate crate and **10** keep it
inside `vibe-cli` — the largest being the whole version manager `vibe self`. Of
5 MCP tools, **2** share a library function with their CLI twin, **2** have no
CLI twin at all, **1** (`query_package`) reads the same data as `vibe list` and
builds its own output by hand.

**Filed as** `BACKLOG.md` B-047.

### В2. The `lang` package kind {#v2}

**Owner's ruling: add a new package kind `lang`, alongside `stack` and the
rest, and give it to the AI-Native language packages.**

**The owner's reasoning, recorded because it is stronger than the boss's:** a
package that explains **how to write in something** is a genre in its own right
and is wider than AI-Native. `github-flavored-markdown` would be a `lang`
package explaining how to write markdown. A stack demanding a particular way of
writing C++ would require a `lang` package describing that way.

**The boss's supporting argument:** it repairs the meaning of `stack`. Today
`kind = "stack"` is carried by both language packages and «family aggregators»
which contain nothing but pinned versions of three other packages — two genres
under one word. After the split, `lang` means language guidance and `stack`
means a family bundle, and each word means one thing.

**The argument the boss WITHDREW, and why it matters that it is withdrawn:** the
boss first justified `lang` by «we need a roster of installed languages». That
is false. **The boot lane already names the installed language stacks** — each
language package contributes a snippet, and an agent reads at session start that
this project follows the Rust guide and the TypeScript guide. The roster question
is answered before the first question is asked. Do not re-derive that argument.

**What is genuinely unanswered by the boot lane, and what `lang` also does not
answer:** the boot lane says *which discipline to hold*. It does not say *what
can be invoked* — which binaries and which servers those languages brought. That
data exists (`collect_binaries` / `collect_mcp_servers` in `vibe-workspace` walk
the lockfile and gather every package's declared binaries and servers) and is
surfaced to no agent. **That gap is what `vibe tools` closes — see В3.**

**Sub-decision, ruled by the owner: how a package is recognised as an AI-Native
language.** Not by its group but by **its dependency on the discipline core**.
Reason: a third party can then publish its own AI-Native language in its own
group and be recognised.

**Measured cost, to be re-measured precisely before the build:** the package kind
is a closed list in code (`crates/vibe-core/src/package_ref.rs`, five values:
`flow | feat | stack | tool | mcp`). Adding a value is a cross-package ripple —
every exhaustive match over the list breaks, and only `--all-targets` finds them
all. Plus a migration: three language packages change their declaration, and with
them every reference in manifests, lockfiles, boot snippets and documentation.
Same shape as the root-coordinate migration but markedly smaller. **The standing
law applies: measure what is already built before building** — it stopped
nineteen builds of already-built things in three days.

**Note:** `kind` is metadata, not identity (identity is group/name/version/hash),
so changing it does **not** change any package's identity.

### В3. `vibe tools` — the tool registry {#v3}

**Owner's ruling (2026-08-06), near-verbatim:** let us make a registry of tools.
A command `vibe tools` will show what we have. For now only the AI-Native MCP
servers, later perhaps more. It looks like we can generate the list
algorithmically, without an LLM — when an LLM appears we will ask it for a
clarification and a human-readable description, the way `explain` does.

**Why this is the right answer to the gap В2 left:** an agent knows which
languages are installed (boot lane) but not what it can invoke. This is that
answer.

**The rails already exist:** `vibe-workspace`'s `collect_binaries` and
`collect_mcp_servers` walk the lockfile, read each installed package's manifest
and gather its declared `[[binary]]` and `[[mcp_server]]` entries. Consumers
today: `vibe mcp status` / `install`, and `vibe bin`. Real declarations in the
tree: **4** `[[mcp_server]]` across 4 manifests, **19** `[[binary]]` headers
across 8.

**Measured cost of an MCP surface, by example:** the simplest existing tool is
~43 lines in one file plus **one line** of registration; the dispatcher needs no
edit at all — the file's own doc says «a new tool is a new cell added here, not
an edit to the dispatcher». The cheapness is the wrapper only; the composite
query it would serve does not exist and must be written.

**This is the omnichannel norm's first consumer and should be built as its
dogfood:** roster logic in a shared crate, `vibe tools` as one surface, an MCP
tool as another.

**Prior art not to re-derive:** `crates/vibe-mcp/src/tools.rs` carries a comment
«Subsequent slices add `list_capabilities` … once `vibe-llm` is real» — a tool of
this shape was planned and never built.

### В4. The error-variant node — join at query time {#v4}

**Owner's ruling: variant (в) — build the tool and do not mix orthogonal
systems.**

**The situation, in plain terms:** there are two independent engines.
**`conform`** is the code-quality gate — it parses sources and produces facts
including which error variants exist, with what texts, citing which
requirements. **`specmap`** is the map of code↔spec links, and it cannot see
those facts. A third engine, **`progress`**, is about spec markup and campaign
verdicts and is unrelated to this question.

**Rejected: (а)** the map extracts the data itself — two engines extracting one
thing, two truths. **(б)** the map reads conform's data — a new dependency
between deliberately separate engines.

**Chosen: (в)** the data stays with its own engine and is joined only at query
time, by the tool that shows map and findings side by side. That tool is B-018
part 2 (see А5).

**Filed as** `BACKLOG.md` B-019 part (в). Parts (а) and (б) are built — (а) the
code fingerprint, (б) slice 1 of the command node, 2026-08-06.

### В5. A standing fact recorded during this conversation {#v5}

**The owner's note about `progress`, recorded because it changes design
decisions:** `progress` was built as a **temporary** means of finishing **this
refactor**, and not every project is obliged to track it. **If it becomes
permanent, its design needs a far deeper pass.** The boss had been reasoning
about it as a permanent part of the system without saying so; that assumption is
now named and is false by default.

---

## 4. Group А — the index {#a-items}

### А0. What the index actually is — read this before touching anything {#a0}

Measured 2026-08-06 (`cache/agents/sorted/M-INDEX/`). The owner's framing
«a microservice, or pre-generated data in the repository?» is a false dichotomy.

**The index is a set of files.** They go into a satellite repository and are
served by any static hosting — raw links, S3, plain nginx. No service required.

**On top of the same files** an optional live server (`vibe-index serve`) can
run; it holds the index in RAM and persists to those same files.

**In this repository no index file is committed at all** — it is assembled by a
separate command into a separate directory.

**The three layers:**

```
repomd.json          manifest of every other file, each with size and hash.
                     The entry point.
primary.jsonl        one JSON line per version of every package. Sorted, so it
                     diffs sanely in git.
by-name/<name>.json  "who publishes a package with this name" — the answer to a
                     short name.
```

Plus two inverted layers for capability and PURL search. A version record is
essentially that version's whole manifest — the index is a **precomputed digest
of manifests**, so nobody clones forty repositories to ask «what versions exist».

**The query path for `vibe install wal`:**

```
1. look in the project lockfile                 disk, no network
2. ask each registry's index                    ← FIRST network call:
                                                  plain HTTP GET
                                                  <base>/by-name/wal.json
                                                  a registry without an index
                                                  is skipped silently
3. two packages share the name?                 refuse to guess, list candidates
4. which versions exist                         index first; without one,
                                                falls back to `git ls-remote`
```

**The index address comes from exactly one place: the environment variable
`VIBEVM_INDEX_URL_<REGISTRY>`** (e.g. `VIBEVM_INDEX_URL_VIBESPECS`). **There is
no config field for it.** Absent variable ⇒ no index call at all, silently.

**Consequence:** a **fully qualified** install needs no index whatsoever — there
is a transparent fall-back to git. The index is required only for short names
and for search, because a git host cannot be enumerated cheaply.

**State: working, with exactly three stubs.** ~12 600 lines, **141 tests**, zero
`#[ignore]`, zero `todo!()`. Working and tested: init, reindex from local clones
(full and incremental), reindex from GitHub, all read verbs, add/remove, verify,
dump, and the whole server — reads, writes, bearer auth, rate limiting, metrics.

| stub | meaning |
|---|---|
| reindex **direct from GitVerse** | their public API cannot enumerate an organisation — a principled limit, not an omission. Workaround: a local mirror or a GitHub mirror |
| **auto-commit-push** of the index | the flag parses and is discarded — «parked until slice 9» |
| **stop the server on Windows** | prints the PID for a manual kill; no portable signal mechanism |

**A boss error corrected by the owner, recorded so it is not repeated:** the boss
stated «the primary registry lives on GitVerse» and it is **false**. The
project's own config names `vibespecs` on **GitHub** as the canonical publish
target — «the only host where `vibe registry publish` drives the create-repo and
push-tag flow end to end» — with the GitVerse registry second and read-only on
fall-through. The boss inferred «primary» from the fact that reindexing *from*
GitVerse is stubbed, and inverted the meaning. **Building an index from the
canonical registry is built and tested.**

### А1. Auto-publication — the blocker {#a1}

**Owner: this is a blocker and must be fixed urgently.**

**Requirement, in the owner's words:** the user must be able to trigger a
reindex manually, but **the server must be able to perform the republication
itself**. There is already a flag; make it work.

**Where it stands:** `--auto-commit-push` is declared, parsed, and discarded by
one line in `crates/vibe-index/src/cli/serve.rs` («parked until slice 9»). Every
other piece is built: accepting writes over HTTP with bearer auth, atomic file
writes, manifest recomputation, integrity verification. **The missing step is
committing and pushing the directory.** The format documentation says outright
that in v0 the operator commits and pushes the content themselves.

**Owner's ruling on the target: where the index publishes is the user's
setting, not a constant.** GitHub/`vibespecs` is canonical for the vibevm
project as a social phenomenon; for the index *service* it is merely one possible
target. **A private repository is a legitimate case.**

### А2. Private index — push AND authenticated reading {#a2}

**Owner's ruling:** yes, a private index needs authentication. Where ssh keys are
used — by keys, as usual. For other cases, something simple: **a token stored in
settings; check whether such a mechanism already exists.**

**The consequence the boss flagged and the owner accepted into scope:** a private
index breaks not only publishing but **reading**. A consumer today fetches
`<base>/by-name/<name>.json` with a plain unauthenticated HTTP GET. If the index
lives in a private repository that request is refused. **Whether the index client
can authenticate at all is UNMEASURED** — measure it first; it changes the work
from «add a push» to «add a push and authenticated reading».

### А3. The organisation cache, `rescan-org`, and the flag's real axis {#a3}

**The owner's proposal:** a flag (off by default) for the many-workers case where
the organisation must be enumerated every time; with a single index worker — the
default — keep a local in-memory image of the organisation state, re-reading it
only on an explicit `rescan-org` or at service start.

**What is already built (measured):** `reindex --incremental` exists, works and
is tested. It keeps a checkpoint (`<data-dir>/state/checkpoint.json`) recording
per package repository the last known head commit and tag list, and re-walks only
repositories whose state changed. Per-package verbs exist too: `add` inserts one
record from a package manifest, `remove` deletes a version or a package — both
built and tested. **For a future web UI, «one package changed» should call
`add`/`remove` directly rather than `reindex` at all.**

**The boss's objection, which the owner accepted:** the premise «between
operations nobody can change the organisation» is **already false today, and not
because of sibling workers**. A developer publishing a package creates a
repository and pushes a tag **directly to the git host**, never passing through
the index service. The image goes stale with one worker just as with ten.

```
A world where the index is the only door     The world we live in today
(the future web UI)                          (people push straight to git)
    │                                            │
    the in-memory image is AUTHORITATIVE         the image goes stale SILENTLY,
    because the same service writes it           and a stale index is worse than
                                                 a slow one: the package exists
                                                 and cannot be found
```

**Therefore, agreed shape:**

1. **Keep the cache.** Stop enumerating on every operation.
2. **The flag is `--cache-org`, on by default** — the owner's ruling of
   2026-08-06, which closes the name left open in §5. It is not `--cluster`:
   that name would promise protection from the wrong risk. The axis argued
   here — «I am the only writer to this organisation» versus «the organisation
   may change without me» — still stands, but the name landed on the mechanism
   rather than on the assumption, so what keeps the default honest is step (3),
   not the flag. Ruling and consequence: `BACKLOG.md` B-065.
3. **Instead of a full enumeration, a cheap freshness check** — git hosts answer
   «has anything changed» with a conditional request that costs almost nothing and
   needs no walk. Cache speed with honest checking.
4. **`rescan-org` as an explicit verb — unconditionally**, regardless of the rest.
5. **Webhooks are the real answer for the web-UI future** (see А4).

### А4. Webhooks, GitHub Actions, and a guide that lives in the specs {#a4}

**Owner's ruling:** plan webhook handling. It may be implementable on top of
GitHub Actions. **Plan writing a user guide for setting it up, and keep that
guide inside our specifications rather than in the documentation, changing it
together with the webhook properties.**

**Why it belongs in the specs and not in `docs/`:** the guide describes how to
configure a mechanism whose properties we define. When those properties change
the guide must change with them, and a guide living beside the contract changes
with it. A guide in `docs/` drifts — this session measured two independent
instances of exactly that (the index documentation against its code, and an
owner guide promising a panel step that does not exist).

**Why webhooks matter to А3:** with them the image is authoritative **because it
is fed**, rather than because we assumed nobody else writes.

### А5. Search over the map — both levels {#a5}

**Owner's ruling: build both.** Simple filters **and** a query language.

**The owner's reasoning, recorded because it determines the shape:** an agent
accustomed to grep and simple queries will use the simple filters; a query
language demands a complex form the agent will not build without need. It is like
grep — you can search simply, or you can search in a more complex way.

**Consequence for the build:** the simple level is **not** «version one to be
replaced later» — it is a permanent level. It must work on its own and must not
be a degenerate case of the query language.

- **Simple filters:** exact URI · substring of a symbol name · element-kind
  filter, combined with AND, plus a hard cap on the number of results. Nothing to
  parse, nothing to break, extends by adding fields.
- **Query language:** the above plus graph traversal (depth N, «has no edge of
  kind X»), which answers questions like «which rules does nothing verify» — and
  introduces a grammar that will need versioning.

**Context:** this is `BACKLOG.md` B-018 part 2, and it is the **only** open part
of that row — parts 1, 3 and 4 are built. The owner himself deferred part 2 on
2026-08-04 («put it in the backlog at medium priority»). **The row was in
neither of the two working lists**, which is how it went unnoticed; it belongs in
the owner-court list.

**It is also the tool В4 depends on** — the query-time join of map and conform
findings.

### А6. Generate the index's wire types {#a6}

**Owner's ruling: variant (а) — generate them.**

**What the measurement changed about the question.** It was «will the index lose
strictness if we generate it». The real finding: **strictness is the house style
for hand-written types — about 63 occurrences across host crates — and appears
zero times in generated output**. All seven existing wire contracts already
swallow unknown fields silently, and nobody had noticed. So there were two
policies for one class of type and nobody had chosen either.

**What survives generation and what does not:**

| | survives? | if not |
|---|---|---|
| field and type documentation | **yes**, works today (schema `metadata.description` becomes a doc comment) | — |
| helper methods on the type | no | a separate hand-written file in the same crate |
| spec links from the type | no | same |
| **strictness (reject unknown fields)** | **no** | nothing but teaching the generator |
| which crate the code lands in | wrong crate by default | **one line** — routing is by schema file stem and one exception already exists |

**Why the generator cannot do strictness, measured:** no key in any of our eight
schemas controls it; the format's own key for open/closed works the **opposite**
way (setting it *opens* the form); and that is validation semantics rather than a
promise about generated Rust. Of the three workarounds, a wrapper type does not
work (strictness is consumed where fields are declared), a separate `impl` file
**cannot work at all** (it is a container attribute the derive consumes at the
definition), and only post-processing the generator's output could.

**The substantive argument for generating anyway, which is why (а) is not a
surrender:** the index record is **read from a foreign registry**, possibly built
by a newer tool. Strictness there means a new field breaks old clients. For a
format that arrives from outside, permissiveness is forward compatibility.
**Softness becomes a deliberate choice rather than an accident.**

---

## 4b. Judging debt — asked, answered and landed on 2026-08-06 {#debt}

**The owner's question:** as we rewrite specs and build plans during the
refactor, does that show up in the progress state — all the facts accumulated
across many phases — or are we simply breaking everything?

**Answer, measured:** not breaking it. The machinery distinguishes three cases,
and it was built for the first. **But it announces only that one**, and the
other two accumulate silently.

**What landed the same day** (commit `95e25cbc`, PROP-043 §10.1 and §10.2):

- The **life of a fact under an active campaign** written into the mechanism's
  own contract: edited ⇒ comes due and is named; **added ⇒ unjudged and nothing
  says so**; **removed ⇒ its verdict stays and keeps counting**.
- **«Stale file» ≠ «a judged fact moved».** A file goes stale when facts are
  merely *added*, leaving every judged fact untouched — so a corpus can carry
  five stale files and owe zero re-judgements. Read the per-fact answer.
- **Sealing refuses a file with any unjudged marker**, and that refusal is
  today the only mechanism that makes an added fact visible at all.
- **The clearance procedure**: the debt is a list with names, not a ratio; the
  unit is one file (sealing is a whole-file assertion); the cheapest file is the
  one you were going to open anyway, because the reading is shared; judge, merge,
  seal, report; and never judge blind to move a number.
- **Content moved into a spec is judged in the same pass that moves it** —
  otherwise the owner's own closure ruling (§2 Б1) manufactures debt at every
  closure.
- **A session reports the debt when it restores context** — landed in the resume
  contract in all three instruction files (commit `b3a27b77`). Reporting is not
  paying; priority stays the owner's.
- **`campaigns/packages-2026-09/tasks/judging-debt.py`** — the measurement, one
  command, three counts with the files and anchors behind them. Its own docstring
  says it is a **stopgap**: the durable home is the shipped verb, recorded as
  `##DEBT-MUST-BE-ASKABLE`.

**The finding that made this a contract rather than a note.** The same five
orphan verdicts were measured, named and written down on **2026-07-28** in
`campaigns/packages-2026-09/PHASE-C-BATCH-PLAN.md` — `authority-line` and
`status-line` in two design documents, `related` in a third — together with the
mechanism that produces them («added by wave 1's own close-out after the file had
been judged»). They were still there, untouched, on **2026-08-06**. Nothing was
wrong with the analysis; it was filed **in a campaign zone, which this project's
own rules call disposable by design**, and it behaved exactly as that promises.
That is the owner's closure ruling seen from the other side.

**The debt as of 2026-08-06, and it is all this week's:** 47 unjudged facts
across 4 files (37 of them in `spec/design/command-nodes.md`, which is judged
nowhere yet), 5 orphaned verdicts across 3 files, 4 stale files. **0.4 % of
11 852 facts.** Reproduce with the script above.

**Not to be confused with the P1.** Two different debts, and merging them yields
the wrong conclusion «four thousand, hopeless»:

| | where it came from | how it is paid |
|---|---|---|
| **unjudged** (47) | we wrote new text | per file, at closing, cheap |
| **weakly judged** (4 151) | how judging was done earlier | §2 Б4's two columns; converts by itself as texts move |

## 5. What was NOT decided and must not be invented {#undecided}

- **What counts as «finished» for the text interface.** The owner ruled «finish
  the TUI subsystem» (in answer to the dead-code question: 39 of 55 suppressions
  sit there). The subsystem has no explicit boundary today, and without one the
  work has no end. **The boss owes the owner options for that boundary** — this
  is the boss's debt, not the owner's.
- **Whether the index client can authenticate** (see А2) — unmeasured.
- **What to do about claims inside fenced blocks** (see Б3) — named, not answered.
- ~~**The `--cluster` flag's final name**~~ — **decided by the owner on
  2026-08-06: `--cache-org`, on by default.** The ruling and its one
  consequence live in `BACKLOG.md` B-065; this line is a tombstone and goes
  with the plan.

## 6. Standing items untouched by this conversation {#standing}

Listed so the next session does not think they were resolved: the Phase E exit
gate (six corpus files stand at `work`); **B-050** (the custom-lint vehicle for
Rust needs a nightly toolchain pin — a ruling, not a build); **B-007** (do specs
owe ADRs, and in what form); **B-015** (parked by the owner until his notice);
**B-017**, **B-020**, **B-024** (the owner ruled «build» on 2026-08-01; not part
of this programme); `AUDIT.md` **-06/-07** (test organisations un-migrated on both
hosts), **-11**, **-13**, and the rider of 2026-06-12 (confirm a history rewrite
was intentional — its third run carrying it).

## 7. Requires the owner's hands, not a decision {#hands}

- **GitHub is unreachable from this machine** — ssh to `git@github.com` is
  redirected to `127.92.0.49`. Not a divergence; must not be forced.
  Diagnose with `ssh -vT git@github.com` and
  `git config --get-regexp 'url\..*\.insteadof'`.
