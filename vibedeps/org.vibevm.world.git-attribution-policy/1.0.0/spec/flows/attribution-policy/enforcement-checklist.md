# Enforcement checklist {#root}

<status stage="spec" state="done"/>

@fact:A-POLICY-WITH-NO-CHECKER-IS-A-WISH **Scope of this document.** A policy with no checker is a wish. @status:spec/done

@fact:scope-of-this-document This
file is the per-surface checklist for the attribution policy, the
mechanical checks that catch slips before they publish, and the
drill for when one lands anyway. @status:impl/done

@fact:written-for-concealment-flipped-for-disclosure Written for the concealment posture;
for disclosure, run the same checks with the polarity flipped (scan
for *missing* marks instead of present ones). @status:impl/done

## The surfaces {#surfaces}

| Surface | What leaks there | Check |
|---|---|---|
| @fact:ROW-SURFACE-COMMIT-MESSAGE-BODIES Commit message bodies @status:spec/done | "generated with…", tool ads, model names @status:spec/done | pre-push scan (below) @status:spec/done |
| @fact:ROW-SURFACE-COMMIT-TRAILERS Commit trailers @status:spec/done | `Co-Authored-By`, `Signed-off-by: <model>` @status:spec/done | pre-push scan (below) @status:spec/done |
| @fact:ROW-SURFACE-BRANCH-AND-TAG-NAMES Branch / tag names @status:spec/done | agent-generated branch names carrying tool names @status:spec/done | `git branch -a` eyeball at review; naming convention in the boot file @status:spec/done |
| @fact:ROW-SURFACE-CODE-COMMENTS Code comments @status:spec/done | "AI-generated", model names in TODO/FIXME @status:spec/done | repo-wide grep, periodically @status:spec/done |
| @fact:ROW-SURFACE-README-DOCS-RELEASE-NOTES README / docs / release notes @status:spec/done | boilerplate credit lines @status:spec/done | part of release checklist @status:spec/done |
| @fact:ROW-SURFACE-PR-TITLES-AND-DESCRIPTIONS PR titles and descriptions @status:spec/done | tool-inserted footers @status:spec/done | PR template with an explicit placeholder to overwrite @status:spec/done |
| @fact:ROW-SURFACE-CI-CONFIGURATION CI configuration @status:spec/done | marketplace actions inserting attribution steps @status:spec/done | review any new CI step's output once @status:spec/done |
| @fact:ROW-SURFACE-GENERATED-FILE-HEADERS Generated-file headers @status:spec/done | scaffolding tools stamping their names @status:spec/done | check scaffold output the first time a generator is adopted @status:spec/done |

## The pre-push scan {#pre-push}

@fact:MESSAGES-AND-TRAILERS-ARE-MECHANICALLY-CHECKABLE The two highest-volume surfaces — messages and trailers — are
mechanically checkable. @status:impl/done

@fact:pre-push-scan-lead Run before every push (or wire as a
`pre-push` hook): @status:impl/done

```sh
# Scan outgoing commits for attribution marks. Nonzero output = stop.
git log --format='%H %B' @{u}..HEAD |
  grep -inE 'co-authored-by|signed-off-by:.*(claude|gpt|gemini|copilot|llama|codex|model)|generated (with|by) [^ ]*(ai|llm|claude|gpt|copilot)' \
  && echo 'ATTRIBUTION MARK FOUND — fix before push' || true
```

@fact:ADAPT-THE-PATTERN-LIST-TO-YOUR-OWN-TOOLS Adapt the pattern list to the tools your team actually runs — the
list above is a starting set, not an oracle. @status:impl/done

@fact:a-real-hook-beats-ten-generic-ones A hook that fires on
your real tools' real phrasing is worth ten generic ones. @status:spec/done

@fact:the-scan-covers-all-co-authored-by-trailers Note the
scan intentionally covers *all* `Co-Authored-By` trailers: under
this policy human co-authors are rare enough to allowlist by hand,
and a false positive costs seconds while a false negative publishes. @status:impl/done

## Tool configuration beats scanning {#configure}

@fact:SCANNING-CATCHES-SLIPS-CONFIGURATION-PREVENTS-THEM Scanning catches slips; configuration prevents them. @status:impl/done

@fact:most-agents-accept-standing-instructions Most coding
agents accept standing instructions (a project rules file read at
session start). @status:spec/done

@fact:PUT-THE-POLICY-IN-THE-AGENTS-STANDING-INSTRUCTIONS Put the policy there — this package's boot snippet
is exactly that — and the agent stops *producing* the marks, which
is cheaper than catching them. @status:impl/done

@fact:SET-A-TOOLS-HARD-TRAILER-SETTING-ONCE Where a tool has a hard setting for
commit trailers, set it once and note it in the project's setup doc. @status:impl/done

## The periodic audit line {#audit}

@fact:SLOW-SURFACES-ARE-NOT-WORTH-A-PER-PUSH-SCAN Slow-accumulating surfaces (comments, docs, release notes) are not
worth a per-push scan. @status:impl/done

@fact:periodic-audit-line-lead Put one line in the project's periodic audit
checklist (if you run `flow:health-audit` or similar): @status:impl/done

> @fact:THE-AUDIT-CHECKLIST-LINE Attribution: repo-wide grep for the pattern set; check surfaces
> added since last audit (new CI steps, new scaffolds, new doc
> generators). @status:impl/done

## When a slip lands {#slip-drill}

1. @fact:DRILL-CAUGHT-BEFORE-PUSH **Caught before push:** amend or rebase locally. No further
   action; this is what the scan exists for. @status:impl/done
2. @fact:DRILL-CAUGHT-AFTER-PUSH **Caught after push:** do **not** rewrite published history on
   reflex — the frozen-history rule (`flow:git-atomic-commits` §pushed)
   wins by default. Record the slip, fix the *source* (the tool or
   template that produced it), and surface to the owner: rewriting
   one commit's metadata out of published history is the owner's
   call, made knowing who has already pulled. @status:impl/done
3. @fact:DRILL-EITHER-WAY **Either way:** if the same surface slips twice, the checklist —
   not the person — is at fault. Add the missing check. @status:impl/done

## Summary {#summary}

- @fact:SUM-EIGHT-SURFACES-TWO-OF-THEM-MECHANICAL Eight surfaces; two of them (messages, trailers) get a mechanical
  pre-push scan, the rest ride templates, tool configuration, and
  the periodic audit. @status:impl/done
- @fact:SUM-CONFIGURE-FIRST-SCAN-AS-BACKSTOP Configure tools not to produce marks; scan as the backstop. @status:impl/done
- @fact:SUM-PRE-PUSH-SLIPS-VERSUS-PUSHED-SLIPS Pre-push slips are amended freely; pushed slips default to
  stand-and-fix-the-source, with history rewrite an owner-level
  exception. @status:impl/done
- @fact:SUM-A-TWICE-SLIPPED-SURFACE-EARNS-A-LINE A surface that slips twice earns a new checklist line. @status:impl/done
