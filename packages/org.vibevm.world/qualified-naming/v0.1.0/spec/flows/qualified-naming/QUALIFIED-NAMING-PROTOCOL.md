# Qualified Naming Protocol {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** This file defines the laws that make a
package namespace scale: why a flat namespace fails, the mandatory
*group*, the identity tuple, why a rename is a new identity, why short
names may live only at the human boundary, and why a *collision* and a
*conflict* are two different failures. @impl/done

##written-for-ecosystem-designers It is written for **ecosystem
designers** — anyone minting the namespace for packages, plugins,
extensions, or artifacts. @impl/done

##sibling-document-pointers Reference grammar: [`ref-grammar.md`](ref-grammar.md);
the fork-by-fork rationale: [`naming-forks.md`](naming-forks.md). @impl/done

## The problem: flat names {#problem}

##A-FLAT-NAMESPACE-ADDRESSES-BY-A-SINGLE-BARE-NAME A flat namespace is one where an artifact is addressed by a single
bare `name`, unique across the whole registry. @spec/done

##it-fails-in-three-predictable-ways It reads beautifully on
day one (`install wal`) and fails in three predictable ways: @spec/done

- ##FAILURE-SQUATTING **Squatting.** The good short names are a finite commons. The first
  arrival takes `http`, `json`, `auth`; everyone after fights over
  `http2`, `json-fast`, `auth-real`. The namespace rewards land-grab
  speed, not quality. @spec/done
- ##FAILURE-TRUST-AMBIGUITY **Trust ambiguity.** In a flat namespace `logger` has no owner you
  can name. Is this the `logger` you audited last week, or a
  same-named replacement someone else published? Nothing in the
  coordinate answers, so trust cannot be delegated — it must be
  re-established per artifact. @spec/done
- ##FAILURE-TRANSITIVE-COLLISIONS **Transitive collisions.** Two dependencies deep in your graph both
  want the bare name `utils`, meaning different things. A flat resolver
  cannot satisfy both; the graph is unbuildable, and the failure
  surfaces far from either author. @spec/done

##EVERY-LAW-BELOW-IS-A-STRUCTURAL-FIX Every law below is a structural fix for one of these, not a
convenience. @impl/done

##A-GROUPED-NAMESPACE-FIXES-ALL-THREE A group-qualified namespace makes squatting local, trust
delegable, and transitive collisions impossible by construction. @impl/done

## Law 1 — every artifact carries a group {#group}

##IDENTITY-BEGINS-WITH-A-MANDATORY-GROUP Identity begins with a **mandatory** `group`: a dot-separated string
of lowercase segments (`com.example.shop`, `io.acme`, `org.vibevm`). @impl/done

##THE-GROUP-IS-THE-UNIT-OF-OWNERSHIP The group is the unit of ownership; `name` is unique *within* it, so
two owners may both ship a `wal` without colliding. @impl/done

- ##GROUP-UNIQUENESS-IS-DELEGATED **Uniqueness is delegated.** The registry guarantees groups are
  distinct; each group's owner guarantees names are distinct inside
  it. Global uniqueness falls out of two local guarantees — no central
  arbiter of every short name. @impl/done
- ##GROUP-REVERSE-FQDN-IS-RECOMMENDED-NOT-ENFORCED **Reverse-FQDN is recommended, not enforced.** Writing the group as
  a reversed domain (`org.example` for `example.org`) piggybacks on
  DNS's existing global uniqueness — a convention Sun introduced for
  Java packages in 1995 for exactly this reason. But whether a group
  *looks* like a reversed domain is **style**: a matter for humans and
  linters. The resolver checks only two things — that the group is
  well-formed grammar, and that it is unique. It never demands you own
  the domain. @impl/done
- ##GROUP-GRAMMAR-IS-THE-ONLY-HARD-RULE **Grammar is the only hard rule.** Segments are `[a-z0-9_-]+`, ASCII
  lowercase, dot-separated. That is enforced. Taste is not. @impl/done

##A-MANDATORY-GROUP-REMOVES-A-GREY-ZONE Making the group mandatory (rather than optional) removes a grey zone:
there is no "has a group" versus "no group" fork to reason about — every
artifact is qualified, always. @impl/done

## Law 2 — identity is a tuple {#identity}

##AN-ARTIFACTS-IDENTITY-IS-THE-TUPLE An artifact's identity is the tuple **`(group, name, version,
content-hash)`**. @impl/done

##two-consequences-carry-the-whole-system Two consequences carry the whole system: @impl/done

- ##GROUP-NAME-IS-GLOBALLY-UNIQUE **`(group, name)` is globally unique.** It names *the package* across
  all its versions. Any type tag (a `kind` such as `flow`, `plugin`,
  `lib`) is metadata — it may help placement or filtering, but it is
  **not** part of identity and never disambiguates two packages. @impl/done
- ##CONTENT-HASH-PINS-THE-BYTES **`content-hash` pins the bytes.** `(group, name, version)` names a
  release; the hash proves which bytes that release is. A mirror in a
  different registry serving the same bytes is the *same* identity; the
  registry URL is a fetch detail, not part of who the artifact is. @impl/done

##THE-COORDINATE-CARRIES-ITS-OWN-OWNERSHIP-AND-INTEGRITY Because identity is a tuple and not a string, the coordinate carries
its own ownership (`group`) and its own integrity (`content-hash`). @impl/done

##THAT-IS-WHAT-LETS-TRUST-BE-DELEGATED That is what lets trust be delegated: you trust `io.acme`, so you trust
every `io.acme/*` name, without re-auditing each one. @impl/done

## Law 3 — a rename is a new identity {#rename}

##A-CHANGED-GROUP-OR-NAME-IS-A-NEW-PACKAGE Change the `group` or the `name`, and you have a **new package** — not
a renamed one. @impl/done

##THIS-FOLLOWS-FROM-LAW-2-NOT-FROM-POLICY This is not a policy choice; it follows from Law 2: the
identity tuple changed, so the identity changed. @impl/done

- ##RENAME-VERSIONS-NEVER-TRANSFER **Versions never transfer.** `io.acme/logger` at `2.3.0` does not
  make `io.acme/log` start at `2.3.0`. The new name starts its own
  version line. History stays attached to the coordinate that earned
  it. @impl/done
- ##RENAME-NO-COORDINATE-IS-EVER-REUSED **No coordinate is ever reused for different content — ever.** Once
  `com.example.shop/cart@1.4.0` has meant one artifact, that exact
  `name@version` must never resolve to different bytes for anyone,
  forever. A consumer who locked `1.4.0` locked a specific meaning;
  silently repointing it is the one betrayal a package system must make
  impossible. Yank a bad release, publish a `1.4.1` — but never let the
  old coordinate mean something new. @impl/done

##the-alias-table-alternative-is-examined-in-naming-forks The rejected alternative — an alias table mapping the old name to the
new — is examined in [`naming-forks.md` §rename](naming-forks.md#rename). @impl/done

##AN-ALIAS-RE-INTRODUCES-THE-AMBIGUITY-THE-GROUP-REMOVED It loses because an alias re-introduces exactly the ambiguity the group
removed: now two coordinates name one artifact, and every reader must
know the mapping to trust what they read. @spec/done

## Law 4 — short names live only at the boundary {#short-names}

##A-SHORT-NAME-IS-THE-BARE-UNQUALIFIED-NAME-A-HUMAN-TYPES A **short name** is the bare, unqualified `name` a human types
(`install wal`). @impl/done

##A-SHORT-NAME-IS-LEGAL-IN-EXACTLY-ONE-PLACE It is a convenience, and it is legal in exactly one
place: **the human-typed CLI input boundary**, resolved **once** against
an index of `(group, name)` candidates. @impl/done

- ##SHORT-NAMES-ARE-NEVER-STORED **Never stored.** The moment a short name is resolved, the tool
  writes the *qualified* form (`org.vibevm.world/wal`) into the manifest and
  lockfile. Persisted state is qualified-only. @impl/done
- ##SHORT-NAMES-ARE-NEVER-RESOLVED-RECURSIVELY **Never resolved recursively.** Resolution happens for a human's
  argument and nothing else. The dependency graph is built entirely
  from qualified names, because every author published through the same
  boundary and stored the qualified form. @impl/done

##THE-BOUNDARY-RULE-MAKES-TRANSITIVE-COLLISIONS-IMPOSSIBLE This single rule is what makes **transitive collisions impossible by
construction**. @impl/done

##AMBIGUITY-CAN-ONLY-ARISE-WHERE-A-HUMAN-IS-PRESENT A short name can only be ambiguous at the one place a
human is present to disambiguate it; it can never be ambiguous three
levels deep in a graph, because no short name ever reaches that far. @impl/done

##the-cargo-npm-pattern-is-the-same-instinct The
cargo/npm pattern is the same instinct: `add serde` on the command
line, `serde = "1"` in the manifest. @spec/done

## Law 5 — collision and conflict are distinct failures {#collision}

##two-failures-look-similar-and-must-never-be-merged Two failures look similar and must never be merged: @impl/done

| Failure | Cause | Resolution |
|---|---|---|
| ##ROW-FAILURE-COLLISION **Collision** @impl/done | one short name matches two *different* packages (different groups) @impl/done | the human picks a group and records the qualified form @impl/done |
| ##ROW-FAILURE-CONFLICT **Conflict** @impl/done | version requirements cannot all be satisfied — contradictory constraints, a declared incompatibility, an unsatisfiable diamond @impl/done | the human relaxes a constraint or drops a dependency @impl/done |

- ##FAILURES-HAVE-DISTINCT-MACHINE-READABLE-IDENTITIES **Distinct machine-readable identities.** Each failure gets its own
  exit code and its own error type, so a script — or an agent — can
  branch on *which* failure occurred without parsing prose. The specific
  numbers are an implementation's choice; the law is only that the two
  differ and are stable. @impl/done
- ##NO-INTERACTIVE-PICK-ON-A-COLLISION **No interactive pick on a collision.** When a short name is
  ambiguous, the tool prints *all* candidates with their exact
  qualified forms and **fails**. It does not offer an arrow-key menu.
  The choice must be *recorded deliberately* — edited into the manifest
  by a human — not clicked once and forgotten. A clicked choice leaves
  no trace of why; a recorded qualified name is self-documenting. @impl/done

##error-shape-detail-pointer Error-shape detail — what a good collision message must contain — is in
[`ref-grammar.md` §errors](ref-grammar.md#errors). @impl/done

## Re-derive for your project {#re-derive}

##re-derive-lead Do not copy this document's example groups — copy the *task*, and let
the agent derive the namespace your ecosystem actually needs: @impl/done

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

- ##SUM-FLAT-NAMES-FAIL-THREE-WAYS Flat names fail three ways: squatting, trust ambiguity, transitive
  collisions. Groups fix all three structurally. @spec/done
- ##SUM-EVERY-ARTIFACT-CARRIES-A-MANDATORY-GROUP Every artifact carries a mandatory **group**; identity is
  `(group, name, version, content-hash)` and `(group, name)` is unique. @impl/done
- ##SUM-REVERSE-FQDN-IS-RECOMMENDED-STYLE Reverse-FQDN is recommended style, not enforced law — the resolver
  checks grammar and uniqueness, nothing about taste. @impl/done
- ##SUM-A-RENAME-IS-A-NEW-IDENTITY A rename is a **new identity**: versions never transfer, and no
  `name@version` coordinate is ever reused for different content. @impl/done
- ##SUM-SHORT-NAMES-LIVE-ONLY-AT-THE-BOUNDARY Short names live **only** at the human CLI boundary, resolved once;
  manifests and locks store the qualified form — so transitive
  collisions cannot exist. @impl/done
- ##SUM-COLLISION-AND-CONFLICT-ARE-DISTINCT **Collision** and **conflict** are distinct failures with distinct
  machine identities; a collision fails with candidates, never an
  interactive pick. @impl/done
