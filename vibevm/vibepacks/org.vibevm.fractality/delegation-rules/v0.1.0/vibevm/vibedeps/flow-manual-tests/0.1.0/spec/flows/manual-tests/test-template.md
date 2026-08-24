# Manual-test template {#root}

**Scope of this document.** The copy-ready skeleton of a manual test,
a clause-by-clause account of what each section must carry, and one
short worked example. The reasoning behind the tier lives in
[`MANUAL-TESTS-PROTOCOL.md`](MANUAL-TESTS-PROTOCOL.md); the rules the
skeleton bakes in live in [`authoring-rules.md`](authoring-rules.md).

## The skeleton {#skeleton}

Copy this into `manual-tests/<milestone>-<slug>.md` and fill each
placeholder. The section order is fixed — a reader learns it once and
navigates every test the same way.

````markdown
# <Feature> — <scenario in a few words>

**Purpose.** One paragraph: which feature, which scenario, and why
this needs a human rather than the automated suite (name the real
surface it proves — auth, per-user layout, a consumer-facing
artifact, "does the output read right?").

## Preconditions

- Tools required and their versions.
- Credentials / network access required (e.g. a real remote you can
  reach, an identity the tool can authenticate with).
- Anything that must be true of the machine before step 1.

## Setup — clean slate

```
export SCRATCH="$(mktemp -d)"
export TOOL_HOME="$SCRATCH/tool-home"   # redirect the tool's per-user state
export PROJECT="$SCRATCH/project"
mkdir -p "$TOOL_HOME" "$PROJECT"
cd "$PROJECT"
```

## Steps

1. <What this step does.>

   ```
   <command>
   ```

   **Expected.** <The observable outcome, specific enough to tell
   pass from fail: exit code, the meaningful output substring, the
   file that must now exist and what it must contain.>

2. <Next step.>

   ```
   <command>
   ```

   **Expected.** <...>

## Teardown

```
rm -rf "$SCRATCH"
unset SCRATCH TOOL_HOME PROJECT
```

## What to file if it fails

- The failing step number; the actual output beside its Expected.
- Verbose logs (re-run the failing command with the debug flag / log
  env var set).
- The consumer-facing artifact under test, as produced (exact bytes).
- Platform, tool version, shell.
````

## Clause by clause {#clauses}

- **Title.** `# <Feature> — <scenario>`. The same words as the
  filename slug, so a reader scanning the index and a reader in the
  file agree on what this is.
- **Purpose.** Says *why a human*, not just *what*. If the scenario
  could be a fast hermetic check, it belongs in the automated suite
  instead — the purpose paragraph is where you justify the tier.
- **Preconditions.** Everything that must hold before step 1, so a
  reader confirms readiness up front instead of failing halfway. Real
  credentials and network reachability belong here — they are the
  reason this is a manual test.
- **Setup — clean slate.** The two mechanisms of Rule 1: a `mktemp -d`
  project and an env redirect of the tool's per-user state into the
  scratch. Every later step operates under `$SCRATCH`; nothing touches
  the real per-user directory.
- **Steps.** Numbered, each a command block plus an **Expected**
  paragraph (Rule 2). A step with no Expected is not a step. Where
  output differs by platform, show the primary form and add a portable
  note (Rule 3).
- **Teardown.** One `rm -rf "$SCRATCH"`. Because all state lives under
  the scratch, cleanup is total and trivial (Rule 4).
- **What to file if it fails.** The evidence list, gathered *before*
  teardown destroys it. The consumer-facing artifact goes in verbatim
  — a paraphrase loses exactly the byte that mattered.

## Worked example {#example}

A first-run smoke test for an invented CLI named `acme`. It proves the
one thing the automated suite fakes: that `acme init` writes a real
config into the real per-user directory layout and reports it back the
way a human expects.

````markdown
# acme — first-run smoke test

**Purpose.** Proves that a clean install of `acme` initialises a
project and writes its per-user config on a real filesystem. The
automated suite fakes the config store; this walks the real on-disk
path and confirms the success message reads correctly to a human.

## Preconditions

- `acme` on `PATH`; `acme --version` prints 1.x.
- No other precondition — this test needs no network.

## Setup — clean slate

```
export SCRATCH="$(mktemp -d)"
export TOOL_HOME="$SCRATCH/tool-home"
export PROJECT="$SCRATCH/project"
mkdir -p "$TOOL_HOME" "$PROJECT"
cd "$PROJECT"
```

## Steps

1. Initialise a project in the empty scratch directory.

   ```
   acme init --name demo
   ```

   **Expected.** Exits 0 and prints
   `Initialised acme project 'demo'`. A config file now exists at
   `$TOOL_HOME/config.toml` and names `demo` as the active project.

2. Confirm the tool reads its own config back.

   ```
   acme status
   ```

   **Expected (primary platform).** Prints `project: demo` and
   `root: $PROJECT`. **Portable note.** The `root:` path uses the
   platform's separators; the project name is the invariant to check.

## Teardown

```
rm -rf "$SCRATCH"
unset SCRATCH TOOL_HOME PROJECT
```

## What to file if it fails

- Failing step number; actual output beside Expected.
- `ACME_LOG=debug acme init --name demo` output.
- The produced `$TOOL_HOME/config.toml`, verbatim.
- Platform, `acme --version`, shell.
````

## Summary {#summary}

- Fixed section order: Title, Purpose, Preconditions, Setup, Steps,
  Teardown, What to file if it fails.
- Purpose justifies the tier; Preconditions gate the run; Setup
  isolates it; every Step carries an Expected.
- Teardown is one `rm -rf "$SCRATCH"`; the failure list is collected
  before teardown runs.
- Fill the skeleton, do not reinvent it — a reader who knows one test
  knows them all.
