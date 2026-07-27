# Scope Discipline {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** This file defines the *never-escalate*
law for integrations that act under a credential: *what* it means for
an integration to stay inside its declared scope, *how* an explicit
prefix check enforces it, *why* an escalation is an error rather than
a warning, how *trust ordering* between sources keeps a lower-trust
answer from overriding a higher-trust one, and *why* these rules live
globally rather than per-module. @impl/done

##sibling-document-pointers It is the companion to the four laws
in [`SECRETS-HYGIENE-PROTOCOL.md`](SECRETS-HYGIENE-PROTOCOL.md). @impl/done

## The never-escalate law {#never-escalate}

##AN-INTEGRATION-IS-BOUND-TO-ITS-GRANTED-SCOPE An integration that operates on behalf of a credential is bound to
the **scope that credential was granted for**, and must refuse to
touch anything outside it. @impl/done

##act-on-the-declared-target-and-nowhere-else-lead If the project declares that publishing
targets organization `X` on some host, the integration that creates,
updates, or even *probes* repositories may act on `X` and nowhere
else: @impl/done

- ##SCOPE-NO-OTHER-ORGANIZATIONS **No other organizations** on the same host. @impl/done
- ##SCOPE-NO-USER-NAMESPACES **No user namespaces** on the same host. @impl/done
- ##SCOPE-NO-UNRELATED-ENDPOINTS **No endpoints** unrelated to the target — no account-wide reads,
  no listing of resources outside `X`. @impl/done

##a-credential-may-technically-reach-further The credential may technically be able to reach further — a token
scoped broadly can write to many repos. @spec/done

##TECHNICAL-REACH-IS-NOT-PERMISSION Technical reach is not
permission. @impl/done

##THE-DECLARED-SCOPE-IS-THE-BOUNDARY The declared scope is the boundary, and the integration
enforces it even when the credential would allow more. @impl/done

## Explicit prefix checks {#prefix-checks}

##THE-LAW-IS-ENFORCED-BY-AN-EXPLICIT-CHECK-IN-THE-ADAPTER The law is enforced by an **explicit check in the adapter**, not by
convention or by hoping the target is right. @impl/done

##prefix-comparison-lead Before an integration
acts, it computes the target's owning scope and compares it against
the declared scope by prefix: @impl/done

```
target = "host.example/acme-org/some-repo"
declared_scope = "host.example/acme-org"

if not target.starts_with(declared_scope + "/"):
    raise ScopeError(target, declared_scope)   # refuse, do not warn
```

##THE-CHECK-IS-A-GUARD-AT-THE-BOUNDARY The check is a **guard at the boundary** — the single point where the
integration is about to act on a resource. @impl/done

##the-check-runs-on-every-action-lead It runs on every action: @impl/done

- ##GUARDED-ACTION-CREATE create, @impl/done
- ##GUARDED-ACTION-MODIFY modify, @impl/done
- ##GUARDED-ACTION-DELETE delete, @impl/done
- ##GUARDED-ACTION-PROBE probe. @impl/done

##AN-UNGUARDED-CODE-PATH-IS-A-BUG A code path that reaches a host
endpoint without passing the check is a bug, caught in review, on the
same footing as a code path that prints a secret value. @impl/done

## Escalation is an error, not a warning {#error-not-warning}

##A-FAILED-CHECK-RAISES-AN-ERROR-AND-REFUSES-THE-ACTION When the prefix check fails, the integration **raises an error and
refuses the action**. @impl/done

##IT-DOES-NOT-WARN-AND-PROCEED It does not log a warning and proceed; it does
not act "just this once" against an out-of-scope target. @impl/done

##THERE-IS-NO-RUNTIME-OVERRIDE-FLAG There is no
override flag that quietly widens the boundary at runtime. @impl/done

##the-reason-is-the-blast-radius The reason is the blast radius. @spec/done

##a-warning-is-indistinguishable-from-success A warning that scrolls past in a log
is indistinguishable from success to an automated run — and an
out-of-scope write has already happened by the time anyone reads the
warning. @spec/done

##AN-ERROR-STOPS-BEFORE-THE-BOUNDARY-IS-CROSSED An error stops the action *before* the boundary is crossed,
which is the only point where stopping is still cheap. @impl/done

##WIDENING-SCOPE-IS-AN-OWNER-CONFIGURATION-CHANGE Widening the
scope is a deliberate configuration change to the declared scope, made
by the owner, reviewed like any other change — never a runtime
decision made by the adapter under pressure. @impl/done

## Trust ordering between sources {#trust-ordering}

##SCOPE-DISCIPLINE-EXTENDS-TO-READING Scope discipline extends to *reading*, not only writing. @impl/done

##trust-order-highest-first-lead When the
same question can be answered by several sources of differing trust —
a primary registry and a mirror, a canonical index and a fallback —
the sources are consulted in **trust order**, highest first, and the
first source with a valid answer **wins outright**: @impl/done

- ##TRUST-ITERATE-IN-PRIORITY-ORDER The resolver iterates sources in priority order. @impl/done
- ##TRUST-FIRST-VALID-ANSWER-TERMINATES The first that returns a satisfying answer terminates the search. @impl/done
- ##TRUST-ANSWERS-ARE-NOT-MERGED Answers are **not merged** across sources of different trust. @impl/done

##this-prevents-a-lower-trust-source-from-influencing This prevents a lower-trust source from influencing a result when a
higher-trust source already has a valid answer. @impl/done

##unioning-would-let-a-malicious-mirror-inject-an-entry If answers were
unioned, a compromised or malicious low-trust mirror could inject an
entry — a poisoned version, a redirect to an attacker's artifact —
into a resolution that a trusted source had already settled. @spec/done

##TRUST-ORDERING-CLOSES-THAT-SURFACE Trust
ordering closes that: once the trusted source answers, the untrusted
one is never consulted for that question, so it has no surface to
influence. @impl/done

| Merge strategy | What a malicious low-trust source can do |
|----------------|------------------------------------------|
| ##ROW-MERGE-UNION-ACROSS-SOURCES Union across sources @spec/done | Inject an entry into any resolution — a poisoned version wins if it sorts first @spec/done |
| ##ROW-MERGE-FIRST-TRUSTED-WINS First-trusted-wins (this rule) @spec/done | Nothing, once a trusted source has answered — it is never consulted @spec/done |

## Why scope rules live globally {#global}

##SCOPE-DISCIPLINE-IS-A-GLOBAL-INVARIANT Like the four laws, scope discipline is a **global invariant**, not a
per-module convention. @impl/done

##the-reason-is-again-blast-radius The reason is again blast radius: an escalated
integration is not a bounded, local failure — it is the **whole host
account**, every resource the credential can reach. @spec/done

##one-forgotten-check-is-enough-to-realize-it A single adapter
that forgot its prefix check, or a single resolver path that merged an
untrusted source, is enough to realize that failure. @spec/done

##a-catastrophic-rule-cannot-live-per-module A rule that must hold on *every* path, whose violation on *any* path
is catastrophic, cannot live in each module's local discipline and be
trusted to have been reimplemented correctly everywhere. @spec/done

##IT-LIVES-ONCE-GLOBALLY-AND-EVERY-PATH-IS-AUDITED It lives once,
globally, and every code path that acts under a credential or resolves
across trust boundaries is audited against it. @impl/done

##module-local-rules-bound-module-local-blast Module-local rules bound
module-local blast; these failures are not module-local. @spec/done

## Summary {#summary}

- ##SUM-ACT-ONLY-INSIDE-THE-DECLARED-SCOPE An integration acts only inside the scope its credential was
  declared for — no other orgs, no user namespaces, no unrelated
  endpoints. Technical reach is not permission. @impl/done
- ##SUM-ENFORCEMENT-IS-AN-EXPLICIT-PREFIX-CHECK Enforcement is an explicit prefix check at the boundary, run on
  every action. @impl/done
- ##SUM-A-VIOLATION-IS-AN-ERROR-NOT-A-WARNING A scope violation is an **error that refuses the action**, never a
  warning that proceeds. Widening scope is an owner configuration
  change, not a runtime decision. @impl/done
- ##SUM-TRUST-ORDERING-HIGHEST-FIRST-NEVER-MERGE Trust ordering: consult sources highest-trust first, first valid
  answer wins, never merge — so a low-trust source cannot influence a
  resolution a trusted source has settled. @impl/done
- ##SUM-GLOBAL-BECAUSE-THE-BLAST-RADIUS-IS-THE-ACCOUNT These rules are global because the blast radius is the whole
  account; every credential-bearing path is audited against them. @spec/done
