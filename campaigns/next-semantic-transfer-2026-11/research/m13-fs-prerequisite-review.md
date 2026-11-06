# D1 shared filesystem prerequisite — candidate review

**Accepted prerequisite in `f2ca79eee29b80fa612bb4fd17215d9d0daef398`; not whole-D1 acceptance.** The current atom implements existing-only store/lock observation, physical store identity, retained-image synchronization and the demonstrated Unix directory-sync correction. Migration journal/application/recovery and CLI/MCP effect grants remain separate work.

Root inspected all production changes and meaningful new tests. Linux directory sync first failed with OS9 EBADF on the held descriptor while a readable handle to fixed `.` under the same capability succeeded. The fix checks physical identity before and after real sync; it never follows a display pathname. Existing-only observation keeps absence distinct from overlap/link/file/I/O refusal and does not create missing state. Creating and noncreating locks share the incumbent name/OS-lock/identity-retry owner.

Retained-image confirmation compares the caller's actual single-link snapshot around a real file flush and reports parent durability. Matching bytes alone are not a flush receipt. Windows uses a noncreating/nontruncating write-capable file handle. This is a finite observation window, not future immutability, whole-change atomicity or hostile-ABA protection. Thread-local test injection is cleared by fixture RAII; no production default changes.

Observed gates so far:11new/helper cases and21transaction cases passed (overlapping); strict Clippy passed. Initial Conform15new findings classified test helper unwraps as domain code; helpers were changed to return Result, with behavioural assertions preserved. Final gate/hash review passed; all13committed blobs match the receipt. Existing11unsafe findings must not be presented as zero findings.

## Separate publication capability decision for the next domain atom

Main's existing common writer already accepts both an optional deterministic stage name and an optional owned receipt. Public wrappers expose these separately. Merely combining the parameters is insufficient: the deterministic reuse branch currently validates a stored stage but drops its file handle and supplies no retained owned stage; the owned result requires actual retained receipt evidence. A domain using restart-safe staging and truthful possible-publication reporting must reconcile that branch explicitly, with real reuse/foreign-stage/rename-fault tests. Do not add a wrapper that reports success without a receipt, and do not globally change unrelated non-owned callers to hide the gap.

This observation is source-backed, not a new runtime failure. It is deliberately outside the current shared observation/sync atom. The next D1 design must choose the existing stronger owner plus the necessary deterministic-reuse extension, or demonstrate an equivalent complete ownership boundary, before publication code is authorized.

## Late special-file admission discriminator

An ordinary file replaced with a FIFO can block a blocking open before the regular-file verification runs. A bounded current-tree test reproduced this for both the owned snapshot and retained-sync entrypoints; it guaranteed a nonblocking RDWR peer and joined both workers before failing. The target already has a nonblocking source-read options owner. Reuse that flag mechanism for these opens while retaining separate source and mutation validation laws; do not change all writer options or substitute the hardlink-permissive source reader for an owned mutation snapshot. Both snapshot/retained-sync paths and lock-name recheck have actual RED→GREEN evidence; final relevant regression/static gates passed.

## Final acceptance evidence

Root reviewed all production changes and exact tests. Transaction23, bounded17, source8, stream13 and doctests4 pass; scopes overlap. Strict Clippy/fmt pass, Conform11existing unsafe findings/0new. Sequential specmap/check9112units5357items4514edges,0orphans0unresolvedhost44existingwarnings. No generator/schema changes require codegen here. No temporary worktree, external dependency or public push.

The FIFO tests guaranteed RDWR-peer unblock and joined both workers before reporting failure; no orphan was left. Root approved extending only read-admission options after actual RED, reusing the existing source option body exactly. Source hardlinks still work; owned mutation snapshots still reject them. Intentional OS lock waiting remains blocking. Native Windows/macOS were not executed; finite observation/ABA/immutability limitations remain explicit. Full migration journal/apply/recovery is the next atom and no old113 completion credit is added here.
