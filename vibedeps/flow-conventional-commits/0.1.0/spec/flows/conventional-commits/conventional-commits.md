# Commit message format {#root}

All commits in this project follow the [Conventional Commits](https://www.conventionalcommits.org/)
specification. This document is the message shape, body structure,
and scope convention each commit must carry.

## Header {#header}

```
type(scope): short imperative subject line
```

- **Target length:** ≤ 60 characters. **Hard limit:** 72. Git web
  UIs truncate beyond that, and a truncated subject on the commit
  list is how decisions become invisible to readers who scan rather
  than scroll.
- **Imperative mood.** "add", not "added"; "fix", not "fixes"; "refactor",
  not "refactored". The subject completes the sentence *"If applied,
  this commit will …"*.
- **Lowercase.** Including the first word after the `type(scope):`
  prefix. The typed prefix is the visual anchor; a capitalised
  first word competes with it for attention.

### Allowed types {#types}

| Type       | When to use |
|------------|-------------|
| `feat`     | New user-visible functionality. |
| `fix`      | Bug fix. Name what broke and what it now does. |
| `chore`    | Housekeeping with no behaviour change. |
| `docs`     | Documentation, including spec updates. |
| `build`    | Build system, external dependency, toolchain pin. |
| `test`     | Add or fix tests; no production-code change. |
| `refactor` | Internal restructuring; no behaviour change. |
| `perf`     | Performance improvement. |
| `style`    | Formatting / whitespace; no semantic change. |
| `ci`       | CI or pipeline configuration. |
| `revert`   | Revert a previous commit; reference the reverted SHA in the body. |

Use exactly one. If a commit feels like two types at once, it is
two commits.

### Scope {#scope}

Scope names the most affected subsystem — a crate, a package, a
module, a documentation area. Examples from this project:
`core`, `install`, `wal`, `registry`, `spec`, `build`.

Choose the **narrowest accurate** scope. `feat(wal): add morning
routine` is better than `feat(core): add wal morning routine`,
because readers filter the log by scope and the narrower form hits
the filter correctly.

Scope is optional in the strict Conventional Commits grammar, but
omit it only when the change legitimately has no scope (e.g. a
project-wide `.gitattributes` addition).

## Body {#body}

A single blank line after the subject, then a free-form body.

### What to include {#body-include}

- **Why this change was made.** Link to the spec section, issue,
  measurement, or conversation that drove it. Use `spec://…` URIs
  so future sessions can follow the reference without having to
  guess where it is documented.
- **What follows from it.** Consequences that are invisible in
  the diff: "this unblocks FEAT-007"; "after this, old callers
  must be migrated"; "this is a temporary workaround for #42".
- **What was considered and rejected.** One line each. Future-you
  re-opens this conversation every six months unless the log says
  "we considered adaptive timeout and rejected it because of UX
  unpredictability".

### What to skip {#body-skip}

- **Do not describe what the diff already shows.** "This commit
  adds a function `foo()`" is noise — the diff shows that. The
  message should answer "why did `foo()` need to exist?".
- **Do not include implementation details that will change on
  the next refactor.** They rot faster than the surrounding prose
  and mislead readers once stale.

### Body length {#body-length}

Free-form. A three-line body is fine for a small fix. A twenty-line
body is fine for a milestone commit where the reasoning matters.
Length is not a virtue, but brevity at the expense of the *why* is
a cost paid on every future read.

### Body format {#body-format}

Prefer paragraphs over bullet lists when the reasoning is
continuous. Bullets are for enumerations of genuinely parallel
items — three rejected alternatives, four affected callers. Do not
bullet a single paragraph of prose into pieces.

## Worked examples {#examples}

### Small fix

```
fix(wal): stop crashing on missing _Updated line

Treat a WAL without `_Updated:` as "age = infinite" rather than
panicking. The guard previously short-circuited the whole status
check, so stale-WAL projects could not run `vibe check`. Tested
against an empty WAL and a WAL whose first line is a stray comment.
```

### Feature

```
feat(registry): freshness TTL for cloned mirrors

Cache under ~/.vibe/registries/<hash>/ now carries a meta.toml
with last_pulled_at. Pulls skip when the cache is fresher than
the configured TTL (default 1 h); `vibe registry sync` forces a
pull regardless. Rationale: every `vibe install` hitting the
network was making offline work painful and slow.

Cited by spec://vibevm/modules/vibe-registry/PROP-001#freshness.
```

### Refactor

```
refactor(core): hoist timestamp helper out of vibe-cli

Both vibe-cli and vibe-registry needed `now_unix_utc`. Keeping it
in vibe-cli forced vibe-registry to depend on the CLI crate, which
violates the intended dependency direction. Moved to
`vibe_core::timestamp`. No behaviour change; 78 tests still green.
```

### Revert

```
revert: "feat(install): auto-retry on 429"

Reverts 1a2b3c4. The retry loop was masking real rate-limit bugs
in the registry layer. Correct fix is to surface the 429 upstream
and let the caller decide. Issue #47.
```

### Docs sync (from sync-from-code flow)

```
docs(spec): sync timeout to 600s in PROP-003 §verification.timeout

Code changed TIMEOUT from 300 s to 600 s after VPN latency
measurement (2026-03-05, 847 messages, 128 users). Spec now
carries the new value, the reason, and the revisit trigger.
```

## Anti-patterns {#antipatterns}

| Bad subject                       | Why it fails                        | Fix                                         |
|-----------------------------------|-------------------------------------|---------------------------------------------|
| `updates`                         | No type, no scope, no *why*.        | `docs(spec): add freshness TTL rationale`   |
| `wip`                             | Not a finished thought.             | Squash into the next real commit.           |
| `fixed bug`                       | Nothing learned from the log.       | Name the bug and what drove the fix.        |
| `feat: add foo, bar, and baz`     | Three ideas, one commit.            | Split.                                      |
| `feat(core): huge refactor`       | Behaviour change rolled into refactor. | Two commits: refactor first, feature second. |
| `Fix: handle edge case`           | Capitalised, vague, no scope.       | `fix(verify): handle empty sender_id`       |

## Interaction with the atomic-commits rule {#atomicity}

Conventional Commits does not by itself enforce atomicity. A commit
with the subject `feat(core): add foo, bar, and baz` is syntactically
valid Conventional Commits *and* a violation of the atomic rule. Both
rules run together:

1. The commit carries exactly one idea (atomic).
2. The message announces that idea in the Conventional Commits shape
   (`type(scope): subject` + *why* body).

Pass both, and the commit is well-formed.
