# Scope Discipline {#root}

<status stage="spec" state="done"/>

@fact:scope-of-this-document **Scope of this document.** This file defines the *never-escalate*
law for integrations that act under a credential: *what* it means for
an integration to stay inside its declared scope, *how* an explicit
prefix check enforces it, *why* an escalation is an error rather than
a warning, how *trust ordering* between sources keeps a lower-trust
answer from overriding a higher-trust one, and *why* these rules live
globally rather than per-module. @status:impl/done

@fact:sibling-document-pointers It is the companion to the four laws
in [`SECRETS-HYGIENE-PROTOCOL.md`](SECRETS-HYGIENE-PROTOCOL.md). @status:impl/done

## The never-escalate law {#never-escalate}

@fact:AN-INTEGRATION-IS-BOUND-TO-ITS-GRANTED-SCOPE An integration that operates on behalf of a credential is bound to
the **scope that credential was granted for**, and must refuse to
touch anything outside it. @status:impl/done

@fact:act-on-the-declared-target-and-nowhere-else-lead If the project declares that publishing
targets organization `X` on some host, the integration that creates,
updates, or even *probes* repositories may act on `X` and nowhere
else: @status:impl/done

- @fact:SCOPE-NO-OTHER-ORGANIZATIONS **No other organizations** on the same host. @status:impl/done
- @fact:SCOPE-NO-USER-NAMESPACES **No user namespaces** on the same host. @status:impl/done
- @fact:SCOPE-NO-UNRELATED-ENDPOINTS **No endpoints** unrelated to the target — no account-wide reads,
  no listing of resources outside `X`. @status:impl/done

@fact:a-credential-may-technically-reach-further The credential may technically be able to reach further — a token
scoped broadly can write to many repos. @status:spec/done

@fact:TECHNICAL-REACH-IS-NOT-PERMISSION Technical reach is not
permission. @status:impl/done

@fact:THE-DECLARED-SCOPE-IS-THE-BOUNDARY The declared scope is the boundary, and the integration
enforces it even when the credential would allow more. @status:impl/done

## Explicit prefix checks {#prefix-checks}

@fact:THE-LAW-IS-ENFORCED-BY-AN-EXPLICIT-CHECK-IN-THE-ADAPTER The law is enforced by an **explicit check in the adapter**, not by
convention or by hoping the target is right. @status:impl/done

@fact:prefix-comparison-lead Before an integration
acts, it computes the target's owning scope and compares it against
the declared scope by prefix: @status:impl/done

```
target = "host.example/acme-org/some-repo"
declared_scope = "host.example/acme-org"

if not target.starts_with(declared_scope + "/"):
    raise ScopeError(target, declared_scope)   # refuse, do not warn
```

@fact:THE-CHECK-IS-A-GUARD-AT-THE-BOUNDARY The check is a **guard at the boundary** — the single point where the
integration is about to act on a resource. @status:impl/done

@fact:the-check-runs-on-every-action-lead It runs on every action: @status:impl/done

- @fact:GUARDED-ACTION-CREATE create, @status:impl/done
- @fact:GUARDED-ACTION-MODIFY modify, @status:impl/done
- @fact:GUARDED-ACTION-DELETE delete, @status:impl/done
- @fact:GUARDED-ACTION-PROBE probe. @status:impl/done

@fact:AN-UNGUARDED-CODE-PATH-IS-A-BUG A code path that reaches a host
endpoint without passing the check is a bug, caught in review, on the
same footing as a code path that prints a secret value. @status:impl/done

## Escalation is an error, not a warning {#error-not-warning}

@fact:A-FAILED-CHECK-RAISES-AN-ERROR-AND-REFUSES-THE-ACTION When the prefix check fails, the integration **raises an error and
refuses the action**. @status:impl/done

@fact:IT-DOES-NOT-WARN-AND-PROCEED It does not log a warning and proceed; it does
not act "just this once" against an out-of-scope target. @status:impl/done

@fact:THERE-IS-NO-RUNTIME-OVERRIDE-FLAG There is no
override flag that quietly widens the boundary at runtime. @status:impl/done

@fact:the-reason-is-the-blast-radius The reason is the blast radius. @status:spec/done

@fact:a-warning-is-indistinguishable-from-success A warning that scrolls past in a log
is indistinguishable from success to an automated run — and an
out-of-scope write has already happened by the time anyone reads the
warning. @status:spec/done

@fact:AN-ERROR-STOPS-BEFORE-THE-BOUNDARY-IS-CROSSED An error stops the action *before* the boundary is crossed,
which is the only point where stopping is still cheap. @status:impl/done

@fact:WIDENING-SCOPE-IS-AN-OWNER-CONFIGURATION-CHANGE Widening the
scope is a deliberate configuration change to the declared scope, made
by the owner, reviewed like any other change — never a runtime
decision made by the adapter under pressure. @status:impl/done

## Trust ordering between sources {#trust-ordering}

@fact:SCOPE-DISCIPLINE-EXTENDS-TO-READING Scope discipline extends to *reading*, not only writing. @status:impl/done

@fact:trust-order-highest-first-lead When the
same question can be answered by several sources of differing trust —
a primary registry and a mirror, a canonical index and a fallback —
the sources are consulted in **trust order**, highest first, and the
first source with a valid answer **wins outright**: @status:impl/done

- @fact:TRUST-ITERATE-IN-PRIORITY-ORDER The resolver iterates sources in priority order. @status:impl/done
- @fact:TRUST-FIRST-VALID-ANSWER-TERMINATES The first that returns a satisfying answer terminates the search. @status:impl/done
- @fact:TRUST-ANSWERS-ARE-NOT-MERGED Answers are **not merged** across sources of different trust. @status:impl/done

@fact:this-prevents-a-lower-trust-source-from-influencing This prevents a lower-trust source from influencing a result when a
higher-trust source already has a valid answer. @status:impl/done

@fact:unioning-would-let-a-malicious-mirror-inject-an-entry If answers were
unioned, a compromised or malicious low-trust mirror could inject an
entry — a poisoned version, a redirect to an attacker's artifact —
into a resolution that a trusted source had already settled. @status:spec/done

@fact:TRUST-ORDERING-CLOSES-THAT-SURFACE Trust
ordering closes that: once the trusted source answers, the untrusted
one is never consulted for that question, so it has no surface to
influence. @status:impl/done

| Merge strategy | What a malicious low-trust source can do |
|----------------|------------------------------------------|
| @fact:ROW-MERGE-UNION-ACROSS-SOURCES Union across sources @status:spec/done | Inject an entry into any resolution — a poisoned version wins if it sorts first @status:spec/done |
| @fact:ROW-MERGE-FIRST-TRUSTED-WINS First-trusted-wins (this rule) @status:spec/done | Nothing, once a trusted source has answered — it is never consulted @status:spec/done |

## Why scope rules live globally {#global}

@fact:SCOPE-DISCIPLINE-IS-A-GLOBAL-INVARIANT Like the four laws, scope discipline is a **global invariant**, not a
per-module convention. @status:impl/done

@fact:the-reason-is-again-blast-radius The reason is again blast radius: an escalated
integration is not a bounded, local failure — it is the **whole host
account**, every resource the credential can reach. @status:spec/done

@fact:one-forgotten-check-is-enough-to-realize-it A single adapter
that forgot its prefix check, or a single resolver path that merged an
untrusted source, is enough to realize that failure. @status:spec/done

@fact:a-catastrophic-rule-cannot-live-per-module A rule that must hold on *every* path, whose violation on *any* path
is catastrophic, cannot live in each module's local discipline and be
trusted to have been reimplemented correctly everywhere. @status:spec/done

@fact:IT-LIVES-ONCE-GLOBALLY-AND-EVERY-PATH-IS-AUDITED It lives once,
globally, and every code path that acts under a credential or resolves
across trust boundaries is audited against it. @status:impl/done

@fact:module-local-rules-bound-module-local-blast Module-local rules bound
module-local blast; these failures are not module-local. @status:spec/done

## Summary {#summary}

- @fact:SUM-ACT-ONLY-INSIDE-THE-DECLARED-SCOPE An integration acts only inside the scope its credential was
  declared for — no other orgs, no user namespaces, no unrelated
  endpoints. Technical reach is not permission. @status:impl/done
- @fact:SUM-ENFORCEMENT-IS-AN-EXPLICIT-PREFIX-CHECK Enforcement is an explicit prefix check at the boundary, run on
  every action. @status:impl/done
- @fact:SUM-A-VIOLATION-IS-AN-ERROR-NOT-A-WARNING A scope violation is an **error that refuses the action**, never a
  warning that proceeds. Widening scope is an owner configuration
  change, not a runtime decision. @status:impl/done
- @fact:SUM-TRUST-ORDERING-HIGHEST-FIRST-NEVER-MERGE Trust ordering: consult sources highest-trust first, first valid
  answer wins, never merge — so a low-trust source cannot influence a
  resolution a trusted source has settled. @status:impl/done
- @fact:SUM-GLOBAL-BECAUSE-THE-BLAST-RADIUS-IS-THE-ACCOUNT These rules are global because the blast radius is the whole
  account; every credential-bearing path is audited against them. @status:spec/done
