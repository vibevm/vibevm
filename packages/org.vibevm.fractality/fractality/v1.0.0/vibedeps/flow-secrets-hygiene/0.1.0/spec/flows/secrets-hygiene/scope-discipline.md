# Scope Discipline {#root}

**Scope of this document.** This file defines the *never-escalate*
law for integrations that act under a credential: *what* it means for
an integration to stay inside its declared scope, *how* an explicit
prefix check enforces it, *why* an escalation is an error rather than
a warning, how *trust ordering* between sources keeps a lower-trust
answer from overriding a higher-trust one, and *why* these rules live
globally rather than per-module. It is the companion to the four laws
in [`SECRETS-HYGIENE-PROTOCOL.md`](SECRETS-HYGIENE-PROTOCOL.md).

## The never-escalate law {#never-escalate}

An integration that operates on behalf of a credential is bound to
the **scope that credential was granted for**, and must refuse to
touch anything outside it. If the project declares that publishing
targets organization `X` on some host, the integration that creates,
updates, or even *probes* repositories may act on `X` and nowhere
else:

- **No other organizations** on the same host.
- **No user namespaces** on the same host.
- **No endpoints** unrelated to the target — no account-wide reads,
  no listing of resources outside `X`.

The credential may technically be able to reach further — a token
scoped broadly can write to many repos. Technical reach is not
permission. The declared scope is the boundary, and the integration
enforces it even when the credential would allow more.

## Explicit prefix checks {#prefix-checks}

The law is enforced by an **explicit check in the adapter**, not by
convention or by hoping the target is right. Before an integration
acts, it computes the target's owning scope and compares it against
the declared scope by prefix:

```
target = "host.example/acme-org/some-repo"
declared_scope = "host.example/acme-org"

if not target.starts_with(declared_scope + "/"):
    raise ScopeError(target, declared_scope)   # refuse, do not warn
```

The check is a **guard at the boundary** — the single point where the
integration is about to act on a resource. It runs on every action:
create, modify, delete, probe. A code path that reaches a host
endpoint without passing the check is a bug, caught in review, on the
same footing as a code path that prints a secret value.

## Escalation is an error, not a warning {#error-not-warning}

When the prefix check fails, the integration **raises an error and
refuses the action**. It does not log a warning and proceed; it does
not act "just this once" against an out-of-scope target. There is no
override flag that quietly widens the boundary at runtime.

The reason is the blast radius. A warning that scrolls past in a log
is indistinguishable from success to an automated run — and an
out-of-scope write has already happened by the time anyone reads the
warning. An error stops the action *before* the boundary is crossed,
which is the only point where stopping is still cheap. Widening the
scope is a deliberate configuration change to the declared scope, made
by the owner, reviewed like any other change — never a runtime
decision made by the adapter under pressure.

## Trust ordering between sources {#trust-ordering}

Scope discipline extends to *reading*, not only writing. When the
same question can be answered by several sources of differing trust —
a primary registry and a mirror, a canonical index and a fallback —
the sources are consulted in **trust order**, highest first, and the
first source with a valid answer **wins outright**:

- The resolver iterates sources in priority order.
- The first that returns a satisfying answer terminates the search.
- Answers are **not merged** across sources of different trust.

This prevents a lower-trust source from influencing a result when a
higher-trust source already has a valid answer. If answers were
unioned, a compromised or malicious low-trust mirror could inject an
entry — a poisoned version, a redirect to an attacker's artifact —
into a resolution that a trusted source had already settled. Trust
ordering closes that: once the trusted source answers, the untrusted
one is never consulted for that question, so it has no surface to
influence.

| Merge strategy | What a malicious low-trust source can do |
|----------------|------------------------------------------|
| Union across sources | Inject an entry into any resolution — a poisoned version wins if it sorts first |
| First-trusted-wins (this rule) | Nothing, once a trusted source has answered — it is never consulted |

## Why scope rules live globally {#global}

Like the four laws, scope discipline is a **global invariant**, not a
per-module convention. The reason is again blast radius: an escalated
integration is not a bounded, local failure — it is the **whole host
account**, every resource the credential can reach. A single adapter
that forgot its prefix check, or a single resolver path that merged an
untrusted source, is enough to realize that failure.

A rule that must hold on *every* path, whose violation on *any* path
is catastrophic, cannot live in each module's local discipline and be
trusted to have been reimplemented correctly everywhere. It lives once,
globally, and every code path that acts under a credential or resolves
across trust boundaries is audited against it. Module-local rules bound
module-local blast; these failures are not module-local.

## Summary {#summary}

- An integration acts only inside the scope its credential was
  declared for — no other orgs, no user namespaces, no unrelated
  endpoints. Technical reach is not permission.
- Enforcement is an explicit prefix check at the boundary, run on
  every action.
- A scope violation is an **error that refuses the action**, never a
  warning that proceeds. Widening scope is an owner configuration
  change, not a runtime decision.
- Trust ordering: consult sources highest-trust first, first valid
  answer wins, never merge — so a low-trust source cannot influence a
  resolution a trusted source has settled.
- These rules are global because the blast radius is the whole
  account; every credential-bearing path is audited against them.
