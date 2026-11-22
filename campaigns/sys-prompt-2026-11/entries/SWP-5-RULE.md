# SWP-5-RULE

Planned at commit `9b197755e`. Line numbers are hints from that commit; find each place by the quoted text or the item name.

**Outcome.** The working-rules protocol says full-context workers are started through the project's launcher

**Write.** `vibevm/vibepacks/org.vibevm.world/working-rules/v1.0.12/rules/protocol.md`

**Check.** `test "$(grep -c 'launcher; nobody assembles a system prompt by hand.' vibevm/vibepacks/org.vibevm.world/working-rules/v1.0.12/rules/protocol.md)" = 1`

**Test.** No cargo test: the outcome is one rule sentence in the package source. Before the edit the grep count is 0 (verified at 9b197755e), so the check fails; after, it is 1.

**Read first.**
- vibevm/vibepacks/org.vibevm.world/working-rules/v1.0.12/rules/protocol.md, section "Bounded workers and independent verification" (the "Full project boot" bullet)

**Steps.**
1. Fast-forward the worktree to the lead's branch first.
2. In the "Full project boot" bullet of section "Bounded workers and independent verification", replace the exact line (it occurs once)
```
  Do not attach a quiet clause that forbids the boot this mode requires.
```
with
```
  Do not attach a quiet clause that forbids the boot this mode requires.
  A worker that needs full project context is started through the project's
  launcher; nobody assembles a system prompt by hand.
```
3. Run the check. Edit only this file; the installed copy under vibevm/vibedeps and vibe.lock are derived.

**Decisions already made.**
- The sentence goes in the "Full project boot" bullet, worded exactly: "A worker that needs full project context is started through the project's launcher; nobody assembles a system prompt by hand."
- The package version stays v1.0.12 (earlier rule additions to this version, e.g. 0940fad9d, did the same).
- After the executor's commit lands, the lead re-installs the package on main as a separate `chore(deps)` commit (as 4bf8f2f41 did), which refreshes the vibedeps copy, the vibe.lock content_hash and the generated boot lane. This is not an executor step: an isolated worktree lacks the install state.

**Not in this entry.** Not in this entry: naming `vibe agent` or Claude-specific flags in the rule, editing decomposing.md or lead-bounded.md, bumping the package version, or touching vibedeps/vibe.lock. A lock or vibedeps drift reported by a gate after this commit is expected until the lead's chore(deps) reinstall.

**Spec edit (apply as a step of this entry).**

none

Entries that write the same file run one after another, never in parallel.
