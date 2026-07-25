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
