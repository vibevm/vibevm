# Qualified Naming Protocol {#root}

**Scope of this document.** This file defines the laws that make a
package namespace scale: why a flat namespace fails, the mandatory
*group*, the identity tuple, why a rename is a new identity, why short
names may live only at the human boundary, and why a *collision* and a
*conflict* are two different failures. It is written for **ecosystem
designers** — anyone minting the namespace for packages, plugins,
extensions, or artifacts. Reference grammar: [`ref-grammar.md`](ref-grammar.md);
the fork-by-fork rationale: [`naming-forks.md`](naming-forks.md).

## The problem: flat names {#problem}

A flat namespace is one where an artifact is addressed by a single
bare `name`, unique across the whole registry. It reads beautifully on
day one (`install wal`) and fails in three predictable ways:

- **Squatting.** The good short names are a finite commons. The first
  arrival takes `http`, `json`, `auth`; everyone after fights over
  `http2`, `json-fast`, `auth-real`. The namespace rewards land-grab
  speed, not quality.
- **Trust ambiguity.** In a flat namespace `logger` has no owner you
  can name. Is this the `logger` you audited last week, or a
  same-named replacement someone else published? Nothing in the
  coordinate answers, so trust cannot be delegated — it must be
  re-established per artifact.
- **Transitive collisions.** Two dependencies deep in your graph both
  want the bare name `utils`, meaning different things. A flat resolver
  cannot satisfy both; the graph is unbuildable, and the failure
  surfaces far from either author.

Every law below is a structural fix for one of these, not a
convenience. A group-qualified namespace makes squatting local, trust
delegable, and transitive collisions impossible by construction.

## Law 1 — every artifact carries a group {#group}

Identity begins with a **mandatory** `group`: a dot-separated string
of lowercase segments (`com.example.shop`, `io.acme`, `org.vibevm`).
The group is the unit of ownership; `name` is unique *within* it, so
two owners may both ship a `wal` without colliding.

- **Uniqueness is delegated.** The registry guarantees groups are
  distinct; each group's owner guarantees names are distinct inside
  it. Global uniqueness falls out of two local guarantees — no central
  arbiter of every short name.
- **Reverse-FQDN is recommended, not enforced.** Writing the group as
  a reversed domain (`org.example` for `example.org`) piggybacks on
  DNS's existing global uniqueness — a convention Sun introduced for
  Java packages in 1995 for exactly this reason. But whether a group
  *looks* like a reversed domain is **style**: a matter for humans and
  linters. The resolver checks only two things — that the group is
  well-formed grammar, and that it is unique. It never demands you own
  the domain.
- **Grammar is the only hard rule.** Segments are `[a-z0-9_-]+`, ASCII
  lowercase, dot-separated. That is enforced. Taste is not.

Making the group mandatory (rather than optional) removes a grey zone:
there is no "has a group" versus "no group" fork to reason about — every
artifact is qualified, always.

## Law 2 — identity is a tuple {#identity}

An artifact's identity is the tuple **`(group, name, version,
content-hash)`**. Two consequences carry the whole system:

- **`(group, name)` is globally unique.** It names *the package* across
  all its versions. Any type tag (a `kind` such as `flow`, `plugin`,
  `lib`) is metadata — it may help placement or filtering, but it is
  **not** part of identity and never disambiguates two packages.
- **`content-hash` pins the bytes.** `(group, name, version)` names a
  release; the hash proves which bytes that release is. A mirror in a
  different registry serving the same bytes is the *same* identity; the
  registry URL is a fetch detail, not part of who the artifact is.

Because identity is a tuple and not a string, the coordinate carries
its own ownership (`group`) and its own integrity (`content-hash`).
That is what lets trust be delegated: you trust `io.acme`, so you trust
every `io.acme/*` name, without re-auditing each one.

## Law 3 — a rename is a new identity {#rename}

Change the `group` or the `name`, and you have a **new package** — not
a renamed one. This is not a policy choice; it follows from Law 2: the
identity tuple changed, so the identity changed.

- **Versions never transfer.** `io.acme/logger` at `2.3.0` does not
  make `io.acme/log` start at `2.3.0`. The new name starts its own
  version line. History stays attached to the coordinate that earned
  it.
- **No coordinate is ever reused for different content — ever.** Once
  `com.example.shop/cart@1.4.0` has meant one artifact, that exact
  `name@version` must never resolve to different bytes for anyone,
  forever. A consumer who locked `1.4.0` locked a specific meaning;
  silently repointing it is the one betrayal a package system must make
  impossible. Yank a bad release, publish a `1.4.1` — but never let the
  old coordinate mean something new.

The rejected alternative — an alias table mapping the old name to the
new — is examined in [`naming-forks.md` §rename](naming-forks.md#rename).
It loses because an alias re-introduces exactly the ambiguity the group
removed: now two coordinates name one artifact, and every reader must
know the mapping to trust what they read.

## Law 4 — short names live only at the boundary {#short-names}

A **short name** is the bare, unqualified `name` a human types
(`install wal`). It is a convenience, and it is legal in exactly one
place: **the human-typed CLI input boundary**, resolved **once** against
an index of `(group, name)` candidates.

- **Never stored.** The moment a short name is resolved, the tool
  writes the *qualified* form (`org.vibevm/wal`) into the manifest and
  lockfile. Persisted state is qualified-only.
- **Never resolved recursively.** Resolution happens for a human's
  argument and nothing else. The dependency graph is built entirely
  from qualified names, because every author published through the same
  boundary and stored the qualified form.

This single rule is what makes **transitive collisions impossible by
construction**. A short name can only be ambiguous at the one place a
human is present to disambiguate it; it can never be ambiguous three
levels deep in a graph, because no short name ever reaches that far. The
cargo/npm pattern is the same instinct: `add serde` on the command
line, `serde = "1"` in the manifest.

## Law 5 — collision and conflict are distinct failures {#collision}

Two failures look similar and must never be merged:

| Failure | Cause | Resolution |
|---|---|---|
| **Collision** | one short name matches two *different* packages (different groups) | the human picks a group and records the qualified form |
| **Conflict** | version requirements cannot all be satisfied — contradictory constraints, a declared incompatibility, an unsatisfiable diamond | the human relaxes a constraint or drops a dependency |

- **Distinct machine-readable identities.** Each failure gets its own
  exit code and its own error type, so a script — or an agent — can
  branch on *which* failure occurred without parsing prose. The specific
  numbers are an implementation's choice; the law is only that the two
  differ and are stable.
- **No interactive pick on a collision.** When a short name is
  ambiguous, the tool prints *all* candidates with their exact
  qualified forms and **fails**. It does not offer an arrow-key menu.
  The choice must be *recorded deliberately* — edited into the manifest
  by a human — not clicked once and forgotten. A clicked choice leaves
  no trace of why; a recorded qualified name is self-documenting.

Error-shape detail — what a good collision message must contain — is in
[`ref-grammar.md` §errors](ref-grammar.md#errors).

## Re-derive for your project {#re-derive}

Do not copy this document's example groups — copy the *task*, and let
the agent derive the namespace your ecosystem actually needs:

```
Read spec/flows/qualified-naming/ in full, then design the namespace
for THIS ecosystem:
1. Name the artifacts it distributes (packages, plugins, extensions)
   and who owns each — the owner set is your group set.
2. Choose a group grammar and a recommended style (reverse-FQDN or
   other). State plainly what is enforced vs merely recommended.
3. Define the identity tuple and the reference grammar: separators for
   type / group / version, and which forms are legal where.
4. State the storage rule (qualified-only in manifests and locks) and
   the boundary rule (short names resolved once, at CLI input).
5. Specify collision vs conflict as two failures with two distinct,
   stable machine identities, each with an example message.
Show me the design as a short spec. Change nothing in code yet.
```

## Summary {#summary}

- Flat names fail three ways: squatting, trust ambiguity, transitive
  collisions. Groups fix all three structurally.
- Every artifact carries a mandatory **group**; identity is
  `(group, name, version, content-hash)` and `(group, name)` is unique.
- Reverse-FQDN is recommended style, not enforced law — the resolver
  checks grammar and uniqueness, nothing about taste.
- A rename is a **new identity**: versions never transfer, and no
  `name@version` coordinate is ever reused for different content.
- Short names live **only** at the human CLI boundary, resolved once;
  manifests and locks store the qualified form — so transitive
  collisions cannot exist.
- **Collision** and **conflict** are distinct failures with distinct
  machine identities; a collision fails with candidates, never an
  interactive pick.
