# harvest — captured runs, written while the knowledge is hot

**This directory being empty at the end of a phase is a gate failure, not a
tidy result.** Wave 1 left its equivalent empty for an entire campaign: Phase C
listed "verification runs saved as doc fixtures; harvest cards written while
knowledge is hot" among its steps and gated only on "every marker carries a
verdict", so the step cost nothing to skip and nobody noticed until Phase G
arrived to consume it and had to be deferred outright.

Amendment **A1** exists because of that, and this campaign's Phase C exit gate
enumerates the fixtures explicitly.

## What goes here

One file per captured run, named for what it proves:

```
<package>-<what-was-run>.md      e.g. rust-ai-native-lang-floor.md
```

Each carries the command and its **real output**, verbatim — never a summary,
never a retelling:

````markdown
# rust-ai-native-lang — floor

_Captured 2026-09-XX against `packages/org.vibevm.ai-native/rust-ai-native-lang/v0.8.0/`._

```console
$ <the exact command>
<the exact output>
```

**What this is evidence for:** `spec://…#anchor`, `spec://…#anchor`
````

## Why verbatim

Two rules of this campaign depend on it. §3.2 says the discipline is verified
by *running it on itself* — the run output **is** the evidence, so a summary of
a run is not evidence of anything. And §3.1's source ordering only works if a
reader can tell which class a verdict rests on; a captured run is the one
source that cannot be confused with a document agreeing with itself.

Wave 1's F-063 is the cautionary case: five security-relevant anchors were
sealed `confirmed` on evidence that compared one spec document against another
carrying the identical error. A captured run would not have made that mistake
available.
