# Source main reconciliation before authorized milestone pushes

Date2026-11-19. Owner authorized source pushes at substantial reviewed builder task boundaries to GitHub and GitVerse, then chose **reconcile main first**, not a new publication branch. This is a preflight/decision record, not a claim of publication or permission to rewrite history.

## Observed immutable endpoints

- GitHub main: `b76de1197ecf8616bd392e6bd2f48f2382132c2e`.
- GitVerse main: `6d2f6e59be3bf55ac85fb2db67ad4e80b6925d79`.
- Both next: `07edf21a13f59b6b1e4813aa6452ae45dcff4090`.
- Both release1.0.0: `44a0d67e8ec687e8d946fe4953790d7bd7d9ca3f`.
- main common ancestor: `b6659978453f50e6d1d4d99626d70b980a2c5847`.

Raw main history difference is547 GitHub-only /127 GitVerse-only commits. `git cherry main <GitVerse-main>` marks all127 as patch-equivalent; there are no merge commits in that GitVerse-only range. Cherry-pick-aware difference is420/0. This establishes duplicate patch history, not automatically equivalent final trees or proof that every old change remains desirable. `git merge-tree --write-tree --name-only` produced323 conflict paths without changing any branch/worktree; ordinary automatic merge is not a safe default.

Builder task descends from GitHub main but not from GitVerse main or configured next. Initial main-to-builder publication range was176 commits/622 files, much broader than the last native-agent milestone. A reviewed final atom does not automatically accept every preceding unpublished commit. Keep the existing original task/campaign acceptance history for classification.

## Exact duplicate-tree witness and approved local join

Further comparison found GitVerse main's complete tree is exactly equal to GitHub ancestor `218996b360b4ea1a669f16f933f1437b524ae6b0` (both tree `fa452f66315490734b4115326cdf34d6575645b3`). Range comparison pairs all127 historical commits; metadata differs in author address and some referenced commit IDs, while endpoint file trees are identical. Thus no old remote file content needs conflict resolution to preserve that already incorporated tree.

The owner explicitly approved joining both histories with unchanged current GitHub main tree. Local main now points to merge `8678f883d263ea02d7eb8809de7f35931fc82abf`, parents exactly the two observed remote main OIDs, tree exactly `2e89ce0fde287697410fa13d21b12cfedaeab03a` (unchanged current GitHub main). Compare-and-swap ref update preserved the task working tree and both parent histories. This joins history only; unpublished builder implementation has NOT been integrated or published by this commit.

Before fanout, tag preflight found GitVerse `v1.0.8`, `v1.0.9`, `v1.0.10`, `v1.0.11` differ from local/GitHub; all other tags and configured next/release refs matched. No tag rewrite is authorized. The ordinary mirror tool remains non-force: any tag drift must be reported as failure, not resolved by rewriting tags or claiming a fully green rollout. Actual standard mirror task ba3ok4e7l exited1 solely for those four existing GitVerse tag conflicts; both main pushes succeeded. Independent `git ls-remote` verifies GitHub main and GitVerse main exactly `8678f883d263ea02d7eb8809de7f35931fc82abf`. Configured next/release pushes and GitHub tags succeeded; no force or tag rewrites occurred. Report main reconciliation published on both hosts, overall fanout NOT green, and builder implementation still unpublished pending its separate integrated-range review.

## Non-negotiable limits

No force push, rewrite-ref lease, hard reset, deletion of old main, blind ours merge, mass conflict resolution, mirroring of unrelated tags or ad hoc target bypass. No mirror manifest changes merely to make a task branch publish. Use PROP-016 and the existing mirror owner; source publication is not a release or package publication. Never publish dirty working files or local secrets.

## Finite reconciliation route

1. Recheck the two remote OIDs immediately before any mutation. Preserve both histories and exact baseline refs; do not operate in the peer checkout or copied worktree registrations.
2. Classify GitVerse-only changes using the established patch-equivalent counterparts and identify any later superseding/reverting work on the accepted GitHub line. Use existing accepted requirements/history, not an assumption that old commits win or that hash difference is missing functionality.
3. Produce an explicit reconciliation candidate that retains both parents/history. If all remote-only effects are duplicate or deliberately superseded, a history-joining commit with the accepted main tree may be appropriate; this needs explicit review of that disposition, not a silent `-s ours` shortcut. If genuine missing effects exist, integrate only those finite effects and test their affected owners. Do not manually resolve323 conflicts wholesale just to join duplicate histories.
4. Review the broad unpublished builder range by meaningful accepted campaign boundaries, exclude unrelated/WIP changes, and bind the exact integrated source commit intended for rollout. Do not infer acceptance from branch ancestry alone.
5. Prepare actual affected tests and non-destructive merge candidate review before changing maintained main. Any genuinely irreversible/sensitive action remains owner approval territory; current source-push grant is not a history-rewrite grant.
6. Once a reviewed main descends from BOTH remote main OIDs, ordinary source push can be fast-forward on both hosts. Preflight every configured mirror ref/tag first; the current mirror tool fans the manifest set, so unrelated refs must already be equal or separately authorized, not silently rolled out.
7. Execute the existing fanout once, verify resulting remote main OIDs on GitHub and GitVerse, report each host separately and retain partial-failure evidence. Do not report both published if only one succeeded.

Product development continues independently. This reconciliation is required before the requested source rollout, not permission to stop SYS-PROMPT/CORP implementation or manufacture extra product gates.
