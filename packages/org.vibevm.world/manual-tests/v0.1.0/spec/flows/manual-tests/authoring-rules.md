# Authoring rules for manual tests {#root}

**Scope of this document.** The four rules every manual test must
satisfy, each with a short worked fragment. The rationale for the tier
lives in [`MANUAL-TESTS-PROTOCOL.md`](MANUAL-TESTS-PROTOCOL.md); the
copy-ready skeleton that bakes these rules in lives in
[`test-template.md`](test-template.md).

A walkthrough that breaks any of these is a bug in the test, not a
property of the product it exercises.

## Rule 1 — Clean slate is mandatory {#clean-slate}

Every run starts from nothing and touches no real user state. Two
mechanisms, always both:

- **A scratch project** created fresh with `mktemp -d`, so the working
  tree the test operates on is disposable and unique per run.
- **An environment redirect** pointing the tool's per-user state
  (cache, config, home) into that scratch, so the real per-user
  directory is never read from or written to.

```
export SCRATCH="$(mktemp -d)"
export TOOL_HOME="$SCRATCH/tool-home"   # redirect the tool's per-user state
export PROJECT="$SCRATCH/project"
mkdir -p "$TOOL_HOME" "$PROJECT"
cd "$PROJECT"
```

Now every command the tool runs reads and writes under `$SCRATCH`.
Nothing under the developer's real home is at risk, and two runs on
the same machine cannot collide.

**A test that mutates real user state is a bug in the test**, even if
every step passes — because the next contributor's run inherits that
mutation and the walkthrough is no longer reproducible. If a step
seems to *need* the real per-user directory, the redirect is wrong or
incomplete; fix the redirect.

## Rule 2 — Self-contained walkthrough {#self-contained}

A reader opens exactly one file, executes it top to bottom, and needs
nothing else — no companion doc, no tribal knowledge, no "ask whoever
wrote this". The file names its own preconditions and provides its own
setup.

Every step is a **command block plus an "Expected" paragraph**. The
command is copy-pasteable; the Expected states the observable outcome
in enough detail that a reader can tell pass from fail without
guessing.

````
3. Initialise the project.

   ```
   acme init
   ```

   **Expected.** The command exits 0 and prints
   `Initialised acme project at <path>`. A config file now exists at
   `$TOOL_HOME/config.toml`; `cat` it and confirm it names the
   current directory as the project root.
````

A command with no Expected is not a test step — it cannot pass or
fail. If you cannot articulate the outcome, you do not yet know what
the step proves. Write the Expected first, then the command that earns
it.

## Rule 3 — Platform coverage {#platform}

Commands are **POSIX-shell compatible** so the walkthrough runs on
every platform a contributor might use. Name one **primary platform**
— the environment the author actually runs the test in — and show its
form first; where output legitimately differs across platforms, add a
short portable note rather than a second full transcript.

````
5. Show the built artifact's path.

   ```
   acme where --bin
   ```

   **Expected (primary platform).** Prints
   `$PROJECT/target/acme.exe`.

   **Portable note.** On macOS and Linux there is no `.exe` suffix —
   the path ends in `/acme`. Path separators and any `stat`-style
   flags differ likewise; the trailing component and exit code are
   what the step checks.
````

Divergences worth a note are the usual ones: executable suffix, path
separators, line endings, flags on platform utilities. Keep the *check*
platform-independent (exit code, the meaningful substring) and let the
note absorb the cosmetic difference.

## Rule 4 — Exit discipline {#exit}

Every walkthrough ends with two fixed sections.

**A copy-pasteable teardown block** that removes everything the run
created — the whole point of the clean-slate setup is that one command
returns the machine to its pre-run state:

````
## Teardown

```
rm -rf "$SCRATCH"
unset SCRATCH TOOL_HOME PROJECT
```
````

Because all state lives under `$SCRATCH` (Rule 1), teardown is a single
`rm -rf` — no hunting through the real per-user directory for stray
files. If teardown needs to reach outside `$SCRATCH`, Rule 1 was
violated somewhere above.

**A "what to file if it fails" list** naming the artifacts a follow-up
session needs to diagnose a divergence, so the reader collects them
*before* running teardown destroys the evidence:

```
## What to file if it fails

- The failing step number and how the actual output differed from
  Expected (paste both).
- Verbose logs: re-run the failing command with the tool's debug
  flag or log env var set.
- The consumer-facing artifact under test (lockfile, export, manifest)
  as produced — its exact bytes, not a paraphrase.
- Platform, tool version, and shell.
```

## Summary {#summary}

- **Clean slate:** `mktemp -d` project plus an env redirect for the
  tool's per-user state; touching real state is a bug in the test.
- **Self-contained:** one file, top to bottom, nothing else needed;
  every step is a command block plus an Expected paragraph.
- **Platform coverage:** POSIX commands, primary platform first, a
  portable note where output differs cosmetically.
- **Exit discipline:** a one-command teardown of `$SCRATCH`, and a
  what-to-collect list gathered before teardown runs.
