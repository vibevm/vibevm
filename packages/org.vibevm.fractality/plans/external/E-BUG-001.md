# E-BUG-001 — packet `acceptance` mangles quoted multi-word commands

- **Status:** open — filed for discussion during fractality's own development.
- **Filed:** 2026-07-12
- **Component:** `fractality-pod` (the acceptance / collect stage that runs a
  packet's `[task].acceptance` commands in the workspace).
- **Severity:** medium — the run's pass/fail *acceptance signal* is wrong (false
  negative). The deliverable is unaffected, but a boss trusting
  `acceptance: N/N ok` without reading the diff would be misled.
- **Found by:** the MT-05 host-relicense dogfood — run
  `01KXBEHEYJCQ1RNJ5657Q31HVA` (relicensing the host root `LICENSE.md` to
  UPL-1.0), GLM `small`, exit 0, $0.39.
- **Evidence:** `~/.fractality/runs/01KXBEHEYJCQ1RNJ5657Q31HVA/{acceptance.log,
  pod.log}` — preserved with the run under `reports/trial-results/`.

## What happened

The packet declared two acceptance checks as command strings:

```toml
acceptance = [
  "findstr /C:\"Subject to the condition set forth below\" LICENSE.md",
  "findstr /C:\"must be removed before any redistribution\" LICENSE.md",
]
```

Both returned `exit Some(1), ok=false` → the run reported `acceptance: 0/2 ok`,
even though the worker had produced a byte-correct `LICENSE.md` (independently
verified: `diff` against the canonical UPL text differed only by the intended
appended note, and `grep` found no EULA/proprietary text remaining).

`acceptance.log` shows `findstr` treating every word of the quoted phrase as a
separate filename:

```
=== findstr /C:"Subject to the condition set forth below" LICENSE.md (exit Some(1), 30 ms) ===
FINDSTR: Cannot open to
FINDSTR: Cannot open the
FINDSTR: Cannot open condition
FINDSTR: Cannot open set
FINDSTR: Cannot open forth
FINDSTR: Cannot open below"
```

`pod.log` confirms the pod stored the command *with its quotes intact* and still
got exit 1:

```
acceptance command finished command="findstr /C:\"Subject to the condition set forth below\" LICENSE.md" exit_code=Some(1) ok=false
```

## What I wanted

Each acceptance string to run as **one** command that honours its quoting — i.e.
`findstr` searches for the single literal phrase
`Subject to the condition set forth below` inside `LICENSE.md` and exits 0
because that phrase is present. Result: `acceptance: 2/2 ok`.

## What I got

`findstr` received the phrase **split on whitespace into separate arguments**. It
took the first token (`/C:"Subject`) as the search literal and treated `to`,
`the`, `condition`, `set`, `forth`, `below"`, … as **filenames to open** — hence
"Cannot open to" — and exited 1. The quotes did not group the phrase into one
argument; they survived as literal characters inside the tokens instead.

## Why these are different

The command string was tokenised on whitespace **without honouring shell
quoting**. A shell — or a quote-aware parser — keeps
`"Subject to the condition set forth below"` as one argument to `findstr /C:`;
naive whitespace-splitting breaks it into eight, destroying the search phrase and
turning the remaining words into bogus filenames. So "one findstr searching for a
phrase" silently became "findstr searching for the token `/C:"Subject` across
seven nonexistent files." The failure is *plausible* (a real non-zero exit with a
real-looking error), which is what makes it dangerous.

## Ideas — what the problem is

1. **Most likely:** the acceptance runner builds child-process argv by splitting
   the command string on whitespace (e.g. `cmd_str.split_whitespace()`), then
   spawns the program with those args directly — bypassing shell quote semantics.
   The intact `command="…"` in `pod.log` shows the quotes survive TOML parsing, so
   the mangling happens at the spawn/tokenise step, not earlier.
2. It does **not** run the command through a shell (`cmd /C …` on Windows,
   `sh -c …` on POSIX). If it did, the shell would parse the quotes correctly —
   which is the mental model a packet author has (they expect the string to behave
   as if pasted into a terminal).
3. Windows `findstr` quoting is itself notoriously brittle; combined with (1) it
   makes this class of failure easy to hit and easy to miss.

## Ideas — how to fix it

1. **Run acceptance through a shell** (lowest surprise): `cmd /C "<command>"` on
   Windows, `sh -c "<command>"` on POSIX. The string then behaves exactly as a
   packet author expects. Cost: a shell dependency + a platform branch; acceptance
   semantics become shell-defined (pipes/redirects/globs start working, for better
   or worse).
2. **Quote-aware tokenisation** without a shell: split the command with a
   `shell-words`-style parser (respects `"…"` / `'…'`) before spawning. Stays
   shell-free and cross-platform, but re-implements quoting and won't support
   pipes/redirects.
3. **Change the schema to explicit argv** (most robust): allow
   `acceptance = [["findstr", "/C:Subject to the condition set forth below", "LICENSE.md"]]`
   (array-of-argv) so there is nothing to tokenise. Downside: less ergonomic than a
   command line; a packet-schema migration.
4. **Surface the discrepancy regardless:** when an acceptance command exits
   non-zero, echo its captured stderr into the run summary (it already lands in
   `acceptance.log`) so a false negative is diagnosable at a glance, not only by
   opening the log.

Pragmatic combination: option 1 (shell) for ergonomics now + option 4 (surface
stderr) so failures are legible; reach for option 3 (argv) if we later want
acceptance to be provably shell-independent.

## Workaround (until fixed)

Packet authors: avoid multi-word quoted phrases in `acceptance`. Use a
single-token match (`findstr Subject LICENSE.md`), a helper script the worker
leaves in the workspace, or lean on **boss-side verification** (diff + grep) as
the real gate — which is exactly what caught this: the acceptance signal was
wrong, the boss review was right. Sibling lesson from MT-05: acceptance must
assert what *changed*, not what is merely present — and either way it is advisory
until the boss reads the diff.

## References

- MT-05 dogfood: [`../../fractality/v0.1.0/spec/manual-tests/MT-05-dogfood-relicense.md`](../../fractality/v0.1.0/spec/manual-tests/MT-05-dogfood-relicense.md) (Recorded run 2026-07-12).
- Delegation review loop (acceptance is advisory until the diff is read):
  `delegation-rules/v0.1.0/spec/flows/delegation-rules/DECISION-MATRIX.md` §review-loop.
