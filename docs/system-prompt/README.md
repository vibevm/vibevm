# The cache-stable system prompt — a guide

**For whom:** a developer who wants their project's standing instructions to
reach a coding agent, or an API caller, without the agent re-reading the boot
lane every session — and who wants to know exactly what that does and does
not buy.

**How verified:** every command below runs offline on this repository's own
test suites; the whole scenario is
[`campaigns/sys-prompt-2026-11/accept.sh`](../../campaigns/sys-prompt-2026-11/accept.sh),
which runs in a network-isolated sandbox with every `vibe` call under
`strace -e trace=connect`. Claims that were **not** measured are marked
**“not measured here”** with the reason. No number below comes from a
provider's bill. One exception, and it says so wherever it appears: the Claude
Code delivery and cache rows were measured against a live model on 2026-11-10
(Claude Code 2.1.322, Linux, print mode) — the record is
[`campaigns/sys-prompt-2026-11/live/RESULTS.md`](../../campaigns/sys-prompt-2026-11/live/RESULTS.md),
and the scope of each number travels with it.

Design record: `spec://org.vibevm.core/vibevm/common/PROP-076#root`.

---

## 1. What it is

A vibevm project's boot is a lot of text. Measured on this repository
(2026-11-10): the compiled lane `vibevm/vibespecs/boot/STATIC.xml` is 251,817
bytes, the entrypoint `AGENTS.md` 27,069, and the eight files `INDEX.md` names
57,894 together — **336,780 bytes** of boot material, of which the lane alone
is **74.8%** and the lane plus the entrypoint **82.8%**.

Most of that changes only when packages, authored boot sources or your own
frame change. So `vibe` generates it once, as bytes that are a pure function
of their inputs:

| Artifact | What it holds | Who reads it |
|---|---|---|
| `vibevm/vibespecs/boot/SYSTEM.md` | your frame, then the entrypoint with its managed block replaced by the transport contract, then the compiled lane | an API caller, in an instruction role |
| `vibevm/vibespecs/boot/AGENT-LANE.md` | the compiled lane, then every unconditional `INDEX.md` file, each wrapped in a marker pair | a coding agent's own system/developer slot |
| `vibevm/vibespecs/boot/SYSTEM.meta.json` | the input witnesses, the segment map and the continuation plan | `vibe`, for freshness; never sent to a model |

That 82.8% is the prefix, so on this repository it is **about 278 KB** — the
exact figure depends on your own frame, which is an input. The agent lane,
composed from the same tree, is **312,345 bytes**: 309,711 of file bodies
across nine segments plus 2,634 of marker lines.

**The agent lane is not the prefix minus a segment.** `SYSTEM.md` is for a
runner with no base prompt of its own, so it carries your frame and the
entrypoint. The agent lane goes into a harness that has already loaded the
entrypoint itself, so it carries neither — and it carries the live `INDEX.md`
files, which the prefix deliberately does not.

**The live part stays live.** No `INDEX.md` row's bytes are ever frozen into
`SYSTEM.md`, however unconditional the row. The sidecar holds a *plan* — the
rows' identity, order and conditions — that a runner executes per request by
reading the current files, and `vibe agent` recomposes the agent lane at every
launch. Editing `vibevm/vibespecs/boot/90-user.xml` therefore leaves the
prefix current and costs no rebuild.

**`SYSTEM.md` alone is not a session.** A caller that sends it and nothing
else gets an explicitly incomplete boot. Obligations your entrypoint states
beyond reading files — resolve a local context, read a plan, consult a
campaign document — are not discharged by generation and are not reported as
discharged.

## 2. Opting in

Write one file:

```
vibevm/vibespecs/boot/SYSTEM-TEMPLATE.md
```

Its presence is the entire opt-in. A copyable starting point with the rules
in it is [`SYSTEM-TEMPLATE.example.md`](SYSTEM-TEMPLATE.example.md).

- **No template** → no generation and no change whatsoever to your boot.
  `vibe install` behaves byte-for-byte as it does today. That is what makes
  this feature free for a project that does not want it.
- **An empty template** → opt in with no frame of your own; the prefix is
  the entrypoint export plus the lane.
- **`vibe` never creates, overwrites or deletes your template.**
- **Opting out** is deleting it. The next install removes exactly the three
  generated artifacts and the ignore file it wrote, and touches nothing else
  in that directory.

The template is **text, not a program** — see the example file for the full
list of what has no effect.

### The artifacts are not committed

`vibe` writes `vibevm/vibespecs/boot/.gitignore` covering the three
artifacts and itself. The repository root's `.gitignore` is never touched —
that file is yours.

The prefix is a deterministic function of committed inputs, so a committed
copy would duplicate ~280 KB the repository already holds, in a file no
human can usefully merge, and would let a stale copy be committed and then
believed. `vibe check` reports a **git-tracked** generated prefix as an
error, naming the path and `git rm --cached`.

`STATIC.*` and `INDEX.md`, two generated files in the very same directory,
keep the opposite rule for the opposite reason: a session whose only tool is
a file reader must be able to boot from a fresh clone. Every consumer of a
*prefix* is a program already running `vibe`.

## 3. `[system_prompt]` — the manifest table

Everything that is not text goes in the node's own `vibe.toml`. Every member
is optional, and omitted when absent, so an existing manifest serialises
byte-identically.

```toml
[system_prompt]
entrypoint = "AGENTS.md"   # or "CLAUDE.md", "GEMINI.md", or false
lane_form  = "verbatim"    # the only accepted value in this version
recipe     = 1             # pins the transport-contract text
host       = "portable"    # or "linux" | "windows" | "macos"
target     = ["linux", "windows"]   # default: every operating system

[system_prompt.harness.claude]
delivery = "launch"        # "launch" | "read"  ("import" is refused, §6)

[system_prompt.harness.codex]
delivery = "launch"
doc_max_bytes = 262144     # the Codex limit this project is built for
```

A bad value fails **deserialisation**, naming the field as a TOML path — it
never silently defaults. `lane_form = "projected"` refuses as reserved
rather than behaving as verbatim; an `entrypoint` outside the closed set
refuses while naming all three legal spellings; `target = []` refuses,
because an empty selection and an absent key mean opposite things.
`[system_prompt]` with no `SYSTEM-TEMPLATE.md` beside it is a refusal too.

### `host` and `target` are two different questions

They cannot be one key, and the case that proves it is ordinary: a Linux
workstation building Windows software.

- **`host` is where the agent runs.** It governs the existing
  `when = "os:<name>"` gate, which has always meant the session's own
  operating system. `"portable"` (the default) keeps such a row in the gated
  continuation band, evaluated per session against an *explicitly supplied*
  target. A **fixed** host resolves them at build time — the matching row is
  baked into the artifacts, the others are dropped — so the gated band is
  empty and `vibe prompt prepare` needs no `--session-target`.
- **`target` is what the product is for.** It selects
  `when = "target:<os>"` rows, which are a property of the project and never
  of a session, so they are **always** resolved at build time and never reach
  a gated band. An absent `target` selects every operating system, so adding
  the spelling changed no existing project's boot — on this repository, not
  one byte of `INDEX.md`.

A baked host OS is not “environment detection”: the name is read from your
own manifest, exactly like `entrypoint`. Nothing about the generating machine
is observed.

**A host mismatch warns and continues.** When the operating system you are
running on differs from a fixed `host`, you get one line naming both and
saying what the prefix carries —

```
note: `[system_prompt] host` declares `windows` and this session runs on `linux`, so the prefix bakes in the `os:windows` continuation rows and carries none for `linux`; it is served as declared rather than refused
```

— from `vibe prompt check` and from `vibe prompt prepare`, in the human
report and in the `warnings[]` of both `--json` documents, and from
`vibe agent` as a stderr note before your harness starts. **Nothing
refuses**, and the exit status is untouched: `check` is still `equal` and
`prepare` still hands back every band. Refusing would leave a
cross-platform worker unable to read its own project's instructions.

Three cases are silent, each for its own reason. `host = "portable"` and an
absent `host` are the same value and carry no host decision, so there is
nothing to differ from — a project that sets neither key never sees this
line. An OS equal to the declared one is the expected state. And a platform
this build does not name says nothing at all, for the same reason an unknown
session gate never becomes `true`. On `vibe prompt prepare`, a
`--session-target os=<name>` you supply **is** your statement of the
session's OS and is what the line is measured against; it still decides the
gated band on its own.

`vibe check` does **not** carry this finding. It reports what is wrong with
the project, and which machine you are sitting at is not a property your
collaborators share.

## 4. The commands

| Command | What it does |
|---|---|
| `vibe prompt build [--node <rel>]` | regenerate the three artifacts through the one boot transaction, and print one summary line |
| `vibe prompt check [--node <rel>] [--json] [--model-window <bytes>]` | compare the current inputs with the published artifacts; read-only |
| `vibe prompt prepare --json [--session-target os=<name>]` | one request's materials: bands P, U and G **separately**, with observed digests and the recommended cache breakpoints |
| `vibe prompt capture [--json] [--with-text] [--task <text>] [--usage <file>]` | the request envelope an API runner sends: which band in which role, in what order |
| `vibe prompt path [--part lane\|system] [--json]` | the absolute path of a generated artifact |
| `vibe prompt flags --harness <kind-or-launcher>` | ready-to-paste launch flags — flags only, never a program |
| `vibe agent <launcher> [--] [args…]` | compose the lane, check the mode constraints, and start the harness with it |

Generation also happens inside `vibe install`, `vibe reinstall` and the
format-change and staged-publication paths, for every node that has opted
in. Nothing is generated at session boot: boot stays pure file reading.

Every build ends with one line naming what happened and where —
`system prompt: rebuilt → <path> (<size>), lane → <path> (<size>)` or
`system prompt: unchanged → <path>` — and a project that has not opted in
says so rather than staying silent.

**`vibe check` carries the verdict too, under the `system_prompt` check id.**
You do not have to know to run `vibe prompt check`: a stale, missing or
unverifiable prefix shows up in the ordinary report, with the exact
`vibe prompt build` that resolves it. A project with no authored
`SYSTEM-TEMPLATE.md` and no leftover artifact is **not reported at all** —
the feature stays invisible to a project that does not want it. Severity is
a **warning**, because a prefix is fully derivable from committed inputs by
one command and nothing but a program already running `vibe` reads it; a
caller who needs a hard gate uses `vibe prompt check`'s non-zero exit. The
one **error** is a generated artifact git tracks, which is the single state
the no-tracking rule exists to prevent.

### `check` and `prepare` are effect-free

Neither writes a product, cache, profile or config file; neither opens a
network connection; and neither hands control to an effectful compile
provider to recompute freshness. `check` reaches no analysis entrypoint at
all: it compares the sidecar's witnesses against the **committed** `STATIC.*`
and the digest of the committed `INDEX.md`. Measured 50–80 ms warm on this
repository. (Whether that committed lane is itself behind its own sources is
`vibe install`'s question, not the prefix's.)

One effect remains, and is stated rather than hidden: the
“is this artifact git-tracked?” question is asked by spawning `git ls-files`,
only where a `.git` entry exists, and “cannot tell” is reported as unknown
rather than as clean.

`prepare` **refuses** a stale artifact with the exact `vibe prompt build`
that fixes it, rather than rebuilding one — a command that promises no writes
does not write. `vibe prompt check` exits non-zero on any verdict but
`equal`, so a wrapper cannot read a stale prefix as current.

### `vibe agent` and named launchers

A harness **kind** (`claude`, `codex`) is the flag dialect and the delivery
rules. A **launcher** is a name bound to a kind and to the argv that starts
it, and it lives in your **user** configuration only —
`<settings>/agents.toml`:

```toml
[launchers.claude2]
kind = "claude"                  # the closed set of kinds
command = ["claude2"]            # an argv *prefix*, so a wrapper may carry its own flags
config_dir = "~/.claude2"        # optional: where that harness keeps its session logs
harness_version = "0.168.0"      # optional: a version you verified yourself
```

`vibe agent claude2 [args…]` resolves the launcher by name, takes its flags
from its kind, and execs `command` + flags + your own arguments. The
built-ins `claude` and `codex` always exist, and a user launcher of the same
name **overrides** the built-in — which is how a personal wrapper takes over
the plain name. Any number of launchers may share a kind: two Claude Code
launchers with different `config_dir`s are ordinary configuration.
`vibe agent list` prints each one's name, kind, command, config directory and
**source** — built-in, or the exact user-config path.

**A project manifest that names a launcher is a refusal.** A repository must
not choose which program runs on a contributor's machine.

`config_dir` exists for one reason: a wrapper that sets `CLAUDE_CONFIG_DIR`
internally makes the log directory unknowable from outside, and the cache
diagnostics read those logs. Absent it, `vibe` takes the directory from the
environment, then the kind's default; when it is still unknown the
diagnostics say so rather than guessing a path and reading someone else's
logs.

**A project that has not opted in still launches.** With no
`SYSTEM-TEMPLATE.md` there is no lane, so `vibe agent <launcher>` passes no
slot flag at all: it prints one note saying the project has not opted in and
that the harness starts unchanged, then starts it with your own arguments —
and the agent boots the classic way, reading the files its instruction file's
block names. A slot flag of your own is then no conflict, because there is
nothing of ours beside it. `vibe prompt flags` in such a project is still a
refusal: there is nothing to print.

### Every agent launches softly

**`vibe agent` never leaves you without a session because *vibe* could not
deliver the lane.** A launcher's job is to start a program. When safe
delivery is impossible the harness starts with **no** slot flag and **no**
carrier — the classic boot, which every project supports because the router
reads the files when it finds no markers — behind one loud warning naming
the reason and the exact command that fixes it:

```
warning: vibe cannot deliver this project's boot lane to `codex`, so the harness starts
WITHOUT it and the agent reads its boot files from disk — the classic boot the managed
block's router already handles.
warning:   reason: this machine's Codex is 0.168.0 and vibe verified the launch carrier on 0.167.0 only. …
warning:   fix: check this version offline with `codex debug prompt-input`, … and then say so with `[launchers.codex]`'s `harness_version = "0.168.0"`; …
warning: `--quiet` does not suppress this: it is about what the agent receives, not about what vibe says.
```

| What happened | What `vibe agent <launcher>` does |
|---|---|
| The prefix build failed | soft launch; the older lane on disk is **not** passed |
| The lane file is absent after that build | soft launch |
| The lane is **stale** after that build | soft launch — the bytes on disk are not this node's current lane |
| This directory is not a vibe project | soft launch |
| Your Codex is a version vibe has not verified | soft launch |
| `harness_version` is not a version number | soft launch |
| Any kind's carrier precondition fails | soft launch |
| The project has not opted in | starts unchanged, with a **note** — nothing to deliver is not a failure |
| `delivery = "read"` for this kind | starts unchanged, with a **note** — the router is doing the work by design |
| A project manifest names a launcher | **refusal** — a repository must not choose which program runs on your machine |
| An unknown launcher name | **refusal** — vibe does not know which program to start |
| Your own slot flag, while vibe is delivering | **refusal** — two system prompts in one slot is a session nobody chose |

**`--quiet` does not suppress the warning.** `--quiet` asks for less
narration about what vibe did; this sentence is about what the **agent
receives** — a session without its boot lane, which you are about to spend
tokens in.

**A stale or half-written lane is never delivered.** If the build failed,
the file on disk is the previous build's or half of this one's. Handing that
to the agent is worse than handing it nothing, because the router cannot
notice a lane it was given.

**`vibe prompt flags` keeps refusing.** It prints flags and starts nothing,
so there is no session to save, and flags pointing at a lane that is not
there are the one answer a shell alias turns silently into a session with no
lane in it. Its one exception is a **stale** lane: it prints the flags
beside a note saying so, because they point at real bytes and you can
decide. A launch cannot.

**A resumed Claude Code session keeps the lane it started with.** Claude Code
resends the system prompt a conversation began with, so boot files edited since
do not arrive, and the router cannot notice — the block states the expected id
of `STATIC.*` alone, and a changed `INDEX.md` file leaves that id untouched.
`vibe agent` says so when your arguments resume a session (`-c`, `--continue`,
`-r`, `--resume`). **A new session is the remedy**, and the only one.

## 5. Delivering it to a coding agent

Three tiers, one mode per harness, set by
`[system_prompt.harness.<name>] delivery`:

| Tier | What carries the lane | Lane position |
|---|---|---|
| **launch** | a vibe-composed lane file handed to the harness's own system/developer slot by `vibe agent <launcher>` | before the entrypoint, in an instruction role |
| **read** | the router block in your instruction file tells the agent to read the files with its tools | after the entrypoint, as tool results |
| **import** | an `@path` line inside the managed block — **refused in this version** (§6) |  |

Default: `launch` for `claude` and `codex`, `read` for every other harness.

**`read` and `launch` share one byte-identical block.** The router's test is
“are the lane markers anywhere in your context”, never “above”. Launched
through `vibe agent` they are there and the agent reads nothing; launched
plain they are absent and the agent reads the lane. So a project in `launch`
mode cannot contradict itself, and `launch` is a safe default before any live
measurement.

**No lane bytes ever go on a command line**, on any operating system: one
argument is capped at 128 KiB on Linux and the whole line at about 32,767
characters on Windows, against a ~310 KB lane. A harness with no file-based
slot therefore has no `launch` tier.

**What vibe enforces around the setting.** `vibe agent` injects **only** for
a harness in `launch`: in `read` it passes no slot flag, writes no Codex
carrier, does not gate on the harness's version and does not refuse a slot
flag of your own — there is nothing for yours to collide with. And in
`launch`, a delivery that cannot be made safely becomes the soft launch
above rather than a refusal. `vibe prompt
check` reports two configuration findings: your `CLAUDE.md`, `AGENTS.md` and
`GEMINI.md` carrying **different** managed blocks, because a *per-harness*
`delivery` only describes what a harness receives while those files are
byte-identical; and the same divergence seen from the harness side, since
Gemini CLI and Qwen Code read several of those names and concatenate them, so
one session would get two contradicting routers. Identical copies are fine
and cost only their own bytes. A managed block **below** your project root
that a harness's downward discovery would also read is `vibe check`'s
`redirect_block` warning.

| Harness | launch | read | recommended default |
|---|---|---|---|
| Claude Code | yes — `--append-system-prompt-file`, plus `--exclude-dynamic-system-prompt-sections` | yes | `launch` |
| Codex | yes — `developer_instructions` at the **top level** of a launch-time profile file | yes | `launch` |
| Qwen Code, Gemini CLI, OpenCode, pi, dsh | no file-based slot known | yes | `read` |

The non-Claude, non-Codex rows are **research, not measured here**: those
harnesses are not installed on the machine this was written on, so their
delivery stays `read`.

**The Claude Code row is measured.** On 2026-11-10, against Claude Code
**2.1.322** on Linux in print mode: a 2,570-byte lane, a 202,349-byte one and
this repository's own **312,345-byte** lane each reached the slot whole — the
agent quoted the last `vibe:lane-end` line byte-equal, reported every canary
including the one on the lane's last line, and **read no lane file with a
tool**. No size limit was found up to 312,345 bytes. Classic mode was measured
in the same run: with the template removed, the agent read `STATIC.*` first and
then the `INDEX.md` files in order, **once each**. A subagent of such a session
sees none of it. Two things that are **not** measured: Windows, and the
interactive terminal UI — an agent cannot drive it, so every session was
`-p … --output-format json`, which passes the same flag.

### Codex and `doc_max_bytes`

Codex truncates a project instruction file at `project_doc_max_bytes`
(default 32 KiB) **with no notice in the prompt**. Two things follow.

The managed `<vibevm>` block sits at the **top** of each instruction file, so
a truncation costs authored prose — legible to its author and reportable by
`vibe check` — and never the boot router.

The 32 KiB is Codex's own *default*, not a law: your `~/.codex/config.toml`
or a trusted project's `.codex/config.toml` may raise it. So the project
declares the limit it is built for, as
`[system_prompt.harness.codex] doc_max_bytes`, and
`vibe agent <codex launcher>` passes `-c project_doc_max_bytes=<declared>` on
every launch. A Codex started **directly** has not been told, so `vibe check`
warns and hands over the exact one-line setting, and counts the authored
bytes a session will not see.

Verified offline against codex-cli **0.167.0** with `codex debug prompt-input`
(which renders the model-visible prompt and calls no model): a 312,345-byte
lane arrives byte-equal and exactly once, as the first part of the second
developer item — directly behind Codex's base instructions and ahead of
skills, permissions and everything dynamic. A 43,271-byte `AGENTS.md`
renders the router whole and drops its tail canary under the default limit,
and renders both under `-c project_doc_max_bytes=262144`.

**The version is part of that claim.** The carrier is a configuration key,
and a key the program does not read produces no warning, no error and no
non-zero exit — a session that starts looking exactly right with no boot lane
in it. So `vibe agent` and `vibe prompt flags --harness codex` ask the
launcher's program its version and will **not deliver the carrier** on one
this build has not verified, naming the installed version, the verified one
(`0.167.0`), the offline command that re-checks it, and
`[launchers.<name>] harness_version`, which records your own measurement by
number. The two commands then part ways: `vibe agent` starts Codex **without
the carrier**, warning loudly (the soft launch of §4), while `vibe prompt
flags` refuses, because it prints and starts nothing. A version that cannot
be read at all is a note and the carrier is delivered: “cannot tell” is not
“wrong”.

### The lane's markers

Each lane segment is wrapped:

```
<!-- vibe:lane-begin <node-id> <path> <hash12> -->
…the file's bytes, verbatim…
<!-- vibe:lane-end <node-id> <path> <hash12> -->
```

The path is the file's project-relative path rather than a label, because the
truncation rule must tell the model **which file** to read. `<hash12>` is the
first 12 hex digits of the SHA-256 of the bytes between the two lines.
Markers exist only in the composed lane; the files on disk are untouched.

They have four uses — presence (both lines ⇒ do not read that file),
truncation (a begin with no end ⇒ read it from disk), duplication (the same
pair twice ⇒ follow one copy) and staleness (the delivered `STATIC` id
differs from the one the router states ⇒ read the lane from disk). **All four
are instructions to a model**, and a model can skip a comparison or miss an
absent end marker in 300 KB of text. What can be relied on is
`vibe prompt check`, offline: every begin has its end, pairs are unique, each
hash matches its own bytes, the router's expected id matches the current
`STATIC.*`, and no marker-shaped line occurs inside a body.

## 6. Limitations

Each of these is a stated boundary, not a to-do.

- **`import` is refused in this version.** Four blockers: `AGENTS.md`,
  `CLAUDE.md` and `GEMINI.md` are kept byte-identical, so one import line
  reaches every import-processing harness and would deliver the lane twice;
  the lane is not import-inert (this repository's `STATIC.xml` carries 68
  `@spec://` tokens, and the composed lane 74, which Gemini CLI and Qwen Code
  would try to resolve as paths); an import is unconditional, so it reaches
  quiet workers against the worker-exclusion rule; and it follows a harness's
  downward discovery into nested projects. The refusal names all four.
- **Ambient config carriers are refused outright** — a project file a harness
  reads on every launch (`.codex/config.toml`, OpenCode `instructions`, pi
  `APPEND_SYSTEM.md`). Each lands in the slot like `launch` but is
  unconditional like `import`, so each reaches quiet workers.
- **A quiet worker receives no central prefix at all.** The profile and the
  packet are selected before the request is constructed, so the prefix is
  never assembled for a bounded worker. There is no “ignore the parts that do
  not apply to you” clause, and deliberately no attempt to write one: a later
  message cannot return spent context.
- **Nothing user-local is pulled in by reference.** An authored instruction
  that *mentions* a home directory, a local plan or settings file, or a token
  path is an instruction to a tool-using agent, not permission to embed that
  file. `vibe` opens none of them, and the sentence itself travels verbatim.
- **No cache is promised.** A text artifact is not a portable KV cache. Equal
  `SYSTEM.md` bytes do not prove identical actual API prefixes, because
  provider wrapper text, role arrangement, tools and request framing may
  change independently of your file. One cache **read** was observed, and it
  is an observation and not a promise: on 2026-11-10 a repeat Claude Code
  session on a 202,349-byte lane reported zero cache creation and an
  85,262-token read, the same bytes at another checkout path with a different
  `git status` gave 75,803, and this repository's own lane gave 183,981 — which
  is the evidence that `--exclude-dynamic-system-prompt-sections` keeps
  per-machine text out of the cached prefix. **The scope is the claim:** one
  account, one model, one Claude Code version, every run inside one hour, on
  Linux, counters read from each run's own usage record. **No TTL, no latency,
  no hit rate and no saving** is claimed anywhere: those are **not measured
  here**, deliberately, because measuring them spends real money on named
  models and is a separately authorised exercise.
- **Cache diagnostics are read, never computed.** `vibe prompt capture
  --usage <file>` reads a provider response you already have; there is no
  constructor that accepts a local estimate. A counter the provider did not
  report is **absent**, never zero — zero is a measurement.
- **Risks `vibe` cannot control.** A tool list precedes the system slot in
  every provider cache hierarchy that was checked, so an MCP server that
  connects late or changes its tools invalidates the lane's entry. A cache
  entry does not transfer between models, so a mid-session switch pays the
  lane again. A resumed Claude Code conversation keeps the system prompt it
  recorded until compaction — **measured**, on 2026-11-10: a resumed session
  came back with the lane it started with after a boot file had changed, and
  the router could not notice, because the block carries the expected id of
  `STATIC.*` alone. `vibe agent` prints this when your arguments resume a
  session, and a new session is the remedy. Claude Code's file flag is hidden
  from `--help` and Codex's profile option is labelled `CONFIG_PROFILE_V2`;
  either may change — and the fallback is safe by construction, because when the
  slot stops working the markers are absent and the router reads. A ~310 KB
  lane in the slot cannot be dropped by compaction the way tool results can.
  User-local additions (`~/.codex/AGENTS.md`, a harness's own global
  instruction file, auto-memory) enter the context outside `vibe`'s
  accounting.
- **A text separator is not a security boundary.** Moving repository text
  into an instruction role is a real elevation of authority, acknowledged
  rather than assumed away: project instructions, package boot text, examples
  and operational grants do not acquire provider-level authority by sitting
  on disk or by being in a file called `SYSTEM.md`.

## 7. What invalidates what

The question this table answers is the one you need to predict the feature's
behaviour. The six verdicts are exactly `vibe prompt check`'s own, and each
non-`equal` one names the command that resolves it.

| What changed | Verdict | What to do |
|---|---|---|
| the task, the time, the current plan, the working directory | — | nothing. The prefix is never built from any of these. |
| the bytes of an applicable dynamic boot file, plan unchanged | `equal` | nothing. A fresh continuation is read per request, and `vibe agent` recomposes the lane at launch. |
| the template's bytes | `stale` | `vibe prompt build` |
| the entrypoint's bytes, or a different declared `entrypoint` | `stale` | `vibe prompt build` |
| the compiled lane's bytes | `stale` | `vibe prompt build` |
| `recipe`, `lane_form` or `host` | `stale` | `vibe prompt build` |
| `INDEX.md`'s composition, order or a condition | `stale` | recompute the boot plan: `vibe install`, then `vibe prompt build`. Never pair an old prefix with a new plan. |
| a generated artifact is gone | `missing` | `vibe prompt build` |
| the template is gone and artifacts remain | `stale` (opted out) | `vibe install` or `vibe prompt build` retires them |
| git tracks a generated artifact | `conflict` | `git rm --cached <path>` |
| the node's world cannot be read at all | `unavailable` | fix the checkout; the message names the path |
| the sidecar witnesses a lane with no `STATIC.*` on disk | `freshness_unverifiable` | `vibe prompt build` |
| the model, the tools, the SDK wrapper or the request framing | — | the file may be identical while the provider's actual cached prefix differs. A local digest is not a remote cache identity. |
| the provider's TTL expired | — | a new cache write may be needed with a completely unchanged `SYSTEM.md`. |

Each `stale` row above is one case of
`crates/vibe-cli/tests/cli_prompt_stale.rs`'s own sweep, which asserts the
verdict **and** the sentence that names it.

### Determinism, and what it rests on

The same relevant inputs always give the same bytes: the segment order is
fixed, the framing and encoding are fixed (UTF-8, no BOM, LF frames), the
continuation plan's order is `INDEX.md`'s order, and the composer is
*incapable* of observing a path, a clock, an environment variable or the host
operating system — pinned by a test over its own dependency surface. Two
checkouts of the same commit in different directories produce byte-identical
artifacts, and a repeated generation with unchanged inputs rewrites nothing
and does not even move an mtime.

One repository-level prerequisite: `.gitattributes` must pin line endings
(`* text=auto eol=lf` here). Without it a Windows checkout produces different
bytes. The cross-operating-system half of the determinism claim is argued
from that plus the composer's proven inability to observe the host; it is
**not measured here** on macOS or Windows, where only
`cargo check --target x86_64-pc-windows-gnu` and the cross-platform test
discipline run.

**It is, however, now runnable by whoever has one of those hosts.**
`campaigns/sys-prompt-2026-11/live/determinism-hashes.sh <scratch-dir>`
builds a committed fixture project twice, at two different paths, with
`vibe install --offline` against a `file://` registry inside the copy — no
network, no agent, no model — and prints the SHA-256 of the three artifacts
plus the two-paths verdict. It prints a digest of the fixture's own **inputs**
first, so a checkout whose bytes differ is diagnosed as that rather than
discovered as an artifact mismatch; the fixture tree carries an explicit
`text eol=lf` attribute so the heuristic is out of the argument. The Linux
figures are recorded in that campaign's `LOG.md`; a macOS or Windows run that
prints the same three hashes closes the claim.

## 8. Does caching save you anything?

Parametrically, and only parametrically. For a prefix of **L** tokens with
**W** cache writes and **H** cache reads, compare

```
uncached:  L × (W + H) × uncached_rate
cached:    L × (W × write_rate + H × read_rate) + storage_cost
```

then add the requests that cannot be cached at all and the unchanged cost of
the dynamic and output tokens. **Substitute current rates**: every rate is
read from your provider's current pricing, never from this document, which
would be wrong within a quarter.

What this repository's own numbers put into **L**: a prefix of about 278 KB
and an agent lane of 312,345 bytes. A token count depends on the
tokenizer, so use the provider's own usage report rather than a byte-to-token
rule of thumb — which is also why `vibe prompt capture --usage <file>` reads
the counters out of a response instead of estimating them.

Two honest caveats on the arithmetic itself. A first request and frequent
expiries can reduce or erase the benefit entirely. And **padding a prefix
with useless text to reach a provider's minimum cacheable length is forbidden
outright** — the prefix's size is the sum of what the resolution says the
session must have, and nothing else.

Whether a boot that large is deserved at all is a separate engineering
question this feature does not answer. Stabilising a large prefix is not the
same as earning one.

## 9. Adopting it in an existing project

1. `cp docs/system-prompt/SYSTEM-TEMPLATE.example.md vibevm/vibespecs/boot/SYSTEM-TEMPLATE.md`
   and edit it down to your project's standing rules.
2. `vibe prompt build` — read the summary line, and check the two sizes
   against what you expected.
3. `vibe prompt check --json` — confirm `equal`, and read the warnings. They
   are the ones worth acting on: an `@` token census, the Codex whole-file
   budget, the lane's size against `--model-window`, a nested managed block,
   a `vibedeps/` copy whose block is not neutralised.
4. `git status` — nothing new should appear. If an artifact does, `vibe` did
   not get to write its ignore file; run the build again and read the error.
5. Add a launcher for however you actually start your agent, and launch
   through `vibe agent <launcher>` instead of the bare program.
6. If your entrypoint carries frequently-edited current-state material — a
   machine fact, an operating ledger, a date — move it into the boot lane.
   The entrypoint is a build input, so every such edit is a cache write. The
   remedy is repository hygiene, and `vibe` will not take it on your behalf:
   it never summarises, shortens, reorders or filters authored instructions.
