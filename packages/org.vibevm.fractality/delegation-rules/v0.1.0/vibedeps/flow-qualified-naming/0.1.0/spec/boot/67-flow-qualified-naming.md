# Flow: Qualified Naming {#root}

This project ships the **qualified-naming** practice for *ecosystem
designers* — anyone defining a namespace for packages, plugins,
extensions, or artifacts. It is a design discipline, not a runtime
rule: read it once while shaping identifiers, not on every session.

## When this applies {#when}

When you design any user-facing namespace — a package registry, a
plugin id scheme, an artifact coordinate, an extension marketplace —
read [`QUALIFIED-NAMING-PROTOCOL.md`](../flows/qualified-naming/QUALIFIED-NAMING-PROTOCOL.md)
**before the first name is minted**. Retrofitting a group onto a
shipped flat namespace is a migration; getting it right first is free.

## The laws in one breath {#laws}

- Every artifact carries a **group**; identity is the tuple
  `(group, name, version, content-hash)`, and `(group, name)` is
  globally unique.
- A **rename is a new identity** — versions never transfer, and no
  `name@version` coordinate is ever reused for different content.
- **Short names resolve only at the human CLI boundary**, once,
  against an index; manifests and lockfiles store the qualified form.
- A **collision** (one short name, two groups) and a **conflict** (a
  version contradiction) are distinct failures with distinct
  machine-readable identities.

Grammar and forms: [`ref-grammar.md`](../flows/qualified-naming/ref-grammar.md).
Fork-by-fork rationale: [`naming-forks.md`](../flows/qualified-naming/naming-forks.md).

## Never {#never}

- Never store a short (unqualified) name in a manifest, lockfile, or
  dependency graph — it is CLI sugar, nothing more.
- Never reuse a `name@version` coordinate for different content: a
  coordinate that meant one artifact must never mean another.
- Never resolve a naming ambiguity interactively — fail with the
  candidate list and let a human record the qualified form.
- Never treat a change of group or name as a rename — it is a new
  package, and versions do not carry over.
