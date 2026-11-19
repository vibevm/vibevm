# Owner-directed clean dependency regeneration

Date2026-11-19. Exact selected root: `vibevm/vibepacks/org.vibevm.fractality/delegation-rules/v1.0.0/vibevm/vibedeps`. The owner explicitly requested committing its current contents, deleting the contents, rebuilding through Vibe, comparing immediate package-directory counts, committing the result and continuing. An actual clarification selected this problematic nested root only, not host vibedeps or other projects.

## Preservation and execution

Current Git-visible dependency contents, including newly present source files, were committed in `d7e21620bd33b6cbef2f83062588c117fb452e87`. Ignored build output was not forced into Git. A separate complete backup of5307 regular files /662214849 bytes, including ignored output, was copied and SHA256-verified before deletion at `/data/git/vibevm-router-reconstruction-20261119/owner-clean-full-before`.

The selected root's contents were cleared through fd-anchored no-follow traversal under a coordination lock and35 file locks; the root directory itself was retained. No other vibedeps root was cleared. An initial preflight stopped before deletion on inaccessible service-process metadata; no process permissions were changed. Inaccessible services were recorded rather than claimed inspected. No active Cargo/rustc or accessible selected-root consumer was found; the architect reported no own reservation/readers. No processes were stopped.

The empty tree was regenerated through current-source Vibe's ordinary pinned offline `reinstall --force` for this nested root only, in a netless namespace with isolated HOME/settings and a read-only canonical local registry/toolchain. This was not whole-host reinstall. Exact lock hashes were enforced by the existing owner, not inferred or hand-edited.

## Result

| Check | Actual result |
|---|---|
| Immediate package directories before |31|
| Immediate package directories after |31|
| Added package directories |none|
| Removed package directories |none|
| Package-directory names |identical|
| Locked packages/source hashes |31 verified|
| Recorded payload file SHA256 |595 verified, zero mismatches|
| Mandatory build callbacks |31 successful|
| Optional MCP registration callbacks |31 failed without explicit scope; registration NOT performed|
| Regeneration command |exit0; callbacks3m56s|
| Git-visible regenerated dependency bytes |unchanged from preservation commit|

The count comparison concerns only direct child package directories under vibedeps, not nested version directories or recursive directory totals. Neither count nor names changed; the owner's conditional `~/temp/MUST-READ.md` report was therefore not required and was not created/overwritten.

Since the regenerated Git-visible tree is byte-identical to the preservation commit, there is no dependency payload diff to commit a second time. This result document is the committed record of the newly executed clean regeneration. Ignored build output remains uncommitted.

## Evidence and boundaries

Evidence root `/data/git/vibevm-router-reconstruction-20261119/`: `OWNER-CLEAN-REGENERATE-BEFORE.json`, `OWNER-CLEAN-FULL-PRESERVATION.json`, `OWNER-CLEAN-RETIRING.json`, `OWNER-CLEAN-DELETE-RESULT.json`, `OWNER-CLEAN-REGENERATE-RESULT.json`, and `owner-clean-regenerate.log`.

This explicit clean reconstruction addresses the owner's selected present-state task. It does not retroactively prove preservation of unknown untracked data before the earlier refetch; that historical limitation remains disclosed in `ROUTER-LIFECYCLE-REPAIRS-20261119.md`. No loss of such data was established. Other canonical roots, host-derived refresh and native Hermes/Pi parity retain their own acceptance boundaries. No external push or publication was performed.
