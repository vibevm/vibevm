# Authoring rules for manual tests {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** The four rules every manual test must
satisfy, each with a short worked fragment. @status:impl/done

@fact:sibling-document-pointers The rationale for the tier
lives in [`MANUAL-TESTS-PROTOCOL.md`](MANUAL-TESTS-PROTOCOL.md); the
copy-ready skeleton that bakes these rules in lives in
[`test-template.md`](test-template.md). @status:impl/done

@fact:A-WALKTHROUGH-THAT-BREAKS-ANY-OF-THESE-IS-A-BUG-IN-THE-TEST A walkthrough that breaks any of these is a bug in the test, not a
property of the product it exercises. @status:impl/done

## Rule 1 — Clean slate is mandatory {#clean-slate}

@fact:EVERY-RUN-STARTS-FROM-NOTHING-AND-TOUCHES-NO-REAL-USER-STATE Every run starts from nothing and touches no real user state. @status:impl/done

@fact:two-mechanisms-lead Two
mechanisms, always both: @status:impl/done

- @fact:MECHANISM-A-SCRATCH-PROJECT **A scratch project** created fresh with `mktemp -d`, so the working
  tree the test operates on is disposable and unique per run. @status:impl/done
- @fact:MECHANISM-AN-ENVIRONMENT-REDIRECT **An environment redirect** pointing the tool's per-user state
  (cache, config, home) into that scratch, so the real per-user
  directory is never read from or written to. @status:impl/done

```
export SCRATCH="$(mktemp -d)"
export TOOL_HOME="$SCRATCH/tool-home"   # redirect the tool's per-user state
export PROJECT="$SCRATCH/project"
mkdir -p "$TOOL_HOME" "$PROJECT"
cd "$PROJECT"
```

@fact:now-every-command-reads-and-writes-under-the-scratch Now every command the tool runs reads and writes under `$SCRATCH`. @status:impl/done

@fact:nothing-under-the-real-home-is-at-risk Nothing under the developer's real home is at risk, and two runs on
the same machine cannot collide. @status:impl/done

@fact:A-TEST-THAT-MUTATES-REAL-USER-STATE-IS-A-BUG-IN-THE-TEST **A test that mutates real user state is a bug in the test**, even if
every step passes — because the next contributor's run inherits that
mutation and the walkthrough is no longer reproducible. @status:impl/done

@fact:A-STEP-THAT-SEEMS-TO-NEED-REAL-STATE-MEANS-FIX-THE-REDIRECT If a step
seems to *need* the real per-user directory, the redirect is wrong or
incomplete; fix the redirect. @status:impl/done

## Rule 2 — Self-contained walkthrough {#self-contained}

@fact:A-READER-OPENS-ONE-FILE-AND-NEEDS-NOTHING-ELSE A reader opens exactly one file, executes it top to bottom, and needs
nothing else — no companion doc, no tribal knowledge, no "ask whoever
wrote this". @status:impl/done

@fact:THE-FILE-NAMES-ITS-OWN-PRECONDITIONS-AND-PROVIDES-ITS-OWN-SETUP The file names its own preconditions and provides its own
setup. @status:impl/done

@fact:EVERY-STEP-IS-A-COMMAND-BLOCK-PLUS-AN-EXPECTED-PARAGRAPH Every step is a **command block plus an "Expected" paragraph**. @status:impl/done

@fact:THE-COMMAND-IS-COPY-PASTEABLE-THE-EXPECTED-TELLS-PASS-FROM-FAIL The
command is copy-pasteable; the Expected states the observable outcome
in enough detail that a reader can tell pass from fail without
guessing. @status:impl/done

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

@fact:A-COMMAND-WITH-NO-EXPECTED-IS-NOT-A-TEST-STEP A command with no Expected is not a test step — it cannot pass or
fail. @status:impl/done

@fact:if-you-cannot-articulate-the-outcome-you-do-not-know-what-it-proves If you cannot articulate the outcome, you do not yet know what
the step proves. @status:spec/done

@fact:WRITE-THE-EXPECTED-FIRST-THEN-THE-COMMAND Write the Expected first, then the command that earns
it. @status:impl/done

## Rule 3 — Platform coverage {#platform}

@fact:COMMANDS-ARE-POSIX-SHELL-COMPATIBLE Commands are **POSIX-shell compatible** so the walkthrough runs on
every platform a contributor might use. @status:impl/done

@fact:NAME-ONE-PRIMARY-PLATFORM-AND-ADD-A-PORTABLE-NOTE Name one **primary platform**
— the environment the author actually runs the test in — and show its
form first; where output legitimately differs across platforms, add a
short portable note rather than a second full transcript. @status:impl/done

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

@fact:divergences-worth-a-note-lead Divergences worth a note are the usual ones: @status:impl/done

- @fact:DIVERGENCE-EXECUTABLE-SUFFIX executable suffix, @status:impl/done
- @fact:DIVERGENCE-PATH-SEPARATORS path
  separators, @status:impl/done
- @fact:DIVERGENCE-LINE-ENDINGS line endings, @status:impl/done
- @fact:DIVERGENCE-FLAGS-ON-PLATFORM-UTILITIES flags on platform utilities. @status:impl/done

@fact:KEEP-THE-CHECK-PLATFORM-INDEPENDENT Keep the *check*
platform-independent (exit code, the meaningful substring) and let the
note absorb the cosmetic difference. @status:impl/done

## Rule 4 — Exit discipline {#exit}

@fact:EVERY-WALKTHROUGH-ENDS-WITH-TWO-FIXED-SECTIONS Every walkthrough ends with two fixed sections. @status:impl/done

@fact:SECTION-A-COPY-PASTEABLE-TEARDOWN-BLOCK **A copy-pasteable teardown block** that removes everything the run
created — the whole point of the clean-slate setup is that one command
returns the machine to its pre-run state: @status:impl/done

````
## Teardown

```
rm -rf "$SCRATCH"
unset SCRATCH TOOL_HOME PROJECT
```
````

@fact:TEARDOWN-IS-A-SINGLE-RM-RF Because all state lives under `$SCRATCH` (Rule 1), teardown is a single
`rm -rf` — no hunting through the real per-user directory for stray
files. @status:impl/done

@fact:TEARDOWN-REACHING-OUTSIDE-THE-SCRATCH-MEANS-RULE-ONE-WAS-VIOLATED If teardown needs to reach outside `$SCRATCH`, Rule 1 was
violated somewhere above. @status:impl/done

@fact:SECTION-A-WHAT-TO-FILE-IF-IT-FAILS-LIST **A "what to file if it fails" list** naming the artifacts a follow-up
session needs to diagnose a divergence, so the reader collects them
*before* running teardown destroys the evidence: @status:impl/done

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

- @fact:SUM-CLEAN-SLATE **Clean slate:** `mktemp -d` project plus an env redirect for the
  tool's per-user state; touching real state is a bug in the test. @status:impl/done
- @fact:SUM-SELF-CONTAINED **Self-contained:** one file, top to bottom, nothing else needed;
  every step is a command block plus an Expected paragraph. @status:impl/done
- @fact:SUM-PLATFORM-COVERAGE **Platform coverage:** POSIX commands, primary platform first, a
  portable note where output differs cosmetically. @status:impl/done
- @fact:SUM-EXIT-DISCIPLINE **Exit discipline:** a one-command teardown of `$SCRATCH`, and a
  what-to-collect list gathered before teardown runs. @status:impl/done
