# Delivering the boot lane to coding agents — consolidated design

Status: **adopted**, 2026-11-10 — D1–D12 as decided and D13 authorised, narrowly
(§11). Companion to `vibevm/vibespecs/common/PROP-076-system-prompt-prefix.xml`
and `PLAN.md` in this directory; where the two disagree, PROP-076 governs and
this document is corrected. Nothing here was verified against a live model:
every check below either ran offline on this machine or is marked as research.

**What is implemented, as of phase P3 (2026-11-10).** The marker grammar of §2
and the agent lane of §1.1 are composed by
`crates/vibe-workspace/src/system_prompt/{lane,markers}.rs`, and the offline
check §2 promises — every begin has its end, pairs are unique, each hash matches
its own bytes, no marker-shaped line inside a body — is
`system_prompt::verify_lane_markers`. §3.3 is proved rather than asserted: the
composer refuses a managed-block marker or a lane marker that reaches the
prefix. Nothing is written yet (P4) and no command exists yet (P5, P6), so §8.4
below describes a surface that is designed and not built.

**Measured on this repository by P3**, superseding the research estimate in the
line below: the composed `AGENT-LANE.md` is **312,345 bytes** — 309,711 of
bodies across 9 segments plus 2,634 of marker lines — and it carries **74**
`@`-reference tokens, all outside a code fence, of which 68 come from
`STATIC.xml`. The composed prefix is **278,280 bytes**, 82.78% of the 336,174
bytes of boot material, with the lane 74.91% of it.

Evidence labels used throughout:

- **[V: command]** — verified on this machine (Linux, Claude Code 2.1.322,
  codex-cli 0.167.0) with the command shown. No model request was made.
- **[B]** — read from the installed binary's strings; shows the code exists, not
  how it behaves end to end.
- **[R]** — from the owner's earlier research; not verified here.
- **[R-nh]** — research about a harness that is not installed here (Qwen Code,
  Gemini CLI, OpenCode, pi, dsh); nothing about it could be checked.
- **[I]** — my inference; treat as a hypothesis for the live run.

Sizes today **[V: `wc -c`]**: `STATIC.xml` 251,817 bytes; the 8 unconditional
INDEX files 57,894 bytes; the lane is therefore about 310 KB. The entrypoint is
26,121 bytes, of which the managed block is 830.

## 0. The short version

1. There are three delivery tiers: **launch** (lane in the harness's system or
   developer slot), **import** (`@` in the instruction file), **read** (the
   router tells the agent to read the files).
2. Launch and read can share **one identical managed block** — the router. Its
   test is "are the lane markers in your context". Launched through vibe, the
   markers are there and the agent reads nothing; launched plain, they are not
   and the agent reads the lane. So a project in launch mode needs no second
   block shape and never contradicts itself.
3. Import is the odd one out. It is unconditional, it reaches quiet workers and
   nested projects, it needs import-inert lane bytes that we do not have (68
   `@spec://` tokens in `STATIC.xml`), and with byte-identical entrypoints it
   cannot be chosen per harness. I recommend shipping it last, opt-in, behind
   refusals — or not in version 1.
4. Inline delivery of a 310 KB lane on the command line is impossible on
   **Linux too**, not only Windows. Every launch mechanism must be file-based.
5. File-based slots exist and were checked for Claude Code (flag present, hidden
   from `--help`) and Codex (a 250 KB `developer_instructions` arrived intact
   through a config file, placed directly after the base instructions).
6. Lane segment markers carry four jobs: presence, truncation, duplication,
   staleness. The in-model part is guidance; the offline check and the live
   experiment are the parts that can be relied on.

## 1. The three tiers

| Tier | What carries the lane | Who decides at session time | Lane position |
|---|---|---|---|
| **launch** | A vibe-composed lane file handed to the harness's system/developer slot by `vibe agent <harness>` | vibe, before the first request | Before the entrypoint, in an instruction role |
| **import** | `@path` lines inside the managed block; the harness inlines the files | the harness, always | With the entrypoint, in whatever role the harness gives instruction files |
| **read** | The router; the agent reads the files with its tools | the model | After the entrypoint, as tool results |

A fourth mechanism exists and should be named so it is not adopted by accident:
**ambient config** — a project file the harness reads on every launch (Codex
`.codex/config.toml`, OpenCode `instructions`, pi `.pi/APPEND_SYSTEM.md`). It
lands in the slot like launch but is unconditional like import, so it reaches
quiet workers. See decision D3.

### 1.1 The agent lane file

Launch (and import, if offered) deliver one vibe-composed file, here called
`vibevm/vibespecs/boot/AGENT-LANE.md`: generated, covered by the same
vibe-written `.gitignore` as `SYSTEM.md`, never tracked.

Contents, in order: `STATIC.xml`, then every unconditional INDEX file in INDEX
order, each wrapped in a marker pair (section 2). No template, no entrypoint,
no router, no `@` import (rules 1 and 2). It is **not** `SYSTEM.md` minus a
segment: `SYSTEM.md` holds band P only, while the agent lane holds the lane part
of P plus band U.

`vibe agent` recomposes it at every launch from the files on disk, so band U
keeps its "current bytes" meaning from `#SYS-NO-FROZEN-INDEX`.

## 2. Lane segment markers

Owner addition, 2026-11-10. Each segment is wrapped:

```
<!-- vibe:lane-begin <node-id> <path> <hash12> -->
…segment bytes, verbatim…
<!-- vibe:lane-end <node-id> <path> <hash12> -->
```

- `<path>` is the project-relative path of the file, for example
  `vibevm/vibespecs/boot/STATIC.xml`. I use the path rather than a label such as
  `STATIC` because the truncation rule must tell the model which file to read,
  and the path is that answer.
- `<hash12>` is the first 12 hex digits of the SHA-256 of the segment bytes
  between the two lines.
- `<node-id>` is the selected node's identity. Without it, two projects both
  have a segment named `vibevm/vibespecs/boot/STATIC.xml`, and a nested
  project's block could match the outer project's lane.
- The lines contain no `@`, are HTML/XML comments, and read as labels. They are
  a pure function of the inputs, so they cost nothing in cache stability.
  `STATIC.xml` already opens with `<!-- vibe:c1 … -->` comment lines
  **[V: `head -c 300 STATIC.xml`]**, so the convention is not new.
- Markers exist only in the composed agent lane file. The files on disk are not
  changed; vibe does not own `90-user.xml`.

The four uses, and how far each can be trusted:

| Use | Signal | Action | Reliability |
|---|---|---|---|
| Presence | begin and end both in context | do not read that file | model behaviour |
| Truncation | begin without its end | read that file from disk | model behaviour; **offline-checkable for Codex** with `codex debug prompt-input` |
| Duplication | the same pair twice | follow one copy | model behaviour; the cost is already paid |
| Staleness | STATIC hash in the lane ≠ the hash written in the managed block | read the lane from disk | model compares two 12-character strings; plausible, unproven |

Only `STATIC.xml` gets an expected hash in the block. Live INDEX files get
markers but no expected value, so editing `90-user.xml` never rewrites
`AGENTS.md`.

Honest limits. The in-model checks are instructions to a model, not a
mechanism. A model can skip the comparison or misjudge a missing end marker in
300 KB of text. What can be relied on:

- `vibe prompt check` (offline): every begin has its end, pairs are unique,
  order matches INDEX, hashes match the bytes, the block's expected hash matches
  the current `STATIC.xml`, and no marker-shaped line occurs inside a segment's
  own bytes. **Implemented in P3** as `system_prompt::verify_lane_markers`, all
  of it but the order comparison, which only a caller holding the boot plan can
  make. The composer runs it over its own output, so the artifact it hands back
  is one the check has already accepted.

**One detail P3 had to settle, because the grammar is only usable if the hash is
recomputable from the delivered text alone.** The composer gives a body that
does not end with a newline one, so that the end marker is always a line of its
own; that byte sits *between* the markers and is therefore part of what the hash
covers. A checker reading delivered text hashes exactly the bytes between the
two lines and gets the number the lines carry — with no knowledge of what was on
disk.
- The live experiment (section 9): the agent is asked to quote the last
  `vibe:lane-end` line, which shows whether the tail arrived.

Where staleness really comes from **[I]**: the entrypoint is tracked and the
lane file is not. After `git pull` without `vibe install`, an alias made from
`vibe prompt flags` still points at the old lane file while `AGENTS.md` carries
the new hash. A second source is Claude Code's snapshot: a resumed conversation
resends the recorded system prompt "even when a later launch passes different
text, until the conversation is compacted" **[V: `claude --help`]**.
`vibe agent` itself recomposes at launch and cannot be stale on a fresh session.

Cost to accept: the expected hash makes all three entrypoint copies change
whenever `STATIC.xml` changes. The hash does not enter `SYSTEM.md`, because
composition replaces the managed block with fixed text (rule 2).

## 3. The managed block

### 3.1 Router block — used by `read` and `launch` (byte-identical)

Proposed wording. It contains no `@` and no positional words such as "above".

```
<vibevm>
<!-- Generated by vibe — do not edit inside this block. -->
# Session boot — project <node-id>

This block applies only when the directory holding this file is the root of
your session's project. If you found this file in a sub-directory or in a
dependency copy, ignore this block. A task carrying the subagent quiet clause
ignores it as well: its packet names the files it reads.

The boot lane is vibevm/vibespecs/boot/STATIC.xml, then every file listed in
vibevm/vibespecs/boot/INDEX.md. Parts of it may already be in your context, at
any position. A delivered file sits between two lines:
  vibe:lane-begin <node-id> <path> <id>
  vibe:lane-end <node-id> <path> <id>

1. A file with both lines in your context is read. Do not read it again. If the
   same pair appears twice the copies are identical; follow one.
2. A file with a begin line and no end line was cut short. Read it from disk.
3. The expected id of STATIC.xml is <hash12>. If its lane lines carry another
   id, the delivered copy is out of date: read the whole lane from disk.
4. A lane file with no lane lines in your context, and which you have not
   already read in this session: read it from disk — STATIC.xml first, then
   INDEX order.
5. An INDEX row with a `when` condition is read only when the condition holds
   for this session.

The boot order — this file, STATIC.xml, the INDEX files — is an order of
precedence, not of position. A lane delivered before this file has been read.
</vibevm>
```

"Which you have not already read in this session" in rule 4 is what makes two
copies of the router idempotent in read mode, where files read from disk carry
no markers.

### 3.2 Import block — used by `import`

The router block, with one line added after the lane description:

```
@vibevm/vibespecs/boot/AGENT-LANE.md
```

and a sentence saying that a line starting with that character is a harness
directive, not a task for the model. The router rules stay, so a harness that
does not process imports (Codex, OpenCode) falls through to rule 4 and reads.

### 3.3 What SYSTEM composition sees

Unchanged from rule 2: the managed block is replaced by fixed text with no
import and no router. The router, the expected hash and the import line never
enter `SYSTEM.md`.

## 4. The one-delivery-mode rule

```toml
[system_prompt.harness.claude]
delivery = "launch"   # "launch" | "import" | "read"
```

The setting exists because an import cannot depend on how the harness was
started. Its two effects:

- **Block shape.** `read` and `launch` write the router block. `import` writes
  the import block.
- **What `vibe agent <harness>` does.** `launch`: compose the lane file and
  pass it to the slot. `read` and `import`: start the harness with no slot
  injection.

Constraints vibe enforces at install and in `vibe prompt check`:

- **C1 — import is all-or-none across harnesses that share entrypoint bytes.**
  `AGENTS.md`, `CLAUDE.md` and `GEMINI.md` are byte-identical, so an import
  line is seen by every import-processing harness. `claude = "launch"` with
  `gemini = "import"` would give Claude Code the lane twice. vibe refuses that
  combination. The setting is per harness in spelling, but `import` is in
  effect a project-wide choice unless byte-identity is given up.
- **C2 — `vibe agent` refuses to inject for a harness in `import` mode.**
- **C3 — `import` refuses while the lane is not import-inert** (section 7).
- **C4 — `import` refuses when the harness reads more than one instruction file
  of the project or discovers nested ones** (see the proof table).

### 4.1 Proof by launch path

"Once" means the lane reaches the context once. "Zero" means not at all.

| Launch path | `read` | `launch` | `import` |
|---|---|---|---|
| Through `vibe agent` | No injection. Router rule 4: agent reads. **Once.** | Slot carries the lane with markers. Rule 1: nothing read. **Once.** | C2: no injection. Harness imports. **Once.** |
| Plain (`claude`, `codex`) | Same as above. **Once.** | No markers. Rule 4: agent reads. **Once**, as read mode. | Harness imports. **Once.** |
| Quiet-clause worker | Block says ignore. **Zero.** | Slot not passed to workers (Codex: no profile flag; Claude Code: main append not inherited by subagents **[I]**, a separate `--append-subagent-system-prompt-file` exists **[B]**). Block says ignore. **Zero.** | Any worker whose harness auto-loads the instruction file gets the lane. **Once — and that breaks `#SYS-WORKER-EXCLUSION`.** Only a worker started with instruction loading off escapes. |
| Nested project file discovered | Scope sentence: ignored. If the model ignores the sentence, the node-id in rule 1–3 does not match and rule 4 would read the nested lane: a model error, **not** a double delivery of the root lane. | Same. The slot holds only the session root's lane. | The harness inlines the nested project's lane regardless of the scope sentence. **Cross-project pollution.** C4 refuses; vibedeps copies must be neutralised. |
| Several instruction files read by one harness | Two identical routers. Rule 4's "not already read" makes the second a no-op. **Once.** | Two identical routers, same verdict from the same markers. **Once.** | Two import lines for the same file. Whether the harness de-duplicates is unknown **[R-nh]**. Rule 1 lets the model follow one copy but the tokens are paid twice. C4 refuses. |

No contradiction arises in `read` and `launch` because every copy of the block
is the same text evaluating the same test against the same context (rule 5 of
the owner's list: presence anywhere, order irrelevant).

The cross-mode case — `launch` for one harness, `import` for another — is the
only way to get a true double delivery, and C1 removes it.

Two residual weaknesses, stated plainly:

- The quiet-worker and nested-project rows for `read`/`launch` depend on the
  model obeying two sentences. Today the same dependence exists in the
  hand-written part of `CLAUDE.md`.
- For `import`, three of five rows need a refusal to stay safe.

## 5. Per-harness matrix

Lane position is described relative to the harness's own prompt. "Dynamic"
means per-machine or per-day content.

### Claude Code — launch and read available; import available with caveats

- **Launch mechanism:** `--append-system-prompt-file <file>`. Present in
  2.1.322 but **hidden from `--help`** (declared with `.hideHelp()`) **[B:
  `strings` on the binary; `claude --help` lists only `--append-system-prompt`,
  and names the file form once, under `--bare`]**. File-based, so no
  command-line limit.
- **Position:** appended after the default system prompt **[V: help text]**.
  After the dynamic sections unless
  `--exclude-dynamic-system-prompt-sections`, which moves "cwd, env info, memory
  paths, git status" into the first user message and is ignored with
  `--system-prompt` **[V: `claude --help`]**. `vibe agent claude` should pass
  it. `CLAUDE.md` and its imports land in first-message context **[R]**, so the
  lane precedes the entrypoint.
- **Cache:** `--system-prompt-snapshot on` is the default "except under
  `--bare` outside cloud and background sessions" **[V: `claude --help`]**. The
  tool list precedes the system prompt in the provider's cache hierarchy
  (`#SYS-CACHE-FACTS`), so a changed MCP tool set invalidates the lane's cache
  entry.
- **One file only:** the binary combines the file's text with an inline
  `--append-system-prompt` rather than choosing one, and rejects that pair in a
  Remote Control session **[B]**. A repeated `--append-system-prompt-file`
  presumably keeps the last **[R]**. `vibe agent` passes exactly one and
  refuses if the user's arguments contain another, or `--system-prompt[-file]`.
- **Size limit:** none known for the file flag **[I]**; verify with the real
  310 KB.
- **Import:** `@path` in `CLAUDE.md`, recursive, depth-limited **[R]**. Whether
  the 68 `@spec://…` tokens inside an imported `STATIC.xml` are attempted as
  imports is unknown.
- **Verify live:** position of the appended text; cross-session cache hit with
  the exclude flag; that subagents do not inherit it; behaviour of a repeated
  flag; 310 KB acceptance; resume after a lane change.

### Codex — launch and read available; no import

- **Launch mechanism:** `developer_instructions`. Three carriers were tested
  with a 250,421-character value built from the real `STATIC.xml`, using the
  offline renderer `codex debug prompt-input`:
  - Profile file `$CODEX_HOME/<name>.config.toml` selected with `-p <name>`:
    arrived intact, head and tail canaries present **[V — version-scoped, and
    the spelling matters]**.
    **Demoted from an unqualified [V] on 2026-11-10 (P6), and re-established
    with the correct spelling.** The key must be at the profile file's **top
    level**. `-p <name>` *layers* that file on top of the base user config —
    codex-cli's own help text for the flag — so inside a
    `[profiles.<name>]` table the key is an ordinary nested table the program
    loads and never reads. P5 wrote the table form and measured nothing
    arriving on codex-cli 0.167.0, which is what `LOG.md`'s falsification
    recorded; top-level on the same version, a 314,270-byte value arrives
    byte-equal and exactly once. Because the failure of this carrier is
    silent — no warning, no error, exit 0 — the claim is scoped to the
    version it was measured on, and `vibe agent codex` refuses an unverified
    one (`PROP-076#SYS-CODEX-PROFILE-TOP-LEVEL`). Verified version:
    **codex-cli 0.167.0**.
  - Project `.codex/config.toml` in a **trusted** project: arrived intact
    **[V]**. In an untrusted project it was silently not loaded, and
    `-c 'projects."<path>".trust_level="trusted"'` did not change that **[V]**.
    This carrier is ambient (decision D3).
  - Inline `-c developer_instructions=…`: works for a short value **[V]**;
    cannot carry the lane (section 6).
- **Position [V: `codex debug prompt-input`]:** item 0 tools; item 1 the base
  instructions (about 21 KB, developer role); item 2 a developer message whose
  **first part is `developer_instructions`**, followed by skills, permissions,
  collaboration mode, deferred tools, recommended plugins; then two more
  developer items; then a user message holding `AGENTS.md` and, after it,
  `<environment_context>` with cwd, shell, `current_date` and timezone. The lane
  therefore sits directly behind the base instructions and ahead of everything
  dynamic.
- **`AGENTS.md` limit:** a 40 KB project `AGENTS.md` was cut at 32 KiB with
  **no truncation notice**; the tail canary was absent **[V]**. The global
  `~/.codex/AGENTS.md` appeared in the same message and the observed sizes are
  consistent with it not being charged to the project's budget. `CLAUDE.md`
  is not read by Codex **[V]**.
  **Amended 2026-11-10 by two owner decisions, and both halves of this
  measurement changed meaning.** The 32 KiB is Codex's *default*
  `project_doc_max_bytes`, which a user's or a trusted project's config may
  raise, so a project now declares the limit it is built for
  (`[system_prompt.harness.codex] doc_max_bytes`,
  `PROP-076#SYS-CODEX-DECLARED-BUDGET`) and `vibe agent codex` passes it as
  `-c project_doc_max_bytes=<declared>`. And the managed block is no longer at
  the very end: it is at the **top** of each instruction file
  (`#SYS-BLOCK-AT-THE-TOP`), so a truncation costs authored prose rather than
  the boot router. This repository's entrypoint is 27,069 bytes with the block
  in its first ~2 KB. Re-measured offline with `codex debug prompt-input`
  (codex-cli 0.167.0) over a 43,271-byte `AGENTS.md`: the router arrives whole
  and the tail canary is dropped under the default limit; both arrive under
  `-c project_doc_max_bytes=262144`.
- **Do not use `model_instructions_file`:** it replaced the base instructions
  entirely in the rendered prompt **[V]**. P6 reconsidered it as a carrier —
  the file would hold Codex's own base instructions followed by the lane, so
  nothing is lost — and rejected it: correctness would then depend on a
  cached copy of another program's prompt staying correct across its
  upgrades, while the layered profile file loses nothing by construction.
- **Cache:** automatic prefix caching **[R]**. Session logs carry
  `cached_input_tokens` and `cache_write_input_tokens` in `token_count` events
  **[V: grep of a local rollout file]**. The string `prompt_cache_key` is in the
  binary but not in the sampled rollout **[V]**.
- **TOML carrier detail:** a `'''` literal block cannot contain `'''`; vibe must
  emit a basic string with escapes and `vibe prompt check` must compare the
  parsed value with the lane bytes.
- **Verify live:** only the cache numbers. Placement, truncation and trust
  gating are checkable offline — vibe can run `codex debug prompt-input` in its
  qualification suite when Codex is installed.

### Qwen Code — read only, unless a file-based slot is found **[R-nh]**

- `--append-system-prompt` takes text; no file variant is documented. With the
  inline limits of section 6, **launch is unavailable on every OS** as
  researched.
- `QWEN_SYSTEM_MD` and `--system-prompt` replace the base prompt; unsuitable.
- Import through hierarchical `QWEN.md` / `AGENTS.md` with recursive `@`
  processing: blocked by C3 and, since it reads two file names, by C4.
- **Verify live:** whether an append-from-file or settings key exists; whether
  both `QWEN.md` and `AGENTS.md` are loaded together.

### Gemini CLI — read; import possible only after the lane is import-inert **[R-nh]**

- No append slot known. `GEMINI_SYSTEM_MD` replaces the base prompt.
- Import through `GEMINI.md`, recursive. Discovers `GEMINI.md` in
  sub-directories; this repo tracks nested ones in `research/rust-demo`,
  `research/ts-demo`, one vibedeps copy and two vibepacks sources **[V:
  `git ls-files`]**. C4 applies.
- **Verify live:** import position and role, recursion on `@spec://`, nested
  discovery trigger, de-duplication.

### OpenCode — read; ambient config exists **[R-nh]**

- `instructions` array in `opencode.json`: file-based, re-read every message,
  ambient. The agent `prompt` key replaces the base prompt; unsuitable.
- `<env>` with today's date precedes the instructions, so the lane's cache
  entry dies at midnight whatever vibe does.
- No per-launch flag known, so there is no `launch` in the strict sense.

### pi — launch-like slot exists but is ambient **[R-nh]**

- `.pi/APPEND_SYSTEM.md` appends; `.pi/SYSTEM.md` replaces. Both are project
  files read on every start: ambient.
- `PI_CACHE_RETENTION=long` extends retention. Extensions may rewrite the
  system prompt each turn, which vibe cannot see.

### dsh — read **[R-nh]**

- The system prompt is a replaceable plugin (`core/system-prompt`); a vibe
  plugin would be a separate project.
- Known bug: the prompt doubles when `CLAUDE.md` and `AGENTS.md` are identical —
  which they are here by project rule. In `read` and `launch` this costs 26 KB
  twice and nothing more (two identical routers). In `import` it doubles the
  lane.

### Summary

| Harness | launch | import | read | Recommended default |
|---|---|---|---|---|
| Claude Code | yes, file flag (hidden) | yes, caveats | yes | `launch` |
| Codex | yes, profile file | no | yes | `launch` |
| Qwen Code | no file slot known | blocked by C3/C4 | yes | `read` |
| Gemini CLI | no | blocked by C3/C4 | yes | `read` |
| OpenCode | ambient only | no | yes | `read` |
| pi | ambient only | no | yes | `read` |
| dsh | no | no | yes | `read` |

## 6. Command-line length

- **Windows:** about 32,767 characters for the whole command line **[R]**.
- **Linux:** a single argument is capped at 128 KiB. A 200,000-byte argument
  failed with "Argument list too long"; 120,000 bytes passed
  **[V: `/bin/true "$(head -c 200000 /dev/zero | tr '\0' x)"`]**.
  `getconf ARG_MAX` reports 2,097,152, which is the total, not the per-argument
  limit.

So `$(cat lane)` and `-c key=<lane>` fail for a 310 KB lane on Linux as well.
The brief's premise that the inline route is a Windows-only problem is wrong at
today's lane size. Design rule: **vibe never puts lane bytes on a command
line; a harness without a file-based slot has no launch tier.**

## 7. Import-inertness

`STATIC.xml` contains 68 tokens of the form `@spec://…`
**[V: `grep -o` count]**; five of the eight INDEX files contain `@` tokens too.
Gemini CLI and Qwen Code process `@path` recursively inside imported content
**[R-nh]**; Claude Code does so to a depth limit **[R]**.

vibe composes the lane verbatim (`#SYS-LANE-VERBATIM`), so it cannot rewrite
these tokens. Therefore `import` stays refused (C3) until either the packages
stop emitting `@spec://` in boot text or a live run shows each harness leaves
such tokens alone. `vibe prompt check` reports the count and the first few
locations.

## 8. Design details

### 8.1 Ordering

PROP-009 states the session-start order as entrypoint, then `STATIC.xml`, then
INDEX. In `launch` the lane arrives first and in a stronger role.

Consequences:

- The current block's "read these files in order" becomes false as a
  description of time. The router's last paragraph replaces it: the order is
  one of precedence, and a lane delivered earlier counts as read.
- Lane text gains instruction-role authority that it does not have as a tool
  result. `#SYS-ROLE-PLACEMENT` already says this elevation is real and must be
  acknowledged. For this repo it matches intent: `CLAUDE.md` defers its commit
  rules to the static lane.
- Text that refers to position breaks silently. `STATIC.xml` itself says "Read
  this first, in full" in its header comment **[V]**; harmless, but
  `vibe prompt check` can list phrases such as "above", "below", "you have just
  read" in the entrypoint and lane for a human to review.
- Recommended PROP-009 amendment, one sentence: the boot order is a precedence
  order; a conforming delivery may place the lane before the entrypoint.

### 8.2 What goes into the slot

Lane only: `STATIC.xml` plus band U. Not the entrypoint (rule 1). Not the
template by default: it is written for an API runner with no base prompt, and
appended to a harness's own prompt it can contradict it ("You are …").

### 8.3 Bands U and G in agent mode

- **U:** inside the lane file, composed at launch from current bytes. A
  mid-session edit of `90-user.xml` is not seen until the next launch — the
  same as today after the agent has read it.
- **G under `host = "portable"`:** `os:` rows are not in the lane file. Router
  rule 5 has the agent read them. vibe does not detect the OS at launch even
  though it could; that keeps the file a pure function of declared inputs,
  matching `#SYS-HOST-AND-TARGET`.
- **G under a fixed host:** matching `os:` rows are baked into the lane file as
  further marked segments; rule 1 covers them. If the session's OS differs,
  `vibe agent` warns and continues, as `prepare` does.
- **`target:` rows** are resolved at generation and reach INDEX as
  unconditional rows or not at all. They are ordinary U segments.

### 8.4 Command surface

*Revised 2026-11-10 by the owner decision `PROP-076#SYS-HARNESS-EXECUTABLE`,
which governs. This section named the programs directly, and that conflated two
different things: the harness **kind** is the flag dialect and the delivery
rules, while the **launcher** is the name bound to a kind and to the argv that
starts it. A developer who reaches a harness through a personal wrapper — one
that sets the harness's own config directory, drops API-key variables and
`exec`s the real program with standing flags — needs the second without
changing the first.*

- **A launcher is a named thing, and only a user may name one.**
  `[agent.launchers.<name>]` in the **user configuration only**, with `kind`
  (required; the closed set `claude`, `codex`, …), `command` (the argv
  **prefix** to exec, so a wrapper may carry its own arguments) and an optional
  `config_dir`. A project manifest that names a launcher is a **refusal**: a
  repository must not choose which program runs on a contributor's machine.
- The built-in launchers `claude` and `codex` always exist, each with `kind`
  equal to its name and `command` the program of that name. A user launcher of
  the same name **overrides** the built-in, which is how a wrapper takes over
  the plain name. Any number of launchers may share a kind — two Claude Code
  launchers with different `config_dir`s are ordinary configuration.
- `vibe agent <launcher-name> [--] [args…]` — resolves the launcher **by
  name**, takes its flags and delivery from its **kind**, composes the lane
  file, checks the mode constraints, and execs `command` + flags + the user's
  own arguments. On Windows there is no `exec`: spawn, forward the console and
  signals, return the child's exit code. An unknown launcher name is a refusal
  that **lists the available launchers**; an unknown kind is a refusal naming
  the field.
  - Kind `claude`: `--append-system-prompt-file <abs path>` exactly once, plus
    `--exclude-dynamic-system-prompt-sections`. Refuses when the user's
    arguments already contain `--append-system-prompt-file`, `--system-prompt`
    or `--system-prompt-file`.
  - Kind `codex`: writes the profile file with `developer_instructions` at
    its **top level**, adds `-p <profile>` and
    `-c project_doc_max_bytes=<declared>`, and refuses an installed codex-cli
    version whose carrier this build has not verified.
  - A kind in `read` or `import` mode: starts it unchanged and says so in one
    line.
- `vibe agent list` — prints each launcher's name, kind, command, config
  directory and **source**, built-in or the exact user-config path, because a
  developer debugging which program ran needs to know which file decided it.
- `vibe prompt flags --harness <name> [--format argv|json]` — accepts either a
  kind or a launcher name, composes the file, and prints **flags only, never a
  program**, which is what lets its output be appended to any wrapper, alias or
  launcher a developer already has. This is the path that can go stale; the
  expected hash is its safety net.
- `vibe prompt check` / `vibe prompt prepare` — extended as in section 8.5.

`vibe agent` should be a thin layer over `flags`, so there is one code path.

**`config_dir` exists for one honest reason**: it names where that launcher's
harness keeps its session logs, which the cache diagnostics of §10 read. A
wrapper that sets `CLAUDE_CONFIG_DIR` internally makes the directory unknowable
from outside, so a launcher may state it; absent that, vibe takes it from the
environment, and absent that, the kind's documented default. **When it is still
unknown the diagnostics say so**, rather than guessing a path and reading
someone else's logs.

### 8.5 What `vibe prompt check` and `prepare` can warn about

- Markers: unpaired, duplicated, out of INDEX order, wrong hash, marker-shaped
  line inside a segment.
- The block's expected STATIC hash differs from `STATIC.xml`.
- The lane file is older than its inputs.
- `@` tokens in lane bytes (count and locations); refusal for `import`.
- Entrypoint size against the **declared** Codex limit
  (`[system_prompt.harness.codex] doc_max_bytes`, default 32 KiB), the
  authored bytes past it, and — now a formality, since the block leads the
  file — whether the managed block lies inside it. Implemented in P4 in
  `vibe check`'s `redirect_block` cell rather than here; what stays P5's is
  the `vibe prompt check` report around it.
- Mode conflicts C1–C4.
- Nested instruction files with a managed block under the project root, and
  un-neutralised blocks in vibedeps copies.
- Entrypoint copies that are not byte-identical.
- Session OS differs from a fixed `host`.
- Lane size in bytes and an estimated token count against a stated context
  size.
- A harness's installed version differs from the version last qualified, when
  the binary is on PATH (`--version` only).

## 9. Risks vibe cannot control

- **MCP and tool set.** Tools precede the system slot in the cache hierarchy.
  A server that connects late or changes its tool list invalidates the lane's
  entry.
- **Model switch.** A cache entry does not transfer between models; a
  mid-session switch pays the lane again.
- **Harness upgrades.** Claude Code's file flag is undocumented in `--help`;
  Codex's profile option is labelled `CONFIG_PROFILE_V2` **[V: `codex exec
  --help`]**. Either can change. The fallback is safe by construction: if the
  slot stops working, the markers are absent and the router reads.
- **Snapshot on resume.** A resumed Claude Code conversation keeps the lane it
  recorded. Only the model-side staleness rule covers this.
- **pi extensions** can rewrite the system prompt per turn.
- **OpenCode's date** precedes the instructions.
- **Context size.** A 310 KB lane in the slot cannot be dropped by compaction
  the way tool results can.
- **Project trust in Codex.** An untrusted project silently loses a project
  config file.
- **User-local additions.** `~/.codex/AGENTS.md`, `~/.claude/CLAUDE.md` and
  auto-memory enter the context outside vibe's accounting.

## 10. Live-verification plan

**Authorised, narrowly (D13, 2026-11-10): Claude Code only — rows C1–C8 — and
only once `vibe agent claude` exists, which is phase P6.** It runs **before the
release is published**, as the campaign's final step after the qualify phase
(`PLAN.md` P9), and it is run **by the central session**: a worker neither
starts an agent session nor spends the owner's subscription. The Codex rows
(X1, X2, X3), O1 and I1 stay **unauthorised**, so the rows for Codex, Qwen
Code, Gemini CLI, OpenCode, pi and dsh remain research and their delivery
default stays `read`. The one Codex row that needs no authorisation is **X0**,
which is offline and spends nothing. Nothing earlier in the campaign depends on
any of this, because every phase's own acceptance is offline.

**Sample project.** A small vibe project whose lane is padded to the real size,
with unique canary words: one in the last line of `STATIC.xml`, one in each
INDEX file, one in the entrypoint outside the block. The agent lane's last
`vibe:lane-end` line is the tail canary. One nested sub-project with its own
block and its own canaries.

**Where to read results.**

- Claude Code: `$CLAUDE_CONFIG_DIR/projects/<project>/<session>.jsonl` — on
  this machine the directory is `~/.claude2`, not `~/.claude` **[V]**. Fields
  `cache_creation_input_tokens`, `cache_read_input_tokens`,
  `ephemeral_1h_input_tokens`, `ephemeral_5m_input_tokens` **[V: grep of the
  current session file]**. Also `--output-format json` usage with `-p`.
- Codex: `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`, `token_count` events,
  `cached_input_tokens` **[V]**. Also `codex exec --json`.

**Steps and pass criteria.**

| # | Step | Pass |
|---|---|---|
| C1 | `vibe agent claude`, one prompt: "quote the last vibe:lane-end line; list files you read" | Tail canary quoted; no read of any lane file in the transcript |
| C2 | Same again, new session, within the cache lifetime | `cache_read_input_tokens` ≥ the lane's token count |
| C3 | Same from a second checkout path with a different git status | Still a cache read of the lane — proves the exclude flag keeps per-machine text out of the prefix. Fail means the flag is not doing what the help says |
| C4 | Plain `claude`, same prompt | Lane files read exactly once each; all canaries reported |
| C5 | `vibe agent claude`, ask it to start one subagent that reports whether it sees a lane marker | Subagent sees none |
| C6 | Pass the file flag twice with different files | Record which one arrives; fixes the "last wins" claim |
| C7 | Change `STATIC.xml`, reinstall, resume the earlier session | Record whether the old lane is resent and whether the agent notices the hash mismatch |
| C8 | Windows: C1 with the full-size file | Same as C1 |
| X0 | Offline, no spend: `codex debug prompt-input` through the profile | Lane is part 0 of developer item 2, both canaries present |
| X1 | `vibe agent codex` twice, `exec --json` | Second run's `cached_input_tokens` ≥ base + lane |
| X2 | Plain `codex` | Lane read once through the router |
| X3 | Entrypoint padded past the declared limit, block at the **top** | **DONE offline 2026-11-10** with `codex debug prompt-input`: router present, tail canary absent at the default limit; both present under `-c project_doc_max_bytes=262144`. The original form of this scenario (block at the end) is what motivated moving the block, and is retired |
| N1 | From the root, make the agent open a file in the nested project | Nested lane not read; root lane not re-read |
| O1 | Qwen, Gemini CLI, OpenCode, pi, dsh in `read` mode | Lane read once; for dsh, record the doubled entrypoint |
| I1 | Only if import is pursued: Claude Code and Gemini CLI with the import block | Lane present once; no attempt to resolve `@spec://`; position and role recorded |

A failure of C1, C2 or X1 means `launch` for that harness defaults back to
`read`. A failure of C5 means `launch` breaks worker exclusion on Claude Code
and must not be the default.

## 11. Owner decisions

**All thirteen are decided** (2026-11-10): D1–D12 adopted exactly as
recommended, D13 authorised with the scope its row states. The third column is
kept as the recommendation that was put, so a later reader can see what was
chosen over what.

| # | Decision | Recommended default — **decided as recommended unless the row says otherwise** |
|---|---|---|
| D1 | Setting shape and defaults | `[system_prompt.harness.<name>] delivery`; `launch` for `claude` and `codex`, `read` for the rest. Safe before live verification because a plain launch in `launch` mode is read mode |
| D2 | Offer `import` in version 1? | No. Reserve the value; refuse it with a message naming C1, C3, C4 and worker exclusion. Revisit after I1 |
| D3 | Use ambient config carriers (Codex project config, OpenCode `instructions`, pi `APPEND_SYSTEM.md`)? | No. They reach quiet workers. For Codex use the profile file |
| D4 | May `vibe agent codex` write `$CODEX_HOME/vibe-<node-id>.config.toml` — a file outside the project? | Yes, written only by `vibe agent` / `vibe prompt flags`, never by install. The alternative is no launch tier for Codex |
| D5 | Marker grammar: node-id included, path as the segment name, 12 hex digits | As in section 2 |
| D6 | Expected STATIC hash in the managed block, accepting that the three entrypoint copies change with every `STATIC.xml` change | Yes |
| D7 | Template in the agent slot | No; lane only |
| D8 | `--exclude-dynamic-system-prompt-sections` passed by `vibe agent claude` | Yes |
| D9 | OS detection by `vibe agent` to bake `os:` rows under `host = "portable"` | No; leave them to router rule 5 |
| D10 | Both `vibe agent` and `vibe prompt flags` | Both; `agent` built on `flags` |
| D11 | Entrypoint over Codex's 32 KiB: warning or refusal | Refusal when the managed block would fall outside the limit; warning within 4 KB of it. **Amended 2026-11-10:** the limit is the **declared** one (`SP-FMT-107`) rather than a constant, and the block moved to the top of the file (`SP-FMT-108`), so the refusal is now a guard reachable only by declaring a limit smaller than the block itself and the warning that matters counts the authored bytes past the limit |
| D12 | PROP-009 sentence: boot order is precedence, not position | Adopt. **Done 2026-11-10:** `#SESSION-START-ORDER` carries it |
| D13 | Authorise and budget the live run of section 10 | **AUTHORISED 2026-11-10, scoped: Claude Code only (C1–C8), before the release is published, only once `vibe agent claude` exists (P6), run by the central session as phase P9. Codex, Qwen Code, Gemini CLI, OpenCode, pi and dsh stay unauthorised; offline X0 needs no authorisation.** The recommendation was "after P7; C1–C5 and X1–X2 first" |

## 12. Placement in the campaign phases

| Phase | Addition |
|---|---|
| ~~P2 capture~~ → **P2b** | The router block as the prospective entrypoint's managed block; expected STATIC hash; block neutralisation in vibedeps copies. *Re-planned: these three arrived after P2's scope was fixed, they are not capture, and each rewrites the body of every instruction file in every project — so they are their own atom, placed after P3 where the marker and hash grammar is settled* |
| **P3 compose — DONE 2026-11-10** | Agent lane composition and markers, beside the `SYSTEM.md` composer; fixed-host baking shared with it (`ContinuationPlan::resolve_host`, one function both composers take their rows from) |
| P4 publish | `AGENT-LANE.md` as a generated, ignored production role |
| P5 operations | `vibe prompt check` / `prepare` additions of section 8.5; `[system_prompt.harness.*]` parsing and C1–C4 |
| P6 adapters | `vibe prompt flags`, `vibe agent` with the named launchers of §8.4, the Claude Code and Codex adapters |
| P7 qualify | Offline marker and block tests; `codex debug prompt-input` placement test when Codex is installed; nested-project fixtures |
| P8 docs | The delivery modes, the limits table, the risks of section 9 |
| **P9**, authorised and scoped | Section 10, Claude Code only, by the central session, before the release is published |

## 13. Where the earlier research was wrong or incomplete

1. **Inline delivery fails on Linux too.** One argument is capped at 128 KiB;
   the lane is 310 KB.
2. **`--append-system-prompt-file` is hidden.** It exists in 2.1.322 but
   `claude --help` does not list it as an option. Treat it as less stable than
   a documented flag.
3. **"Last flag wins" is incomplete.** The binary combines the file with an
   inline `--append-system-prompt`. Which of two file flags wins is untested.
4. **Snapshot default has an exception** (`--bare` outside cloud and background
   sessions), and the snapshot survives a resume with different text. The
   second point creates a staleness case the research did not mention.
5. **Codex placement is better than described.** `developer_instructions` is
   not merely "before AGENTS.md": it is the first thing after the base
   instructions, ahead of skills, permissions, tools and plugins.
6. **Codex has file-based carriers** the research did not list: the `-p`
   profile file and the trusted-project config file. Both carried 250 KB.
7. **Codex truncates `AGENTS.md` silently**, with no notice in the prompt.
8. **Codex also injects a date** (`current_date`, timezone), but after
   `AGENTS.md`, so it does not disturb the lane.
9. **Codex has an offline prompt renderer**, `codex debug prompt-input`. Most
   Codex placement questions need no subscription spend.
10. **Claude Code logs are under `$CLAUDE_CONFIG_DIR`**, which here is
    `~/.claude2`; fractality runs use a fresh directory per run.
11. **`prompt_cache_key`** is in the Codex binary but absent from the sampled
    session log; cache evidence should come from `cached_input_tokens`.

Not contradicted, and not confirmable here: everything about Qwen Code, Gemini
CLI, OpenCode, pi and dsh.
