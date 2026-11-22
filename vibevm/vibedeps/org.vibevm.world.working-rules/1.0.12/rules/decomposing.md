# Preparing work for a bounded lead

This page is for an open lead who turns a large goal into a completion list
that a bounded lead (`lead-bounded.md`) will execute. A bounded lead is good at
finishing a well-shaped entry and bad at deciding what the entry should be. Do
all deciding here, before the hand-off. The bounded lead never decomposes,
never adds entries, and never reads the material you decomposed from.

## Why bounded leads spiral

These are the observed causes. Each rule below removes one of them.

- **Open denominators.** A goal like "deliver the useful rest of the design"
  has no end the lead can see. The lead keeps finding more work, because more
  work exists.
- **Structure that invites completion.** Specs with ontologies, crosswalks,
  historical IDs and status columns read as "make this whole". The lead fills
  gaps that no user will ever notice.
- **Choices inside the entry.** "Decide whether", "pick a format", "consider".
  Each choice becomes research, then a document, then a new entry.
- **Checks that cannot fail or cannot pass.** Hedged acceptance ("no X is
  claimed"), checks that already pass, and checks that need the whole gate.
  The lead cannot tell done from not done, so it re-verifies forever.
- **Too much context.** Long rules, the whole plan, the boot lane. The lead
  treats every sentence as an obligation.

## Before you write any entry

1. **Inventory against the code, not against the old plan.** For every
   historical item, find out what is true today, with a file path or a test
   name as evidence. Old status labels are claims, not facts.
2. **Recover the user intent, then restate it for today's architecture.** An
   old design often has the right intent and the wrong form. Rewrite it in
   terms of today's commands, crates and data files. Drop only items with no
   behaviour a user would notice, or items that today's product already
   serves (name what serves them).
3. **Group into capabilities.** A capability is something a release note
   could announce. Entries come from capabilities, never from historical IDs.
   Historical IDs stay in a mapping table for you. The bounded lead never sees
   it.
4. **Answer the product questions yourself or get the owner's answer before
   hand-off.** A question left in the list becomes a loop.

## Writing an entry

5. **One entry, one observable behaviour.** The `Outcome` line says what a
   user or a test can see, in one sentence. Not "implement X support".
6. **Name the test before it exists.** Give the test file and test function
   name, and the exact check command filtering to it (for example
   `cargo test -p vibe-install --test uninstall configured_entry_removed`).
   Before the work, the command runs zero tests or fails. After, it passes. Run
   it yourself once to confirm it does not already pass.
7. **Pre-make every decision.** Command names, flags, output fields, error
   text, file locations, type names, where the test fixture lives. Write them
   into the entry's note or the packet. If you cannot decide, the entry is not
   ready.
8. **Write area as globs, as narrow as possible.** If the change needs a spec
   edit, either make the spec edit yourself before hand-off or name the spec
   file in the write area with the exact unit to add. Never "update specs as
   needed".
9. **Size: at most about 500 changed lines and one crate plus its tests.**
   Split larger work into entries that each leave the build green and each
   show something.
10. **State the tempting overreach.** One line in the note: "Not in this
    entry: …" naming the most likely scope creep (a refactor, a second
    command, a migration of other callers).
11. **No entry whose outcome is a document, an inventory, a receipt, a
    re-qualification of working behaviour, or spec completeness.**
12. **No negative or hedged acceptance.** "Refuses with error E" is fine.
    "Does not claim X" is not an outcome.

## Shaping the list

13. **Order by real prerequisites only.** An entry depends on another only if
    its check cannot pass without the other's code. Independent entries get
    disjoint write areas so they can run in parallel.
14. **Keep the open list short**: about 15 entries per goal. Put the rest in
    later goals with their own lists. A list that looks endless is endless.
15. **Make it executable without the source material.** The bounded lead
    reads the entry, the packet template and the files it names, nothing
    else. If an entry needs the design document to make sense, move the
    needed sentence into the entry.
16. **Pass the work through a skeptic before hand-off.** Have someone else try
    each entry cold: is any decision left open? Can the check fail before and
    pass after? Is the write area enough? Fix what they find. Do not hand off
    an entry that failed this.

## What a skeptic finds in a first decomposition

Measured on ADVANCED-ARCHITECTURE (2026-11-22). A strong model decomposed 17
capabilities into 62 entries. A second strong model then read every entry
cold, as the weak lead would, and found 3 to 9 problems in **every** capability
(82 in total). By kind: wrong code facts 19, hidden prerequisites 12, vague
outcomes 10, write areas too small 9, checks that already pass or cannot fail
10, open decisions 4, invitations to scope creep 3, other 20. So rule 16 is
not optional. The recurring traps, with the fix for each:

19. **A name filter that matches nothing passes.** `cargo test <filter>` exits
    0 when no test matches, so a check for a test that does not exist yet
    passes before any work. Run checks through a wrapper that fails when no
    test ran (in vibevm: `bash tools/cargo-test-ran.sh <cargo test args>`), or
    pin the count. Never chain an already-passing test into a check with
    `&&`: the entry then looks "already true".
20. **Code facts drift from memory.** Quote current signatures, field types
    and variant lists from the code, not from a design document. Verify each
    one before hand-off.
21. **Hidden prerequisites.** Name them in the entry. In vibevm these are:
    gitignored tool binaries a fresh worktree lacks (`tools/jtd-codegen`);
    generated files (`specmap.json`, generated wire types); every literal of a
    struct that gains a field (grep `Name {` across the workspace); every
    caller of a renamed helper; every file of a moved module tree.
22. **File-length gate.** Run `wc -l` on every file an entry grows. Near the
    limit, the entry puts new code into a new child module.
23. **Spec edits.** The spec file is in the write area and a step applies the
    edit. The entry that first emits a spec marker also creates its anchor.
    A fact becomes done only in the entry that makes all of it true.
24. **Cite by name, not by line.** Line ranges rot as soon as an earlier entry
    lands in the same file. Pin the brief to the planning commit and treat
    line numbers as hints.
25. **Exact bytes.** Give literal text (doc comments, TOML, fixtures) in fenced
    blocks, unescaped. Never use "…" in text that must be reproduced.
26. **Lint gates.** Refactors leave dead fields and test-only imports. Say how
    the entry handles them when the gate runs lints with warnings as errors.
27. **State words.** Do not use `blocked`, `deferred` or `accepted` for data
    inside an entry. They collide with the list's states.
28. **Machine paths.** No absolute paths or machine settings in checks.
29. **Generated files need project state that task worktrees lack.** A
    regenerated map or index (in vibevm: `specmap.json`, which needs a
    gitignored authority file written by an install) cannot be rebuilt in an
    isolated worktree. Plan it as a lead step on the main tree after the
    executor's commit is applied, not as an executor step.
30. **Isolated worktrees start from the remote default branch,** not from the
    local branch the lead works on. The first step of every packet
    fast-forwards the worktree to the lead's branch, or the brief is missing.
31. **Compatibility with released clients.** When an entry changes a document
    that already released versions read (a release manifest, a registry
    record), check how the old reader reacts. A strict old reader turns an
    additive change into a break; the tolerant reader ships one release
    earlier.
32. **Questions still come up.** In the second round, 11 of 18 capabilities
    raised a question only the owner or the planner could answer. Answer them
    before hand-off and write each answer into the plan; an entry whose
    question is open waits as `blocked` with the question in its note.
33. **Repository gates that focused checks never run.** A focused check proves
    the behaviour, not the repository's structural gates (in vibevm:
    `conform`, which wants a spec edge and a doctest on every new public item
    and a 600-line file budget). Name those requirements in the entry, or the
    first full gate after the list fails on entries already accepted.

## After hand-off

17. Owner-directed changes go into the list as edits (rule 3 of
    `lead-bounded.md`); you do not re-plan behind the bounded lead's back.
18. Entries that come back `blocked` or `deferred` come back to you. Reshape
    them here. Do not ask the bounded lead to reshape them.
