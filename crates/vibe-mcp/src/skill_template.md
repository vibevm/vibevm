---
name: vibevm
description: Use whenever the user mentions vibevm, vibe install/update/check/show/outdated/registry/search/init/list/uninstall/mcp/skill/agentic/command, packages, skills, subskills, or the lockfile. Also when the user asks to create a new vibevm-managed project. The MCP server exposes lockfile-aware queries; query it before guessing about installed packages. `vibe skill` installs package-declared skills into agents; `vibe agentic …` + `vibe command` delegate reasoning tasks back to you to carry out.
---

# vibevm

This skill teaches you how to use **vibevm** — a CLI package manager
for spec-driven AI-assisted development. It covers only the package
manager's CLI surface and the MCP-server queries it exposes. Any
project-specific conventions (custom commit rules, write-ahead log
protocols, custom spec corpus, etc.) live elsewhere — see the last
section.

## First step — detect the situation

Check whether `vibe.toml` exists in the working directory.

- **Exists** → the directory is a vibevm-managed project. You can
  call `vibe install`, `vibe update`, MCP tools, etc. against it.
  See [Section B](#section-b--inside-a-vibevm-project).
- **Absent** → not a vibevm project. If the user wants to create
  one, see [Section A](#section-a--no-vibevm-project-here-yet).
  If they just want ad-hoc CLI access (`vibe search`, `vibe show
  config`, etc.), most read-only commands work without a project.

If the user explicitly says "create a vibevm project" / "set up
vibevm" / "I have a vibevm project" — trust them, skip the probe.

## Section A — no vibevm project here yet

To create one:

1. **`vibe init`** — scaffolds `vibe.toml` (project manifest with
   default registries) + an empty `vibe.lock`. It also writes a
   handful of starter files (`spec/boot/00-core.md`,
   `spec/boot/90-user.md`, `CLAUDE.md`, `AGENTS.md`, `GEMINI.md`).
   These starter files are **project conventions**, not load-bearing
   for vibevm itself — they're a template the project's owner can
   keep, edit, or delete. vibevm commands work the same way whether
   they're present or not.

   **Two registries are already configured** by `vibe init`:
   `vibespecs` (GitHub) and `vibespecs-gitverse` (GitVerse). **Do
   NOT call `vibe registry add`** unless the user explicitly asked
   for a custom host — adding a redundant entry just slows resolves
   and confuses code review.

2. **(Optional) `vibe install <pkgref> --assume-yes`** — install one
   or more packages from the configured registries. There is no
   required first package.

3. The project is now operational. Switch to Section B for further
   work.

### Happy path — typical bootstrap

The two-command shape an agent should use when the user asks to
"create a vibevm project with `<kind>:<name>`":

```
vibe init --invoked-by <agent>
vibe install <kind>:<name> --assume-yes --invoked-by <agent>
```

That is sufficient. No `vibe search`, no `vibe registry add`, no
environment variables. If the install fails with a real error
(see "Reading exit codes" below), report it back to the user
verbatim — do NOT improvise registry / index reconfiguration.

## Section B — inside a vibevm project

### Use the MCP server before guessing

The vibevm MCP server is registered as `vibevm` in your MCP config.
The lockfile is canonical — query the server rather than inferring
package state from filenames or commit history.

Tools:

- **`query_package(name)`** — returns the lockfile entry for an
  installed package: kind, name, version, content_hash, registry,
  source_url, source_ref, resolved_commit, files_written, features,
  subskills_active (with delivery + describes), top-level describes
  PURL, language.
- **`read_subskill(package, subskill_path)`** — returns the textual
  content of an active subskill's files. For `lazy-pull` deliveries,
  reads from the package cache; for `eager` / `lazy-push`, reads
  from the project tree.
- **`materialise_subskill(package, subskill_path, force?)`** —
  promotes a `lazy-pull` subskill into the project tree on demand.
  Refuses to overwrite existing files unless `force=true`.

If the user asks about installed packages — what's installed, what
version, what features are active, what files a package contributed
— call `query_package` first. Don't infer.

### The type oracle (`tcg_*` — projects with a discipline stack installed)

When the project's lockfile carries a language stack that ships the
agentic type oracle (today: `stack:org.vibevm/typescript-ai-native`),
four more tools answer type questions at millisecond latency from a
persistent language-service process. Consult them BEFORE writing an
edit — checking a hypothetical file costs less than one red gate run:

- **`tcg_validate(language, file, content?)`** — type-check `file`
  through the project's own compiler; pass `content` to check an
  UNWRITTEN edit (an in-memory overlay — disk is never touched).
  Returns compiler diagnostics PLUS the discipline gate's findings,
  each flagged `baselined` (sanctioned) or new, and advice strings.
- **`tcg_scope(language, file, position?)`** — what is in scope:
  symbols with kinds, the file's cell and seam, and the branded types
  exported at reachable seams (heuristic-labelled).
- **`tcg_complete(language, file, position, content?, prefix?, max?)`**
  — type-valid completions at a position; entries carry type text and
  an `unsafe` flag on any-typed candidates. Pass `prefix` — details
  are computed after the cut.
- **`tcg_type(language, file, position, content?)`** — quick info
  (type display + docs) at a position.

`language` is `"typescript"` today (more arrive as values, not new
tools). The floor gates stay the truth — the oracle exists so your
edit passes them on the first try. If the stack is not installed the
tools answer with the exact `[requires]`/`vibe install` recipe.

### Project conventions are out of scope here

A vibevm project may carry its own additional disciplines (write-
ahead-log protocols, commit-message rules, spec corpus,
PROP/FEAT documents, etc.). Those are NOT part of this skill — they
live in:

- The project's own `CLAUDE.md` / `AGENTS.md` / `GEMINI.md` (if
  present — read those before acting; they're how the project's
  owner expresses local rules).
- Additional skills installed in the project's `.<agent>/skills/`
  or your user-level `~/.<agent>/skills/` dirs.
- Specific packages installed via `vibe install` (for example,
  `flow:wal` ships a write-ahead-log protocol; a project that
  installed it has opted into that protocol; one that didn't,
  hasn't).

If you see custom conventions in the project (new files in `spec/`
you don't recognise, references to a protocol in CLAUDE.md, etc.),
follow what's documented there. This skill doesn't impose any
particular workflow on the project.

## Agentic commands — vibevm composes the instruction, you execute it

vibevm carries stable, algorithmic knowledge of its own domain — the
spec-driven model, the package and dependency rules, the discipline. For a
task that needs reasoning, it puts that knowledge to work: it composes a
precise, domain-grounded **instruction** and hands it to you to carry out.
Two things make this the right division of labour, not a hand-off of
drudgery:

- **In agent mode you are the better executor.** You hold the live
  context, the tools, and the conversation with the user, so you are best
  placed to act on a complex instruction.
- **vibevm's instruction is worth trusting.** It is grounded in vibevm's
  algorithmic domain knowledge rather than improvised, so it is more
  informative and more reliable than a prompt written from scratch. Treat
  it as the authoritative description of the task and follow it.

These are the `vibe agentic …` commands. The protocol is two steps:

1. Run a producer, e.g. **`vibe agentic explain`**. It does NOT produce
   the answer — it parks an instruction and tells you to fetch it. (It
   writes only `.vibe/agentic/command.md`; nothing else.)
2. Run **`vibe command`**. It prints the parked instruction to stdout and
   clears the slot. **Read that instruction and carry it out yourself** —
   it is a task *for you*, not a report to echo back verbatim. When the
   slot is empty it prints `no pending command` and exits 0.

Example: `vibe agentic explain` parks "explain this project in at most
three paragraphs from README.md + vibe.toml". After `vibe command`, do
exactly that and answer the user.

There is **no automatic write-back** — vibevm does not see your result.
If the result should reach vibevm, that is your call to arrange with a
follow-up command; vibevm will not wait for one.

### Two ways to reach these — pick by situation

- **One-shot CLI** (`vibe agentic explain`, then `vibe command`):
  stateless, one process per call. Best when you touch vibevm once and
  move on — e.g. scanning a directory of many vibevm projects for a quick
  fact. Per-call state does not survive.
- **MCP server** (the `vibevm` server's `agentic_explain` tool): the
  instruction comes back inline in the tool result — no file, zero
  latency, session persists. Best when you are working inside one project
  for a while. If the `vibevm` MCP server is registered, prefer its
  `agentic_explain` tool over the CLI two-step.

## Installing package skills into agents — `vibe skill`

Packages can ship **skills** for coding agents (declared in a package's
`[[skill]]` manifest table — separate from the package's kind, so a
`tool`, `flow`, `feat`, or `stack` can all carry skills). `vibe skill`
projects the skills declared by the project and its installed packages
into agents' own skill directories (`.<agent>/skills/<name>/`). This is
vibevm's **standalone mode** — no LLM required.

- **`vibe skill list`** — what skills are declared and where they'd land.
- **`vibe skill install [--agent <a>] [--scope project|user|both] [--skill <name>] --assume-yes`**
  — project them. Default: every declared skill into every
  skill-supporting agent (Claude Code, OpenCode, Codex). Idempotent.
- **`vibe skill uninstall …`** — remove only vibevm-projected skills.

## Common — applies to both sections

### Non-interactive invocation: always pass `--assume-yes`

You are running vibe through an agent harness, not at a real terminal.
`vibe install` and `vibe uninstall` show the plan and then prompt the
operator to confirm; without a TTY they exit with code `1` and the
message:

```
error: no TTY available for confirmation; re-run with `--assume-yes` to apply this plan non-interactively
```

The plan IS printed before that error, which can be misread as success.
**It is not success — exit code is 1 and nothing was written to disk.**

Always pass `--assume-yes` (alias `--yes`) on every `vibe install` /
`vibe uninstall` invocation. The plan still prints; the prompt is
skipped; the command runs to completion. Same flag for both commands.

### Reading exit codes

`vibe` exits with `0` on success, non-zero on failure. The output
preceding the prompt or the report is **not** a status indicator on
its own — read the exit code (or the `error:` prefix in the last
output lines) before declaring victory. `error:` in the last line is
always a failure even if a plan / partial report was printed earlier.

### `vibe search` is optional discovery

`vibe search <query>` walks per-registry index servers (PROP-005).
On a fresh machine the index URLs are usually unset (`VIBEVM_INDEX_URL_<R>`
is empty) and `search` returns "0 searched, N without index URL".
**This is expected, not an error, and does not block install.**

If you already know the pkgref (`flow:wal`, `stack:rust-cli`, etc.)
go straight to `vibe install <kind>:<name> --assume-yes`. The install
path resolves through the configured `[[registry]]` entries directly
via git — it does not consult the index. The index is a discovery
optimisation, not a runtime dependency.

Do not "fix" empty search results by adding new registries, setting
fictional `VIBEVM_INDEX_URL_<R>` values, or otherwise mutating the
project's configuration.

### `vibe --help` is the source of truth for the CLI

The vibevm CLI evolves between releases. Before suggesting **any**
vibe command, run `vibe <subcommand> --help` and read the actual
current surface. Do not invent flags from training data — they
will be wrong.

Frequently relevant subcommands (all of them have flags worth
checking via `--help`):

- `vibe init` — scaffold a new project (Section A entry point).
- `vibe install <pkgref> --assume-yes` — install one or more packages.
  `--assume-yes` is mandatory in non-TTY runs (see above).
  `vibe install` with no pkgref reads `vibe.toml` `[requires]` and
  installs every entry — the cargo `cargo build` shape.
- `vibe uninstall <pkgref> --assume-yes` — symmetric to install.
- `vibe update <pkgref>` / `vibe update --all` — version-bump within
  the lockfile's root constraints.
- `vibe outdated [--json]` — preview upstream-newer versions.
- `vibe check [--quiet] [--json]` — spec-consistency linter.
- `vibe show effective | config | features | subskills | purls`
  — pure inspection.
- `vibe list [--kind] [--verbose]` — what's in the lockfile.
- `vibe search <query>` — optional discovery (see above; safe to skip).
- `vibe registry list` — inspect registries. Do **not** call
  `vibe registry add` / `set-mirror` / `remove` unless the user
  explicitly asked for it; `vibe init` already wires the defaults.
- `vibe registry vendor | sync | publish` — operator-driven registry
  CRUD; reach for these only on explicit request.
- `vibe mcp install | upgrade | uninstall | status | serve` —
  agent integration. Use `--scope project|user|both` and
  `--what mcp|skill|both` to control where and what.
- `vibe skill list | install | uninstall` — project package-declared
  skills into agents (standalone mode). `--assume-yes` in non-TTY runs;
  `--agent` / `--scope` / `--skill` narrow the target.
- `vibe agentic explain` then `vibe command` — delegated reasoning: the
  first parks an instruction, the second prints it for you to carry out
  (see the agentic-commands section above). No auto write-back.

### Pass `--invoked-by <agent>` (recommended)

vibevm stamps a top-level `invoked_by` field on every JSON envelope
for log / telemetry attribution. Pass `--invoked-by <YOUR-AGENT>`
on every CLI invocation:

- `--invoked-by claude-code`
- `--invoked-by claude-desktop`
- `--invoked-by cursor`
- `--invoked-by opencode`
- `--invoked-by codex`

Or set `VIBE_INVOKED_BY=<your-agent>` once for the session — the
CLI reads it whenever the flag is absent. The flag wins on
conflict.

This is **not** required for the commands to function — without
it, envelopes simply have no `invoked_by` field. But the operator
relies on attribution to track which agent proposed which change,
so pass it whenever you can.
