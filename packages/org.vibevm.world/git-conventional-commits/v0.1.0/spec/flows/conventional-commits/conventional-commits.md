# Commit message format {#root}

<status stage="spec" state="done"/>

##ALL-COMMITS-FOLLOW-THE-CONVENTIONAL-COMMITS-SPECIFICATION All commits in this project follow the [Conventional Commits](https://www.conventionalcommits.org/)
specification. @impl/done

##this-document-is-the-shape-body-and-scope-convention This document is the message shape, body structure,
and scope convention each commit must carry. @impl/done

## Header {#header}

```
type(scope): short imperative subject line
```

- ##HEADER-TARGET-LENGTH-AND-HARD-LIMIT **Target length:** ≤ 60 characters. **Hard limit:** 72. Git web
  UIs truncate beyond that, and a truncated subject on the commit
  list is how decisions become invisible to readers who scan rather
  than scroll. @impl/done
- ##HEADER-IMPERATIVE-MOOD **Imperative mood.** "add", not "added"; "fix", not "fixes"; "refactor",
  not "refactored". The subject completes the sentence *"If applied,
  this commit will …"*. @impl/done
- ##HEADER-LOWERCASE **Lowercase.** Including the first word after the `type(scope):`
  prefix. The typed prefix is the visual anchor; a capitalised
  first word competes with it for attention. @impl/done

### Allowed types {#types}

| Type       | When to use |
|------------|-------------|
| ##ROW-TYPE-FEAT `feat` @impl/done | New user-visible functionality. @impl/done |
| ##ROW-TYPE-FIX `fix` @impl/done | Bug fix. Name what broke and what it now does. @impl/done |
| ##ROW-TYPE-CHORE `chore` @impl/done | Housekeeping with no behaviour change. @impl/done |
| ##ROW-TYPE-DOCS `docs` @impl/done | Documentation, including spec updates. @impl/done |
| ##ROW-TYPE-BUILD `build` @impl/done | Build system, external dependency, toolchain pin. @impl/done |
| ##ROW-TYPE-TEST `test` @impl/done | Add or fix tests; no production-code change. @impl/done |
| ##ROW-TYPE-REFACTOR `refactor` @impl/done | Internal restructuring; no behaviour change. @impl/done |
| ##ROW-TYPE-PERF `perf` @impl/done | Performance improvement. @impl/done |
| ##ROW-TYPE-STYLE `style` @impl/done | Formatting / whitespace; no semantic change. @impl/done |
| ##ROW-TYPE-CI `ci` @impl/done | CI or pipeline configuration. @impl/done |
| ##ROW-TYPE-REVERT `revert` @impl/done | Revert a previous commit; reference the reverted SHA in the body. @impl/done |

##USE-EXACTLY-ONE-TYPE Use exactly one. @impl/done

##TWO-TYPES-AT-ONCE-MEANS-TWO-COMMITS If a commit feels like two types at once, it is
two commits. @impl/done

### Scope {#scope}

##SCOPE-NAMES-THE-MOST-AFFECTED-SUBSYSTEM Scope names the most affected subsystem — a crate, a package, a
module, a documentation area. @impl/done

##examples-of-scope-from-this-project Examples from this project:
`core`, `install`, `wal`, `registry`, `spec`, `build`. @impl/done

##CHOOSE-THE-NARROWEST-ACCURATE-SCOPE Choose the **narrowest accurate** scope. @impl/done

##the-narrower-form-hits-the-log-filter-correctly `feat(wal): add morning
routine` is better than `feat(core): add wal morning routine`,
because readers filter the log by scope and the narrower form hits
the filter correctly. @spec/done

##scope-is-optional-in-the-grammar-but-omit-it-only-with-reason Scope is optional in the strict Conventional Commits grammar, but
omit it only when the change legitimately has no scope (e.g. a
project-wide `.gitattributes` addition). @spec/done

## Body {#body}

##A-BLANK-LINE-AFTER-THE-SUBJECT-THEN-A-FREE-FORM-BODY A single blank line after the subject, then a free-form body. @impl/done

### What to include {#body-include}

- ##INCLUDE-WHY-THIS-CHANGE-WAS-MADE **Why this change was made.** Link to the spec section, issue,
  measurement, or conversation that drove it. Use `spec://…` URIs
  so future sessions can follow the reference without having to
  guess where it is documented. @impl/done
- ##INCLUDE-WHAT-FOLLOWS-FROM-IT **What follows from it.** Consequences that are invisible in
  the diff: "this unblocks FEAT-007"; "after this, old callers
  must be migrated"; "this is a temporary workaround for #42". @impl/done
- ##INCLUDE-WHAT-WAS-CONSIDERED-AND-REJECTED **What was considered and rejected.** One line each. Future-you
  re-opens this conversation every six months unless the log says
  "we considered adaptive timeout and rejected it because of UX
  unpredictability". @impl/done

### What to skip {#body-skip}

- ##SKIP-WHAT-THE-DIFF-ALREADY-SHOWS **Do not describe what the diff already shows.** "This commit
  adds a function `foo()`" is noise — the diff shows that. The
  message should answer "why did `foo()` need to exist?". @impl/done
- ##SKIP-IMPLEMENTATION-DETAILS-THAT-WILL-CHANGE **Do not include implementation details that will change on
  the next refactor.** They rot faster than the surrounding prose
  and mislead readers once stale. @impl/done

### Body length {#body-length}

##BODY-LENGTH-IS-FREE-FORM Free-form. @impl/done

##A-THREE-LINE-BODY-IS-FINE-FOR-A-SMALL-FIX A three-line body is fine for a small fix. @impl/done

##A-TWENTY-LINE-BODY-IS-FINE-FOR-A-MILESTONE-COMMIT A twenty-line
body is fine for a milestone commit where the reasoning matters. @impl/done

##brevity-at-the-expense-of-the-why-costs-every-future-read Length is not a virtue, but brevity at the expense of the *why* is
a cost paid on every future read. @spec/done

### Body format {#body-format}

##PREFER-PARAGRAPHS-WHEN-THE-REASONING-IS-CONTINUOUS Prefer paragraphs over bullet lists when the reasoning is
continuous. @impl/done

##BULLETS-ARE-FOR-GENUINELY-PARALLEL-ITEMS Bullets are for enumerations of genuinely parallel
items — three rejected alternatives, four affected callers. @impl/done

##DO-NOT-BULLET-A-SINGLE-PARAGRAPH-OF-PROSE Do not
bullet a single paragraph of prose into pieces. @impl/done

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
| ##ROW-ANTI-UPDATES `updates` @impl/done | No type, no scope, no *why*. @impl/done | `docs(spec): add freshness TTL rationale` @impl/done |
| ##ROW-ANTI-WIP `wip` @impl/done | Not a finished thought. @impl/done | Squash into the next real commit. @impl/done |
| ##ROW-ANTI-FIXED-BUG `fixed bug` @impl/done | Nothing learned from the log. @impl/done | Name the bug and what drove the fix. @impl/done |
| ##ROW-ANTI-THREE-IDEAS `feat: add foo, bar, and baz` @impl/done | Three ideas, one commit. @impl/done | Split. @impl/done |
| ##ROW-ANTI-HUGE-REFACTOR `feat(core): huge refactor` @impl/done | Behaviour change rolled into refactor. @impl/done | Two commits: refactor first, feature second. @impl/done |
| ##ROW-ANTI-CAPITALISED-VAGUE `Fix: handle edge case` @impl/done | Capitalised, vague, no scope. @impl/done | `fix(verify): handle empty sender_id` @impl/done |

## Interaction with the git-atomic-commits rule {#atomicity}

##CONVENTIONAL-COMMITS-DOES-NOT-ENFORCE-ATOMICITY Conventional Commits does not by itself enforce atomicity. @impl/done

##A-VALID-MESSAGE-CAN-STILL-VIOLATE-THE-ATOMIC-RULE A commit
with the subject `feat(core): add foo, bar, and baz` is syntactically
valid Conventional Commits *and* a violation of the atomic rule. Both
rules run together: @impl/done

1. ##BOTH-RULES-ONE-IDEA-PER-COMMIT The commit carries exactly one idea (atomic). @impl/done
2. ##BOTH-RULES-THE-CONVENTIONAL-COMMITS-SHAPE The message announces that idea in the Conventional Commits shape
   (`type(scope): subject` + *why* body). @impl/done

##PASS-BOTH-AND-THE-COMMIT-IS-WELL-FORMED Pass both, and the commit is well-formed. @impl/done
