# DRIFT-013 — `--plain`'s help stops describing a Phase 2 that shipped {#root}

<status stage="impl" state="plan" ref="DRIFT-013"/>

**Status:** queued
**Executor:** Opus. **Reviewer:** Fable, against §6 verbatim.
**Cluster:** cli (vibe-cli clap surface)
**Unit-stability check:** PROP-036's status line and §2.11 were corrected in
Phase D d2a/d2d; the contract this help text cites is now accurate, so the
code is the only side still stale.

## 1. Goal {#goal}

`vibe tree --plain --help` describes what the flag does today instead of
telling the user it is a no-op.

## 2. Contract {#contract}

> **Fallback:** non-tty and `--plain` render a static ASCII tree; `--json`
> the JSON — neither enters interactive mode.
> — `spec://vibevm/modules/vibe-cli/PROP-036#tui`

> **Status: IMPLEMENTED** … `vibe tree --json` validates against the shipped
> `package-tree.schema.v1.json` per its own `--help`, and `-t` is live.
> — `spec://vibevm/modules/vibe-cli/PROP-036#root`

Finding realised: **F-036**.

## 3. Current state {#current}

From Phase C verification evidence — do not re-discover:

- `crates/vibe-cli/src/cli/inspect.rs:85-89` — the doc comment on `plain`
  reads: *"Force the plain ASCII tree instead of the interactive TUI. The TUI
  is Phase 2 (PROP-036 §2.11); today output is plain regardless, so this flag
  is currently a no-op on a tty."*
- That was true when written. It is false now: the console TUI shipped
  (PROP-037 Spec 2) and is the attended default — the sibling flag at
  `inspect.rs:91-94` already calls `-c` *"today's default"*, so the two
  comments in the same struct contradict each other.

## 4. Required behavior {#behavior}

1. Rewrite the `plain` doc comment to state the shipped behaviour: `--plain`
   forces the static ASCII tree and suppresses the interactive TUI, on a tty
   as well as off it. Keep the `spec://` citation, repointed to the anchor
   that actually governs the fallback.
2. Read the surrounding flags in the same struct while you are there and fix
   any other help text that describes an unshipped state. Report each one you
   changed **and each you deliberately left**, with the reason — a sweep that
   silently redefines "while I was here" is worse than a narrow fix.
3. Verify the behaviour before describing it: run `vibe tree --plain` on a
   tty and confirm it really does render the static tree rather than the TUI.
   If it does **not**, the finding is bigger than a comment — the flag is a
   genuine no-op — so STOP per §8 rather than writing help text that is
   accurate about nothing.

Edge cases: `--plain` with `-c` (console) and with `-t` (terminal) — say in
the help which wins, or say nothing rather than guess. `conflicts_with` is
declared on `-c`/`-t` but not on `--plain`; if the combination is legal,
the help should not imply otherwise.

Error paths: none — help text and, if §4.3 forces it, a clap `conflicts_with`.

## 5. Boundaries {#boundaries}

- Help text and clap attributes only. Do **not** change the rendering
  behaviour of `vibe tree` — if the flag misbehaves, that is §8, not a fix
  to make the comment true.
- Do not touch the TUI, the goldens, or the tree model.
- Never edit spec text.

## 6. Acceptance {#acceptance}

```bash
cargo test -p vibe-cli
bash tools/self-check.sh
cargo run -q -p vibe-cli --bin vibe -- tree --help
```

- `vibe tree --help` output quoted verbatim in §9, showing the new text.
- If any `--help` golden or snapshot test covers this string, it is updated
  in the same commit — a golden that still asserts the stale sentence is the
  same defect one layer down.
- Manual: `vibe tree --plain` on a tty renders the static ASCII tree; record
  the first three lines in §9 as proof the help now matches reality.
- Discipline: `cargo fmt --all`, clippy clean, atomic commits, no AI
  attribution.

## 7. Analogies {#analogies}

`crates/vibe-cli/src/cli/inspect.rs:91-94` — the `-c` flag's comment, which
is already written in the shipped present tense and cites its contract.

## 8. Stop rule {#stop}

If `--plain` turns out to be a genuine no-op on a tty: STOP, mark
`<!-- REVIEW: … -->` at the flag, record it in §9, set status `returned`.
That is a behaviour defect against `PROP-036#tui`, and it wants its own task
and its own test — not a comment that describes an intention.

Budget signal: past ~2 files or ~80 lines, stop and return.

## 9. Log {#log}

- queued 2026-07-25 (Fable). Found by Phase C c1 from the manual-test side.
- executed 2026-07-25 (Opus). §8 did **not** fire — `--plain` is real.

**§4.3 first, before any rewrite.** Verified on a genuine console (a fresh
`pwsh` window: `[Console]::IsOutputRedirected = False`, host `ConsoleHost`,
no redirection of stdout, which is what `console::user_attended()` reads):

- `vibe tree --plain --path .` renders the static ASCII tree and exits `0`.
- Control on the same kind of console: `vibe tree -c --path .` was **still
  running after 12 s** — the TUI blocking on input. So the tty is genuinely
  detected there, and the plain output above is `--plain` doing work, not the
  non-tty fallback firing by accident.

First three lines of `vibe tree --plain` on that tty (scraped from the console
screen buffer, since redirecting stdout would have destroyed the tty):

```text
project: C:\Users\olegc\git\v\vibevm
STATIC.md: 62835 bytes, 1390 lines, 26 contribution(s)
packages: 36   roots: 5
```

**Edge cases (§4).** On the same console, `vibe tree --plain -c` and
`vibe tree --plain -t` are both **legal** — clap accepts them, because
`conflicts_with` is declared only between `-c` and `-t` — and both exit `0`
printing the plain tree, with no vibeterm/vibeframe process appearing. So
`--plain` wins over `-t` as well as over `-c`; that matches the code
(`commands/tree/mod.rs:89` and `:102` both guard on `!args.plain`). The help
now states that precedence instead of guessing at it, and **no
`conflicts_with` was added to `--plain`**, since the combination is legal and
the help must not imply otherwise.

**§4.1 — the rewrite.** `crates/vibe-cli/src/cli/inspect.rs:85-89`, citation
kept and repointed at the anchor that governs the fallback (`PROP-036 §2.11`
`#tui`, `##TUI-FALLBACK`) rather than at "the TUI is Phase 2":

```rust
/// Force the plain ASCII tree instead of the interactive TUI — on a tty
/// as well as off it (PROP-036 §2.11: `--plain` and a non-tty both render
/// the static tree; neither enters interactive mode). Legal together with
/// `-c`/`-t` and wins over both, so `--plain -t` prints the tree here
/// rather than opening vibeterm.
```

**§4.2 — sweep of the rest of `TreeArgs`, with reasons for the ones left:**

- `plain` — **changed**, above.
- `console` (`inspect.rs:93-96`) — **left**. "(today's default)" is accurate:
  `resolve_launch_mode` falls through to `TreeSettings::launch_mode`, and an
  absent/unknown `vibe.tree.launch-mode` parses to `LaunchMode::Console`
  (`commands/tree/tui/settings.rs:163-167`, "the clean-install default").
  This is §7's analogy and it still holds.
- `terminal` (`inspect.rs:98-101`) — **left**. `-t` is live (PROP-036 status
  line) and the text is already in the shipped present tense; the run above
  confirms nothing but `--plain` diverts it.
- `path` (`inspect.rs:81-83`) — **left**. "Defaults to the current directory"
  is true of the flag (`default_value = "."`). It is *incomplete* — the human
  surfaces then fall back to the remembered last project and finally to a
  folder picker (`commands/tree/mod.rs:112-172`) — but incomplete is not
  "describes an unshipped state", and documenting the whole resolution order
  is a different task's scope, not this one's "while I was here".

**Goldens.** None. Searched the tree for each of these help strings and for
`--plain`: no golden, snapshot, or assertion covers them. The only other
place the fallback is described is the subcommand's own about-text
(`crates/vibe-cli/src/cli.rs:182-188`), which already reads "a non-tty or
`--plain` renders a static ASCII tree" — correct, left alone.

**Noted, out of scope (§5).** The module doc at
`crates/vibe-cli/src/commands/tree/mod.rs:6-8` still calls the TUI "Phase 2"
— the same stale framing, one file over. It is neither help text nor a clap
attribute, so §5 kept this task off it; it wants its own line in a later
sweep. No marker was left in the source, since §8 did not fire.

`vibe tree --help` verbatim after the change (clap does not wrap here, so the
lines are terminal-width independent):

```text
Analyze the resolved spec/dependency tree (PROP-036): the effective boot load type per package (`static` / `dynamic` / `none`), the transitive / condition / STATIC.md flags, the two boot lanes, and the in-place `@spec` markers. Read-only. `--json` emits the machine model (validated against the shipped `package-tree.schema.v1.json`); a non-tty or `--plain` renders a static ASCII tree

Usage: vibe.exe tree [OPTIONS]

Options:
      --json                Produce machine-readable JSON output
      --path <PATH>         Project root. Defaults to the current directory [default: .]
      --plain               Force the plain ASCII tree instead of the interactive TUI — on a tty as well as off it (PROP-036 §2.11: `--plain` and a non-tty both render the static tree; neither enters interactive mode). Legal together with `-c`/`-t` and wins over both, so `--plain -t` prints the tree here rather than opening vibeterm
      --quiet               Reduce output to a single summary line (useful in scripts / CI)
  -c, --console             Open the in-terminal console TUI (today's default). Mutually exclusive with `-t` (TERMINAL-AIUI §6.2)
      --invoked-by <AGENT>  Identifier of the agent or harness invoking this command. Free-form string; conventional values are `claude-code`, `claude-desktop`, `cursor`, `opencode`, `codex`. When set, the value is stamped onto every JSON envelope vibe emits (`"invoked_by": "<value>"`) so the caller's context is recoverable from logs and machine-readable output. Falls back to the `VIBE_INVOKED_BY` environment variable when the flag is absent; flag wins on conflict. The `vibevm` skill installed by `vibe mcp install --with-skill` instructs each agent to pass this flag automatically
  -t, --terminal            Open in the vibeterm desktop terminal instead of the current terminal. Mutually exclusive with `-c` (TERMINAL-AIUI §6.2)
      --unattended          Run unattended — skip every confirmation prompt and refuse to open any interactive wizard. Equivalent to passing `--assume-yes` (`vibe install` / `vibe uninstall`) or `--yes` (`vibe mcp install` / `upgrade` / `uninstall`) to whichever subcommand needs it. Falls back to the `VIBE_UNATTENDED` environment variable (truthy values: `1`, `true`, `yes`, `on` — case-insensitive); flag wins on conflict. Stamps `"unattended": true` on every JSON envelope so log aggregators can tell scripted runs from interactive ones. Designed for first-time-user provisioning, CI, and other fully scripted environments
  -h, --help                Print help
```

**Acceptance run (§6).** `cargo fmt --all` run; `cargo fmt --all --check`
clean. `cargo test -p vibe-cli` — green, exit 0. `bash tools/self-check.sh`
— fmt / `cargo test --workspace` / `cargo clippy --workspace --all-targets
-D warnings` / `vibe check --path . --quiet` (0 errors) all green, then
`cargo xtask conform check` reported findings in **other sessions' in-flight
files only** (`crates/vibe-registry/src/git_backend/shell.rs` over the
600-line budget; an `.expect()` in `crates/vibe-cli/src/commands/progress/
tests.rs`) — two other agents were editing the tree throughout. Nothing in
this task's blast radius: `cargo xtask conform check --scope
crates/vibe-cli/src/cli` is **0 findings, exit 0**. Not fixed here, by design
— they belong to whoever is holding those files.
