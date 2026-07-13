# Commit autonomy protocol {#root}

An agent working in a repository is constantly deciding: *do I just do this, or do I stop and
ask?* Too many confirmation handshakes make an authorised body of work crawl; too few risk an
irreversible mistake. This protocol draws the line on the **commit and push** surface: what
proceeds on the agent's own judgment, and what always stops for a human first.

## The default: routine proceeds {#routine}

**Routine large changes proceed without asking** — and are committed and pushed — when the
activity has already been authorised. Routine means: implementing a planned milestone, finishing
a feature slice, a refactor touching many files for one coherent reason, or any large-but-expected
step of work the human has already greenlit. The approval was given upstream; re-asking "shall I
proceed?" mid-flow is overhead the human already paid for by authorising the activity.

This default optimises for throughput on work that is understood and approved. It presumes the
other disciplines are in force — the changes still land as atomic commits in the message format,
and still respect every red line below.

## The red lines: non-routine stops and asks {#red-lines}

Regardless of any "proceed" posture, a fixed set of operations **always stops and asks first**,
because their blast radius or irreversibility exceeds what an agent should assume authority over:

- **Rewriting published history** — rebasing pushed commits, `git commit --amend` on pushed work.
- **Force operations** — `git push --force` / `--force-with-lease`.
- **Large binary blobs** — anything that bloats the repository irreversibly.
- **CI, signing, or secrets configuration** — changes with reach beyond the working tree.
- **Anything whose reversal would cost work** — the catch-all; if undoing it is expensive, ask.

These are not suspended by a heads-down / "move fast" posture: a mode may remove the
"may I proceed with routine work?" handshake, but never the "may I cross an irreversible
threshold?" one. If a step cannot land without crossing a red line, the agent stops at that
boundary, reports, and asks.

## When uncertain, ask {#uncertain}

The line between routine and non-routine is a judgment call. When a change sits near the
boundary — its reversal cost is unclear, or it touches a surface the human tends to guard —
default to asking. A surplus question costs a moment; a surplus irreversible action costs work.

## Re-derive for your project {#re-derive}

Name your own red-line set — the operations in *your* stack whose reversal costs real work
(a production deploy, a schema migration, a published release) — and state that everything
outside it, once authorised, proceeds without a handshake. The shape is universal; the specific
red lines are yours.
