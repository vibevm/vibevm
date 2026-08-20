# Commit message format {#root}

<status stage="spec" state="done"/>

@fact:ALL-COMMITS-FOLLOW-THE-CONVENTIONAL-COMMITS-SPECIFICATION All commits in this project follow the [Conventional Commits](https://www.conventionalcommits.org/)
specification. @status:impl/done

@fact:this-document-is-the-shape-body-and-scope-convention This document is the message shape, body structure,
and scope convention each commit must carry. @status:impl/done

## Header {#header}

```
type(scope): short imperative subject line
```

- @fact:HEADER-TARGET-LENGTH-AND-HARD-LIMIT **Target length:** ≤ 60 characters. **Hard limit:** 72. Git web
  UIs truncate beyond that, and a truncated subject on the commit
  list is how decisions become invisible to readers who scan rather
  than scroll. @status:impl/done
- @fact:HEADER-IMPERATIVE-MOOD **Imperative mood.** "add", not "added"; "fix", not "fixes"; "refactor",
  not "refactored". The subject completes the sentence *"If applied,
  this commit will …"*. @status:impl/done
- @fact:HEADER-LOWERCASE **Lowercase.** Including the first word after the `type(scope):`
  prefix. The typed prefix is the visual anchor; a capitalised
  first word competes with it for attention. @status:impl/done

### Allowed types {#types}

| Type       | When to use |
|------------|-------------|
| @fact:ROW-TYPE-FEAT `feat` @status:impl/done | New user-visible functionality. @status:impl/done |
| @fact:ROW-TYPE-FIX `fix` @status:impl/done | Bug fix. Name what broke and what it now does. @status:impl/done |
| @fact:ROW-TYPE-CHORE `chore` @status:impl/done | Housekeeping with no behaviour change. @status:impl/done |
| @fact:ROW-TYPE-DOCS `docs` @status:impl/done | Documentation, including spec updates. @status:impl/done |
| @fact:ROW-TYPE-BUILD `build` @status:impl/done | Build system, external dependency, toolchain pin. @status:impl/done |
| @fact:ROW-TYPE-TEST `test` @status:impl/done | Add or fix tests; no production-code change. @status:impl/done |
| @fact:ROW-TYPE-REFACTOR `refactor` @status:impl/done | Internal restructuring; no behaviour change. @status:impl/done |
| @fact:ROW-TYPE-PERF `perf` @status:impl/done | Performance improvement. @status:impl/done |
| @fact:ROW-TYPE-STYLE `style` @status:impl/done | Formatting / whitespace; no semantic change. @status:impl/done |
| @fact:ROW-TYPE-CI `ci` @status:impl/done | CI or pipeline configuration. @status:impl/done |
| @fact:ROW-TYPE-REVERT `revert` @status:impl/done | Revert a previous commit; reference the reverted SHA in the body. @status:impl/done |

@fact:USE-EXACTLY-ONE-TYPE Use exactly one. @status:impl/done

@fact:TWO-TYPES-AT-ONCE-MEANS-TWO-COMMITS If a commit feels like two types at once, it is
two commits. @status:impl/done

### Scope {#scope}

@fact:SCOPE-NAMES-THE-MOST-AFFECTED-SUBSYSTEM Scope names the most affected subsystem — a crate, a package, a
module, a documentation area. @status:impl/done

@fact:examples-of-scope-from-this-project Examples from this project:
`core`, `install`, `wal`, `registry`, `spec`, `build`. @status:impl/done

@fact:CHOOSE-THE-NARROWEST-ACCURATE-SCOPE Choose the **narrowest accurate** scope. @status:impl/done

@fact:the-narrower-form-hits-the-log-filter-correctly `feat(wal): add morning
routine` is better than `feat(core): add wal morning routine`,
because readers filter the log by scope and the narrower form hits
the filter correctly. @status:spec/done

@fact:scope-is-optional-in-the-grammar-but-omit-it-only-with-reason Scope is optional in the strict Conventional Commits grammar, but
omit it only when the change legitimately has no scope (e.g. a
project-wide `.gitattributes` addition). @status:spec/done

## Body {#body}

@fact:A-BLANK-LINE-AFTER-THE-SUBJECT-THEN-A-FREE-FORM-BODY A single blank line after the subject, then a free-form body. @status:impl/done

### What to include {#body-include}

- @fact:INCLUDE-WHY-THIS-CHANGE-WAS-MADE **Why this change was made.** Link to the spec section, issue,
  measurement, or conversation that drove it. Use `spec://…` URIs
  so future sessions can follow the reference without having to
  guess where it is documented. @status:impl/done
- @fact:INCLUDE-WHAT-FOLLOWS-FROM-IT **What follows from it.** Consequences that are invisible in
  the diff: "this unblocks FEAT-007"; "after this, old callers
  must be migrated"; "this is a temporary workaround for #42". @status:impl/done
- @fact:INCLUDE-WHAT-WAS-CONSIDERED-AND-REJECTED **What was considered and rejected.** One line each. Future-you
  re-opens this conversation every six months unless the log says
  "we considered adaptive timeout and rejected it because of UX
  unpredictability". @status:impl/done

### What to skip {#body-skip}

- @fact:SKIP-WHAT-THE-DIFF-ALREADY-SHOWS **Do not describe what the diff already shows.** "This commit
  adds a function `foo()`" is noise — the diff shows that. The
  message should answer "why did `foo()` need to exist?". @status:impl/done
- @fact:SKIP-IMPLEMENTATION-DETAILS-THAT-WILL-CHANGE **Do not include implementation details that will change on
  the next refactor.** They rot faster than the surrounding prose
  and mislead readers once stale. @status:impl/done

### Body length {#body-length}

@fact:BODY-LENGTH-IS-FREE-FORM Free-form. @status:impl/done

@fact:A-THREE-LINE-BODY-IS-FINE-FOR-A-SMALL-FIX A three-line body is fine for a small fix. @status:impl/done

@fact:A-TWENTY-LINE-BODY-IS-FINE-FOR-A-MILESTONE-COMMIT A twenty-line
body is fine for a milestone commit where the reasoning matters. @status:impl/done

@fact:brevity-at-the-expense-of-the-why-costs-every-future-read Length is not a virtue, but brevity at the expense of the *why* is
a cost paid on every future read. @status:spec/done

### Body format {#body-format}

@fact:PREFER-PARAGRAPHS-WHEN-THE-REASONING-IS-CONTINUOUS Prefer paragraphs over bullet lists when the reasoning is
continuous. @status:impl/done

@fact:BULLETS-ARE-FOR-GENUINELY-PARALLEL-ITEMS Bullets are for enumerations of genuinely parallel
items — three rejected alternatives, four affected callers. @status:impl/done

@fact:DO-NOT-BULLET-A-SINGLE-PARAGRAPH-OF-PROSE Do not
bullet a single paragraph of prose into pieces. @status:impl/done

## Worked examples {#examples}

### Small fix {#example-small-fix}

```
fix(wal): stop crashing on missing _Updated line

Treat a WAL without `_Updated:` as "age = infinite" rather than
panicking. The guard previously short-circuited the whole status
check, so stale-WAL projects could not run `vibe check`. Tested
against an empty WAL and a WAL whose first line is a stray comment.
```

### Feature {#example-feature}

```
feat(registry): freshness TTL for cloned mirrors

Cache under ~/.vibe/registries/<hash>/ now carries a meta.toml
with last_pulled_at. Pulls skip when the cache is fresher than
the configured TTL (default 1 h); `vibe registry sync` forces a
pull regardless. Rationale: every `vibe install` hitting the
network was making offline work painful and slow.

Cited by spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-001#freshness.
```

### Refactor {#example-refactor}

```
refactor(core): hoist timestamp helper out of vibe-cli

Both vibe-cli and vibe-registry needed `now_unix_utc`. Keeping it
in vibe-cli forced vibe-registry to depend on the CLI crate, which
violates the intended dependency direction. Moved to
`vibe_core::timestamp`. No behaviour change; 78 tests still green.
```

### Revert {#example-revert}

```
revert: "feat(install): auto-retry on 429"

Reverts 1a2b3c4. The retry loop was masking real rate-limit bugs
in the registry layer. Correct fix is to surface the 429 upstream
and let the caller decide. Issue #47.
```

### Docs sync (from sync-from-code flow) {#example-docs-sync}

```
docs(spec): sync timeout to 600s in PROP-003 §verification.timeout

Code changed TIMEOUT from 300 s to 600 s after VPN latency
measurement (2026-03-05, 847 messages, 128 users). Spec now
carries the new value, the reason, and the revisit trigger.
```

## Anti-patterns {#antipatterns}

| Bad subject                       | Why it fails                        | Fix                                         |
|-----------------------------------|-------------------------------------|---------------------------------------------|
| @fact:ROW-ANTI-UPDATES `updates` @status:impl/done | No type, no scope, no *why*. @status:impl/done | `docs(spec): add freshness TTL rationale` @status:impl/done |
| @fact:ROW-ANTI-WIP `wip` @status:impl/done | Not a finished thought. @status:impl/done | Squash into the next real commit. @status:impl/done |
| @fact:ROW-ANTI-FIXED-BUG `fixed bug` @status:impl/done | Nothing learned from the log. @status:impl/done | Name the bug and what drove the fix. @status:impl/done |
| @fact:ROW-ANTI-THREE-IDEAS `feat: add foo, bar, and baz` @status:impl/done | Three ideas, one commit. @status:impl/done | Split. @status:impl/done |
| @fact:ROW-ANTI-HUGE-REFACTOR `feat(core): huge refactor` @status:impl/done | Behaviour change rolled into refactor. @status:impl/done | Two commits: refactor first, feature second. @status:impl/done |
| @fact:ROW-ANTI-CAPITALISED-VAGUE `Fix: handle edge case` @status:impl/done | Capitalised, vague, no scope. @status:impl/done | `fix(verify): handle empty sender_id` @status:impl/done |

## Interaction with the git-atomic-commits rule {#atomicity}

@fact:CONVENTIONAL-COMMITS-DOES-NOT-ENFORCE-ATOMICITY Conventional Commits does not by itself enforce atomicity. @status:impl/done

@fact:A-VALID-MESSAGE-CAN-STILL-VIOLATE-THE-ATOMIC-RULE A commit
with the subject `feat(core): add foo, bar, and baz` is syntactically
valid Conventional Commits *and* a violation of the atomic rule. Both
rules run together: @status:impl/done

1. @fact:BOTH-RULES-ONE-IDEA-PER-COMMIT The commit carries exactly one idea (atomic). @status:impl/done
2. @fact:BOTH-RULES-THE-CONVENTIONAL-COMMITS-SHAPE The message announces that idea in the Conventional Commits shape
   (`type(scope): subject` + *why* body). @status:impl/done

@fact:PASS-BOTH-AND-THE-COMMIT-IS-WELL-FORMED Pass both, and the commit is well-formed. @status:impl/done
