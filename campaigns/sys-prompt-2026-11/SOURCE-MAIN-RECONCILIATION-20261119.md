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

## Reviewed builder milestone integration

Owner's standing milestone source-push grant applies to both hosts after main reconciliation. Existing accepted campaign records were checked: USER-PROJECT technical gate reports, PRIOR owner acceptance7cd10d049 with explicit limits, earlier corporate baseline and reviewed intermediate native milestones. Current dirty projection/schema candidates and untracked drafts were excluded. Builder's bounded historical changed-blob scan covered661 unique blobs; seven credential-URL matches were classified as synthetic security fixtures or documentation placeholders, zero confirmed real credentials. This is limited-pattern evidence, not an absolute absence claim.

Local reviewed main integration is `aac918599f796bfd048360a1d525b544e44efffb`, parents reconciled main8678 and exact builder boundary `6f33c5c9c1877ee23def28e32f4ee2bec0b0e55e`. Its tree is exactly the reviewed builder tree `4bb123ace765b984fa6f73c61796b61d7f2e09a4`; no task working-tree changes entered through this operation. Standard mirror task btsiruamp exited1 only because the four known existing GitVerse tags refused non-force updates. Both main pushes succeeded; independent remote queries verified GitHub and GitVerse main exactly `aac918599f796bfd048360a1d525b544e44efffb`, with a zero file diff against the accepted builder tree. No force/tag rewrite occurred. Source milestone publication succeeded on both hosts; overall fanout remains partial due to tag drift. This is an intermediate source milestone, not full SYS-PROMPT, new CORP-ENV or NST completion.

## Shared-projection milestone rollout

The next large reviewed boundary consists exactly of builder `89d3e188b` (81 accepted projection/schema/generated/report files) and report-only `c25b060f830f7676aa9a4f3de9455db8dd62a2e3`, after previously published6f33. All81 committed blobs were independently compared with the accepted frozen manifest. Publication main `d5b1ac46a696a8e1b956ded320b2be4e7c85e8b8` retains parents aac918 and c25b and exactly the c25b tree; subsequent dirty Hermes MCP work is excluded.

Standard mirror task bg1ez3key published main on both hosts; independent remote queries verified GitHub main and GitVerse main exactly d5b1ac46a696a8e1b956ded320b2be4e7c85e8b8. Overall exit1 again reports only the same four preexisting GitVerse tag refusals, with no force or tag overwrite. SYS-PROMPT/CORP/NST completion is not implied by source milestone publication.

## Hermes MCP milestone rollout

Exact builder milestone `2d78ca2c22b348e9f6b1ceb17d5150d796787201` was independently compared with its accepted 25-file manifest before integration. Publication main `4e7d068882b5f1c5eaa1f1f281a75cf255f9136e` retains the previous published main and builder milestone as parents and has exactly the reviewed builder tree. Later profile-aware skills work is excluded.

Standard mirror task bnjqrfyy5 published main on both hosts; independent remote queries verified GitHub main and GitVerse main exactly `4e7d068882b5f1c5eaa1f1f281a75cf255f9136e`. Overall exit1 again reflects only the four known existing GitVerse tag refusals; no force or tag rewrite occurred. This publishes the accepted MCP milestone, not full SYS-PROMPT, CORP-ENV or NST completion.

## Hermes builtin profile-skills milestone rollout

Exact builder milestone `cda62ffbbaeea51f1402622d9d2c332a79b87005` contains exactly the 11 accepted manifest paths; every committed blob was independently matched against manifest SHA256 `e312ba2821ece3864c8fa1395b87ca4569e4c5643e887a444e3b9d8d62fa5491`. Publication main `3daa6ee170af7679ed41f289ec357ad173841be3` retains previous main `4e7d068882b5f1c5eaa1f1f281a75cf255f9136e` and the builder milestone as parents, with exactly the accepted tree `0173988eba938ec1a52ea9b56ad51e2edd7042b5`. Unrelated dirty files and future native session-selector work are excluded.

Standard mirror task `bu6i0c2u2` published main on both hosts. Independent remote queries verified GitHub and GitVerse main exactly `3daa6ee170af7679ed41f289ec357ad173841be3`; file diff against the accepted builder source is zero. Overall exit1 reflects only the same four existing GitVerse tag refusals (`v1.0.8` through `v1.0.11`). No force or tag rewrite occurred. Bounded builtin skills acceptance and source publication are not general package-skill/activation completion, full native session parity or SYS-PROMPT/CORP-ENV/NST closure.

## Explicit owner-authorized GitVerse tag alignment

On 2026-11-19 the owner explicitly requested replacing the conflicting GitVerse tags with the GitHub versions. This is a narrow exception to the prior no-tag-rewrite boundary, not standing permission for future tag or branch rewrites. Both remotes and local tag objects were checked immediately before mutation. One atomic push updated exactly four GitVerse tag refs with per-ref expected-old-OID leases; no delete/recreate gap, other tags or branches were involved:

| Tag | Previous GitVerse OID | Selected GitHub / resulting GitVerse OID |
|---|---|---|
| v1.0.8 | `753145f2546071e3418323cafeff0eb4c46b5660` | `6cf994349d9eccf87551e40061f11bc15f7e8f1a` |
| v1.0.9 | `67c59ba58e9127494a27239b494247332eee0dd2` | `3dbc249839c290a9c6e0308277801dbb6688bdf6` |
| v1.0.10 | `49a2d745ec2ad8bb123ba867a53400b0a3ff0c3d` | `e00e1533bf561b20551929f2416869688528bbf7` |
| v1.0.11 | `5e470926ad0f34d61e37d387ce4b7c153356a804` | `e9d6e6fabc8b75c4febc05509189ff9880ad722a` |

Independent post-update queries verified all tags and configured branches match across both hosts (17 refs each), with main unchanged at `3daa6ee170af7679ed41f289ec357ad173841be3`. `cargo xtask mirror --check` returned success for all configured branches. The historical mirror tasks above remain exit1; this later authorized repair resolves their outstanding tag drift rather than retroactively making them green.

## Hermes bounded query-selector milestone rollout

Exact accepted builder boundary `4391c12c8f596e43eb01fafe57f5390d3b8341af` contains exactly the 10 accepted paths; every committed blob independently matched refreshed manifest SHA256 `11c278d0309e3fc741139d94f3772303e059b96ed2e8d98496f7a77bf333390e`. Publication main `1c13c94cd047f26ac7c46fb088b98f2be19fb2bb` retains previous main `3daa6ee170af7679ed41f289ec357ad173841be3` and that builder boundary as parents, with exactly accepted tree `badf45a22e71a8f95fb9a030a3c6fce70f5268a9`. Subsequent parser/frontend work is excluded.

Standard mirror task `bqdosq89y` exited0: main, next, release1.0.0 and tags succeeded on both hosts after the authorized tag repair. Independent remote queries verified GitHub and GitVerse main exactly `1c13c94cd047f26ac7c46fb088b98f2be19fb2bb`, with zero file diff against the accepted builder boundary. This fanout is fully successful; historical exit1 receipts remain unchanged. Publication accepts no additional behavior: unattended oneshot remains BLOCKED, parser/config-default/shared-gateway/worker/platform gaps and final SYS parity remain open. No SYS-PROMPT/CORP-ENV/NST closure or release is implied.

## Hermes configured-worktree query milestone rollout

Exact accepted builder boundary `d83a1e5ada444ba497b4d058d6004eaf735e17cd` contains exactly seven accepted paths; every committed blob independently matched manifest SHA256 `95d6e04e1aa167fdfcdea0e5926e8e322ea5a881c204181dda6ff3fe59934467`. Publication main `6131738c5ebc5c7cbb72bb4e7fe303297e126808` retains previous main `1c13c94cd047f26ac7c46fb088b98f2be19fb2bb` and that builder boundary as parents, with exactly accepted tree `a6b716317d1249c27e151c31c829ee09b05c6671`. Subsequent worker qualification is excluded.

Standard mirror task `bgof53xjg` exited0: all configured branches and tags succeeded on both hosts. Independent remote queries verified both main refs exactly `6131738c5ebc5c7cbb72bb4e7fe303297e126808`, and diff against the accepted builder boundary is zero. This publishes only the bounded finite-query configured-worktree repair, not classic/TUI or unattended oneshot support, worker/platform qualification, full SYS-PROMPT/CORP-ENV/NST completion or a release.

## Native worker identity milestone rollout

Exact accepted builder boundary `a5bb3eea81a30dd0f96737c35330594a5865279f` contains the two accepted paths; both committed blobs independently matched manifest SHA256 `a9d7f44cb49666806d7cce2cfdb01c5c5521756e6f1a26b8410e3899e69e320e`. Publication main `1f327d9811dc823588ad5cae1ab5ed1c0b1df1a5` retains previous main `6131738c5ebc5c7cbb72bb4e7fe303297e126808` and that boundary as parents, with exactly accepted tree `248669533f2fb25491eaaf48649e18ad20408efc`. Subsequent parity evidence consolidation is excluded.

Standard mirror task `bp2hg79rw` exited0: all configured branches and tags succeeded on both hosts. Independent queries verified both remote main refs exactly `1f327d9811dc823588ad5cae1ab5ed1c0b1df1a5`; diff against the accepted builder boundary is zero. Publication covers the Pi executable-identity repair and bounded native worker evidence only. It does not establish separate child filesystem isolation, all worker modes/platforms, Hermes prerequisite completion, final parity acceptance, SYS-PROMPT/CORP-ENV/NST closure or a release.

## Non-negotiable limits

Outside the exact four-tag owner-authorized repair above: no force push, rewrite-ref lease, hard reset, deletion of old main, blind ours merge, mass conflict resolution, mirroring of unrelated tags or ad hoc target bypass. No mirror manifest changes merely to make a task branch publish. Use PROP-016 and the existing mirror owner; source publication is not a release or package publication. Never publish dirty working files or local secrets.

## Finite reconciliation route

1. Recheck the two remote OIDs immediately before any mutation. Preserve both histories and exact baseline refs; do not operate in the peer checkout or copied worktree registrations.
2. Classify GitVerse-only changes using the established patch-equivalent counterparts and identify any later superseding/reverting work on the accepted GitHub line. Use existing accepted requirements/history, not an assumption that old commits win or that hash difference is missing functionality.
3. Produce an explicit reconciliation candidate that retains both parents/history. If all remote-only effects are duplicate or deliberately superseded, a history-joining commit with the accepted main tree may be appropriate; this needs explicit review of that disposition, not a silent `-s ours` shortcut. If genuine missing effects exist, integrate only those finite effects and test their affected owners. Do not manually resolve323 conflicts wholesale just to join duplicate histories.
4. Review the broad unpublished builder range by meaningful accepted campaign boundaries, exclude unrelated/WIP changes, and bind the exact integrated source commit intended for rollout. Do not infer acceptance from branch ancestry alone.
5. Prepare actual affected tests and non-destructive merge candidate review before changing maintained main. Any genuinely irreversible/sensitive action remains owner approval territory; current source-push grant is not a history-rewrite grant.
6. Once a reviewed main descends from BOTH remote main OIDs, ordinary source push can be fast-forward on both hosts. Preflight every configured mirror ref/tag first; the current mirror tool fans the manifest set, so unrelated refs must already be equal or separately authorized, not silently rolled out.
7. Execute the existing fanout once, verify resulting remote main OIDs on GitHub and GitVerse, report each host separately and retain partial-failure evidence. Do not report both published if only one succeeded.

Product development continues independently. This reconciliation is required before the requested source rollout, not permission to stop SYS-PROMPT/CORP implementation or manufacture extra product gates.

## Reviewed host integration baseline (including Pi H4)

Verified the nine-commit linear range after `a5bb3eea8` against nine frozen manifests, including the report-only `2ebbde8a69ec09d7e80c1bca4ebdda217fa29f80`. The eight host checks passed on `ef69b21c`; the report adds no product behavior. Standard mirror succeeded for every configured ref. Independent remote queries confirmed both GitHub and GitVerse main `1e719c002ae32ac87d0c107c3fd2a58333b9537c`, exact reviewed tree `21a540f3b1532d1c89063a4f825577bd1f12275e`. No new READY-gateway implementation, Hermes upstream draft, release or campaign closure was published.
