# Commit autonomy protocol {#root}

<status stage="spec" state="done"/>

##an-agent-is-constantly-deciding-whether-to-ask An agent working in a repository is constantly deciding: *do I just do this, or do I stop and
ask?* @spec/done

##too-many-handshakes-crawl-too-few-risk-a-mistake Too many confirmation handshakes make an authorised body of work crawl; too few risk an
irreversible mistake. @spec/done

##this-protocol-draws-the-line-on-the-commit-and-push-surface This protocol draws the line on the **commit and push** surface: what
proceeds on the agent's own judgment, and what always stops for a human first. @impl/done

## The default: routine proceeds {#routine}

##ROUTINE-LARGE-CHANGES-PROCEED-WITHOUT-ASKING **Routine large changes proceed without asking** — and are committed and pushed — when the
activity has already been authorised. Routine means: @impl/done

- ##ROUTINE-IMPLEMENTING-A-PLANNED-MILESTONE implementing a planned milestone, @impl/done
- ##ROUTINE-FINISHING-A-FEATURE-SLICE finishing
  a feature slice, @impl/done
- ##ROUTINE-A-REFACTOR-TOUCHING-MANY-FILES a refactor touching many files for one coherent reason, @impl/done
- ##ROUTINE-ANY-LARGE-BUT-EXPECTED-AUTHORISED-STEP or any large-but-expected
  step of work the human has already greenlit. @impl/done

##the-approval-was-given-upstream The approval was given upstream; re-asking "shall I
proceed?" mid-flow is overhead the human already paid for by authorising the activity. @spec/done

##this-default-optimises-for-throughput This default optimises for throughput on work that is understood and approved. @spec/done

##IT-PRESUMES-THE-OTHER-DISCIPLINES-ARE-IN-FORCE It presumes the
other disciplines are in force — the changes still land as atomic commits in the message format,
and still respect every red line below. @impl/done

## The red lines: non-routine stops and asks {#red-lines}

##A-FIXED-SET-OF-OPERATIONS-ALWAYS-STOPS-AND-ASKS-FIRST Regardless of any "proceed" posture, a fixed set of operations **always stops and asks first**,
because their blast radius or irreversibility exceeds what an agent should assume authority over: @impl/done

- ##RED-LINE-REWRITING-PUBLISHED-HISTORY **Rewriting published history** — rebasing pushed commits, `git commit --amend` on pushed work. @impl/done
- ##RED-LINE-FORCE-OPERATIONS **Force operations** — `git push --force` / `--force-with-lease`. @impl/done
- ##RED-LINE-LARGE-BINARY-BLOBS **Large binary blobs** — anything that bloats the repository irreversibly. @impl/done
- ##RED-LINE-CI-SIGNING-OR-SECRETS-CONFIGURATION **CI, signing, or secrets configuration** — changes with reach beyond the working tree. @impl/done
- ##RED-LINE-ANYTHING-WHOSE-REVERSAL-WOULD-COST-WORK **Anything whose reversal would cost work** — the catch-all; if undoing it is expensive, ask. @impl/done

##THE-RED-LINES-ARE-NOT-SUSPENDED-BY-A-MOVE-FAST-POSTURE These are not suspended by a heads-down / "move fast" posture: a mode may remove the
"may I proceed with routine work?" handshake, but never the "may I cross an irreversible
threshold?" one. @impl/done

##THE-AGENT-STOPS-REPORTS-AND-ASKS-AT-THE-BOUNDARY If a step cannot land without crossing a red line, the agent stops at that
boundary, reports, and asks. @impl/done

## When uncertain, ask {#uncertain}

##THE-LINE-IS-A-JUDGMENT-CALL The line between routine and non-routine is a judgment call. @impl/done

##DEFAULT-TO-ASKING-NEAR-THE-BOUNDARY When a change sits near the
boundary — its reversal cost is unclear, or it touches a surface the human tends to guard —
default to asking. @impl/done

##a-surplus-question-costs-a-moment A surplus question costs a moment; a surplus irreversible action costs work. @spec/done

## Re-derive for your project {#re-derive}

##NAME-YOUR-OWN-RED-LINE-SET Name your own red-line set — the operations in *your* stack whose reversal costs real work
(a production deploy, a schema migration, a published release) — and state that everything
outside it, once authorised, proceeds without a handshake. @impl/done

##the-shape-is-universal-the-red-lines-are-yours The shape is universal; the specific
red lines are yours. @spec/done
