# Commit autonomy protocol {#root}

<status stage="spec" state="done"/>

@fact:an-agent-is-constantly-deciding-whether-to-ask An agent working in a repository is constantly deciding: *do I just do this, or do I stop and
ask?* @status:spec/done

@fact:too-many-handshakes-crawl-too-few-risk-a-mistake Too many confirmation handshakes make an authorised body of work crawl; too few risk an
irreversible mistake. @status:spec/done

@fact:this-protocol-draws-the-line-on-the-commit-and-push-surface This protocol draws the line on the **commit and push** surface: what
proceeds on the agent's own judgment, and what always stops for a human first. @status:impl/done

## The default: routine proceeds {#routine}

@fact:ROUTINE-LARGE-CHANGES-PROCEED-WITHOUT-ASKING **Routine large changes proceed without asking** — and are committed and pushed — when the
activity has already been authorised. Routine means: @status:impl/done

- @fact:ROUTINE-IMPLEMENTING-A-PLANNED-MILESTONE implementing a planned milestone, @status:impl/done
- @fact:ROUTINE-FINISHING-A-FEATURE-SLICE finishing
  a feature slice, @status:impl/done
- @fact:ROUTINE-A-REFACTOR-TOUCHING-MANY-FILES a refactor touching many files for one coherent reason, @status:impl/done
- @fact:ROUTINE-ANY-LARGE-BUT-EXPECTED-AUTHORISED-STEP or any large-but-expected
  step of work the human has already greenlit. @status:impl/done

@fact:the-approval-was-given-upstream The approval was given upstream; re-asking "shall I
proceed?" mid-flow is overhead the human already paid for by authorising the activity. @status:spec/done

@fact:this-default-optimises-for-throughput This default optimises for throughput on work that is understood and approved. @status:spec/done

@fact:IT-PRESUMES-THE-OTHER-DISCIPLINES-ARE-IN-FORCE It presumes the
other disciplines are in force — the changes still land as atomic commits in the message format,
and still respect every red line below. @status:impl/done

## The red lines: non-routine stops and asks {#red-lines}

@fact:A-FIXED-SET-OF-OPERATIONS-ALWAYS-STOPS-AND-ASKS-FIRST Regardless of any "proceed" posture, a fixed set of operations **always stops and asks first**,
because their blast radius or irreversibility exceeds what an agent should assume authority over: @status:impl/done

- @fact:RED-LINE-REWRITING-PUBLISHED-HISTORY **Rewriting published history** — rebasing pushed commits, `git commit --amend` on pushed work. @status:impl/done
- @fact:RED-LINE-FORCE-OPERATIONS **Force operations** — `git push --force` / `--force-with-lease`. @status:impl/done
- @fact:RED-LINE-LARGE-BINARY-BLOBS **Large binary blobs** — anything that bloats the repository irreversibly. @status:impl/done
- @fact:RED-LINE-CI-SIGNING-OR-SECRETS-CONFIGURATION **CI, signing, or secrets configuration** — changes with reach beyond the working tree. @status:impl/done
- @fact:RED-LINE-ANYTHING-WHOSE-REVERSAL-WOULD-COST-WORK **Anything whose reversal would cost work** — the catch-all; if undoing it is expensive, ask. @status:impl/done

@fact:THE-RED-LINES-ARE-NOT-SUSPENDED-BY-A-MOVE-FAST-POSTURE These are not suspended by a heads-down / "move fast" posture: a mode may remove the
"may I proceed with routine work?" handshake, but never the "may I cross an irreversible
threshold?" one. @status:impl/done

@fact:THE-AGENT-STOPS-REPORTS-AND-ASKS-AT-THE-BOUNDARY If a step cannot land without crossing a red line, the agent stops at that
boundary, reports, and asks. @status:impl/done

## When uncertain, ask {#uncertain}

@fact:THE-LINE-IS-A-JUDGMENT-CALL The line between routine and non-routine is a judgment call. @status:impl/done

@fact:DEFAULT-TO-ASKING-NEAR-THE-BOUNDARY When a change sits near the
boundary — its reversal cost is unclear, or it touches a surface the human tends to guard —
default to asking. @status:impl/done

@fact:a-surplus-question-costs-a-moment A surplus question costs a moment; a surplus irreversible action costs work. @status:spec/done

## Re-derive for your project {#re-derive}

@fact:NAME-YOUR-OWN-RED-LINE-SET Name your own red-line set — the operations in *your* stack whose reversal costs real work
(a production deploy, a schema migration, a published release) — and state that everything
outside it, once authorised, proceeds without a handshake. @status:impl/done

@fact:the-shape-is-universal-the-red-lines-are-yours The shape is universal; the specific
red lines are yours. @status:spec/done
