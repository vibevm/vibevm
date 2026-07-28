# Enforcement checklist {#root}

<status stage="spec" state="done"/>

##A-POLICY-WITH-NO-CHECKER-IS-A-WISH **Scope of this document.** A policy with no checker is a wish. @spec/done

##scope-of-this-document This
file is the per-surface checklist for the attribution policy, the
mechanical checks that catch slips before they publish, and the
drill for when one lands anyway. @impl/done

##written-for-concealment-flipped-for-disclosure Written for the concealment posture;
for disclosure, run the same checks with the polarity flipped (scan
for *missing* marks instead of present ones). @impl/done

## The surfaces {#surfaces}

| Surface | What leaks there | Check |
|---|---|---|
| ##ROW-SURFACE-COMMIT-MESSAGE-BODIES Commit message bodies @spec/done | "generated with…", tool ads, model names @spec/done | pre-push scan (below) @spec/done |
| ##ROW-SURFACE-COMMIT-TRAILERS Commit trailers @spec/done | `Co-Authored-By`, `Signed-off-by: <model>` @spec/done | pre-push scan (below) @spec/done |
| ##ROW-SURFACE-BRANCH-AND-TAG-NAMES Branch / tag names @spec/done | agent-generated branch names carrying tool names @spec/done | `git branch -a` eyeball at review; naming convention in the boot file @spec/done |
| ##ROW-SURFACE-CODE-COMMENTS Code comments @spec/done | "AI-generated", model names in TODO/FIXME @spec/done | repo-wide grep, periodically @spec/done |
| ##ROW-SURFACE-README-DOCS-RELEASE-NOTES README / docs / release notes @spec/done | boilerplate credit lines @spec/done | part of release checklist @spec/done |
| ##ROW-SURFACE-PR-TITLES-AND-DESCRIPTIONS PR titles and descriptions @spec/done | tool-inserted footers @spec/done | PR template with an explicit placeholder to overwrite @spec/done |
| ##ROW-SURFACE-CI-CONFIGURATION CI configuration @spec/done | marketplace actions inserting attribution steps @spec/done | review any new CI step's output once @spec/done |
| ##ROW-SURFACE-GENERATED-FILE-HEADERS Generated-file headers @spec/done | scaffolding tools stamping their names @spec/done | check scaffold output the first time a generator is adopted @spec/done |

## The pre-push scan {#pre-push}

##MESSAGES-AND-TRAILERS-ARE-MECHANICALLY-CHECKABLE The two highest-volume surfaces — messages and trailers — are
mechanically checkable. @impl/done

##pre-push-scan-lead Run before every push (or wire as a
`pre-push` hook): @impl/done

```sh
# Scan outgoing commits for attribution marks. Nonzero output = stop.
git log --format='%H %B' @{u}..HEAD |
  grep -inE 'co-authored-by|signed-off-by:.*(claude|gpt|gemini|copilot|llama|codex|model)|generated (with|by) [^ ]*(ai|llm|claude|gpt|copilot)' \
  && echo 'ATTRIBUTION MARK FOUND — fix before push' || true
```

##ADAPT-THE-PATTERN-LIST-TO-YOUR-OWN-TOOLS Adapt the pattern list to the tools your team actually runs — the
list above is a starting set, not an oracle. @impl/done

##a-real-hook-beats-ten-generic-ones A hook that fires on
your real tools' real phrasing is worth ten generic ones. @spec/done

##the-scan-covers-all-co-authored-by-trailers Note the
scan intentionally covers *all* `Co-Authored-By` trailers: under
this policy human co-authors are rare enough to allowlist by hand,
and a false positive costs seconds while a false negative publishes. @impl/done

## Tool configuration beats scanning {#configure}

##SCANNING-CATCHES-SLIPS-CONFIGURATION-PREVENTS-THEM Scanning catches slips; configuration prevents them. @impl/done

##most-agents-accept-standing-instructions Most coding
agents accept standing instructions (a project rules file read at
session start). @spec/done

##PUT-THE-POLICY-IN-THE-AGENTS-STANDING-INSTRUCTIONS Put the policy there — this package's boot snippet
is exactly that — and the agent stops *producing* the marks, which
is cheaper than catching them. @impl/done

##SET-A-TOOLS-HARD-TRAILER-SETTING-ONCE Where a tool has a hard setting for
commit trailers, set it once and note it in the project's setup doc. @impl/done

## The periodic audit line {#audit}

##SLOW-SURFACES-ARE-NOT-WORTH-A-PER-PUSH-SCAN Slow-accumulating surfaces (comments, docs, release notes) are not
worth a per-push scan. @impl/done

##periodic-audit-line-lead Put one line in the project's periodic audit
checklist (if you run `flow:health-audit` or similar): @impl/done

> ##THE-AUDIT-CHECKLIST-LINE Attribution: repo-wide grep for the pattern set; check surfaces
> added since last audit (new CI steps, new scaffolds, new doc
> generators). @impl/done

## When a slip lands {#slip-drill}

1. ##DRILL-CAUGHT-BEFORE-PUSH **Caught before push:** amend or rebase locally. No further
   action; this is what the scan exists for. @impl/done
2. ##DRILL-CAUGHT-AFTER-PUSH **Caught after push:** do **not** rewrite published history on
   reflex — the frozen-history rule (`flow:git-atomic-commits` §pushed)
   wins by default. Record the slip, fix the *source* (the tool or
   template that produced it), and surface to the owner: rewriting
   one commit's metadata out of published history is the owner's
   call, made knowing who has already pulled. @impl/done
3. ##DRILL-EITHER-WAY **Either way:** if the same surface slips twice, the checklist —
   not the person — is at fault. Add the missing check. @impl/done

## Summary {#summary}

- ##SUM-EIGHT-SURFACES-TWO-OF-THEM-MECHANICAL Eight surfaces; two of them (messages, trailers) get a mechanical
  pre-push scan, the rest ride templates, tool configuration, and
  the periodic audit. @impl/done
- ##SUM-CONFIGURE-FIRST-SCAN-AS-BACKSTOP Configure tools not to produce marks; scan as the backstop. @impl/done
- ##SUM-PRE-PUSH-SLIPS-VERSUS-PUSHED-SLIPS Pre-push slips are amended freely; pushed slips default to
  stand-and-fix-the-source, with history rewrite an owner-level
  exception. @impl/done
- ##SUM-A-TWICE-SLIPPED-SURFACE-EARNS-A-LINE A surface that slips twice earns a new checklist line. @impl/done
