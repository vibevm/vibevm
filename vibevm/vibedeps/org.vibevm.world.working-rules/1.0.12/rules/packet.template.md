# Task packet template

Copy this file, fill every field, and hand the result to one executor together
with `worker.md`, the executor page beside this template. A packet with an empty field is not ready.
The lead writes packets; an executor never writes or edits one.

```
Packet:  P-<number>
Tree:    <absolute path of the worktree the executor works in>
Branch:  <branch already checked out there>
Budget:  <minutes of wall clock, e.g. 40>

Goal:    <one sentence: what is true after this packet that is not true now>

Read:    <exact file paths, at most 8>
Write:   <exact file paths, at most 5; nothing else may change>

Steps:   1. <concrete edit: file, function, what to add or remove>
         2. <...>
         (at most 6 steps; no step says "decide", "choose", "if appropriate")

Check:   <one shell command; exit code 0 means done>
Commit:  <full commit message: type(scope): subject, blank line, 2-4 sentences>
```

## Sizing rules for the lead

- One packet changes one crate, at most 5 files, and fits one context without
  compaction. If it does not, split it before dispatch, not during.
- `Check` is a focused command (`cargo test -p <crate> <filter>`,
  `cargo check -p <crate>`), never the workspace gate.
- Every choice is made in the packet. If the lead cannot write the steps without
  "decide", the task needs a strong session, not an executor.
- Packets that run at the same time have disjoint `Write` lists and separate
  worktrees.
- The lead does not put owner quotations, plans, history or other packets into a
  packet.
