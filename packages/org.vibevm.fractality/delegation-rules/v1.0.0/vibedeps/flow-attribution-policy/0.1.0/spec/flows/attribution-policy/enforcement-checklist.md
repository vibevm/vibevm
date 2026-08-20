# Enforcement checklist {#root}

**Scope of this document.** A policy with no checker is a wish. This
file is the per-surface checklist for the attribution policy, the
mechanical checks that catch slips before they publish, and the
drill for when one lands anyway. Written for the concealment posture;
for disclosure, run the same checks with the polarity flipped (scan
for *missing* marks instead of present ones).

## The surfaces {#surfaces}

| Surface | What leaks there | Check |
|---|---|---|
| Commit message bodies | "generated with…", tool ads, model names | pre-push scan (below) |
| Commit trailers | `Co-Authored-By`, `Signed-off-by: <model>` | pre-push scan (below) |
| Branch / tag names | agent-generated branch names carrying tool names | `git branch -a` eyeball at review; naming convention in the boot file |
| Code comments | "AI-generated", model names in TODO/FIXME | repo-wide grep, periodically |
| README / docs / release notes | boilerplate credit lines | part of release checklist |
| PR titles and descriptions | tool-inserted footers | PR template with an explicit placeholder to overwrite |
| CI configuration | marketplace actions inserting attribution steps | review any new CI step's output once |
| Generated-file headers | scaffolding tools stamping their names | check scaffold output the first time a generator is adopted |

## The pre-push scan {#pre-push}

The two highest-volume surfaces — messages and trailers — are
mechanically checkable. Run before every push (or wire as a
`pre-push` hook):

```sh
# Scan outgoing commits for attribution marks. Nonzero output = stop.
git log --format='%H %B' @{u}..HEAD |
  grep -inE 'co-authored-by|signed-off-by:.*(claude|gpt|gemini|copilot|llama|codex|model)|generated (with|by) [^ ]*(ai|llm|claude|gpt|copilot)' \
  && echo 'ATTRIBUTION MARK FOUND — fix before push' || true
```

Adapt the pattern list to the tools your team actually runs — the
list above is a starting set, not an oracle. A hook that fires on
your real tools' real phrasing is worth ten generic ones. Note the
scan intentionally covers *all* `Co-Authored-By` trailers: under
this policy human co-authors are rare enough to allowlist by hand,
and a false positive costs seconds while a false negative publishes.

## Tool configuration beats scanning {#configure}

Scanning catches slips; configuration prevents them. Most coding
agents accept standing instructions (a project rules file read at
session start). Put the policy there — this package's boot snippet
is exactly that — and the agent stops *producing* the marks, which
is cheaper than catching them. Where a tool has a hard setting for
commit trailers, set it once and note it in the project's setup doc.

## The periodic audit line {#audit}

Slow-accumulating surfaces (comments, docs, release notes) are not
worth a per-push scan. Put one line in the project's periodic audit
checklist (if you run `flow:health-audit` or similar):

> Attribution: repo-wide grep for the pattern set; check surfaces
> added since last audit (new CI steps, new scaffolds, new doc
> generators).

## When a slip lands {#slip-drill}

1. **Caught before push:** amend or rebase locally. No further
   action; this is what the scan exists for.
2. **Caught after push:** do **not** rewrite published history on
   reflex — the frozen-history rule (`flow:atomic-commits` §pushed)
   wins by default. Record the slip, fix the *source* (the tool or
   template that produced it), and surface to the owner: rewriting
   one commit's metadata out of published history is the owner's
   call, made knowing who has already pulled.
3. **Either way:** if the same surface slips twice, the checklist —
   not the person — is at fault. Add the missing check.

## Summary {#summary}

- Eight surfaces; two of them (messages, trailers) get a mechanical
  pre-push scan, the rest ride templates, tool configuration, and
  the periodic audit.
- Configure tools not to produce marks; scan as the backstop.
- Pre-push slips are amended freely; pushed slips default to
  stand-and-fix-the-source, with history rewrite an owner-level
  exception.
- A surface that slips twice earns a new checklist line.
