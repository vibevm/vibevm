# Authoring rules for manual tests {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** The four rules every manual test must
satisfy, each with a short worked fragment. @impl/done

##sibling-document-pointers The rationale for the tier
lives in [`MANUAL-TESTS-PROTOCOL.md`](MANUAL-TESTS-PROTOCOL.md); the
copy-ready skeleton that bakes these rules in lives in
[`test-template.md`](test-template.md). @impl/done

##A-WALKTHROUGH-THAT-BREAKS-ANY-OF-THESE-IS-A-BUG-IN-THE-TEST A walkthrough that breaks any of these is a bug in the test, not a
property of the product it exercises. @impl/done

## Rule 1 — Clean slate is mandatory {#clean-slate}

##EVERY-RUN-STARTS-FROM-NOTHING-AND-TOUCHES-NO-REAL-USER-STATE Every run starts from nothing and touches no real user state. @impl/done

##two-mechanisms-lead Two
mechanisms, always both: @impl/done

- ##MECHANISM-A-SCRATCH-PROJECT **A scratch project** created fresh with `mktemp -d`, so the working
  tree the test operates on is disposable and unique per run. @impl/done
- ##MECHANISM-AN-ENVIRONMENT-REDIRECT **An environment redirect** pointing the tool's per-user state
  (cache, config, home) into that scratch, so the real per-user
  directory is never read from or written to. @impl/done

```
export SCRATCH="$(mktemp -d)"
export TOOL_HOME="$SCRATCH/tool-home"   # redirect the tool's per-user state
export PROJECT="$SCRATCH/project"
mkdir -p "$TOOL_HOME" "$PROJECT"
cd "$PROJECT"
```

##now-every-command-reads-and-writes-under-the-scratch Now every command the tool runs reads and writes under `$SCRATCH`. @impl/done

##nothing-under-the-real-home-is-at-risk Nothing under the developer's real home is at risk, and two runs on
the same machine cannot collide. @impl/done

##A-TEST-THAT-MUTATES-REAL-USER-STATE-IS-A-BUG-IN-THE-TEST **A test that mutates real user state is a bug in the test**, even if
every step passes — because the next contributor's run inherits that
mutation and the walkthrough is no longer reproducible. @impl/done

##A-STEP-THAT-SEEMS-TO-NEED-REAL-STATE-MEANS-FIX-THE-REDIRECT If a step
seems to *need* the real per-user directory, the redirect is wrong or
incomplete; fix the redirect. @impl/done

## Rule 2 — Self-contained walkthrough {#self-contained}

##A-READER-OPENS-ONE-FILE-AND-NEEDS-NOTHING-ELSE A reader opens exactly one file, executes it top to bottom, and needs
nothing else — no companion doc, no tribal knowledge, no "ask whoever
wrote this". @impl/done

##THE-FILE-NAMES-ITS-OWN-PRECONDITIONS-AND-PROVIDES-ITS-OWN-SETUP The file names its own preconditions and provides its own
setup. @impl/done

##EVERY-STEP-IS-A-COMMAND-BLOCK-PLUS-AN-EXPECTED-PARAGRAPH Every step is a **command block plus an "Expected" paragraph**. @impl/done

##THE-COMMAND-IS-COPY-PASTEABLE-THE-EXPECTED-TELLS-PASS-FROM-FAIL The
command is copy-pasteable; the Expected states the observable outcome
in enough detail that a reader can tell pass from fail without
guessing. @impl/done

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

##A-COMMAND-WITH-NO-EXPECTED-IS-NOT-A-TEST-STEP A command with no Expected is not a test step — it cannot pass or
fail. @impl/done

##if-you-cannot-articulate-the-outcome-you-do-not-know-what-it-proves If you cannot articulate the outcome, you do not yet know what
the step proves. @spec/done

##WRITE-THE-EXPECTED-FIRST-THEN-THE-COMMAND Write the Expected first, then the command that earns
it. @impl/done

## Rule 3 — Platform coverage {#platform}

##COMMANDS-ARE-POSIX-SHELL-COMPATIBLE Commands are **POSIX-shell compatible** so the walkthrough runs on
every platform a contributor might use. @impl/done

##NAME-ONE-PRIMARY-PLATFORM-AND-ADD-A-PORTABLE-NOTE Name one **primary platform**
— the environment the author actually runs the test in — and show its
form first; where output legitimately differs across platforms, add a
short portable note rather than a second full transcript. @impl/done

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

##divergences-worth-a-note-lead Divergences worth a note are the usual ones: @impl/done

- ##DIVERGENCE-EXECUTABLE-SUFFIX executable suffix, @impl/done
- ##DIVERGENCE-PATH-SEPARATORS path
  separators, @impl/done
- ##DIVERGENCE-LINE-ENDINGS line endings, @impl/done
- ##DIVERGENCE-FLAGS-ON-PLATFORM-UTILITIES flags on platform utilities. @impl/done

##KEEP-THE-CHECK-PLATFORM-INDEPENDENT Keep the *check*
platform-independent (exit code, the meaningful substring) and let the
note absorb the cosmetic difference. @impl/done

## Rule 4 — Exit discipline {#exit}

##EVERY-WALKTHROUGH-ENDS-WITH-TWO-FIXED-SECTIONS Every walkthrough ends with two fixed sections. @impl/done

##SECTION-A-COPY-PASTEABLE-TEARDOWN-BLOCK **A copy-pasteable teardown block** that removes everything the run
created — the whole point of the clean-slate setup is that one command
returns the machine to its pre-run state: @impl/done

````
## Teardown

```
rm -rf "$SCRATCH"
unset SCRATCH TOOL_HOME PROJECT
```
````

##TEARDOWN-IS-A-SINGLE-RM-RF Because all state lives under `$SCRATCH` (Rule 1), teardown is a single
`rm -rf` — no hunting through the real per-user directory for stray
files. @impl/done

##TEARDOWN-REACHING-OUTSIDE-THE-SCRATCH-MEANS-RULE-ONE-WAS-VIOLATED If teardown needs to reach outside `$SCRATCH`, Rule 1 was
violated somewhere above. @impl/done

##SECTION-A-WHAT-TO-FILE-IF-IT-FAILS-LIST **A "what to file if it fails" list** naming the artifacts a follow-up
session needs to diagnose a divergence, so the reader collects them
*before* running teardown destroys the evidence: @impl/done

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

- ##SUM-CLEAN-SLATE **Clean slate:** `mktemp -d` project plus an env redirect for the
  tool's per-user state; touching real state is a bug in the test. @impl/done
- ##SUM-SELF-CONTAINED **Self-contained:** one file, top to bottom, nothing else needed;
  every step is a command block plus an Expected paragraph. @impl/done
- ##SUM-PLATFORM-COVERAGE **Platform coverage:** POSIX commands, primary platform first, a
  portable note where output differs cosmetically. @impl/done
- ##SUM-EXIT-DISCIPLINE **Exit discipline:** a one-command teardown of `$SCRATCH`, and a
  what-to-collect list gathered before teardown runs. @impl/done
