# Naming forks {#root}

**Scope of this document.** The design forks a namespace author faces,
each with its options, the choice this practice recommends, and the
reasoning — so a future designer need not re-litigate settled ground.
Four forks: flat versus grouped, enforce versus recommend, where short
names live, and rename as alias versus new identity. The laws these
forks produce are in [`QUALIFIED-NAMING-PROTOCOL.md`](QUALIFIED-NAMING-PROTOCOL.md).

## Fork 1 — flat versus grouped {#flat-vs-grouped}

The root fork: is an artifact addressed by a bare `name` unique across
the whole registry, or by a `group`-qualified coordinate?

Both models shipped at scale, so the trade is empirical, not
theoretical:

| Aspect | Flat (Cargo, npm-unscoped) | Grouped (Maven `groupId:artifactId`) |
|---|---|---|
| Short-name ergonomics | best — `serde`, `express` | worse — `org.example:widget` |
| Squatting pressure | high — names are a finite commons | low — squatting is local to a group |
| Trust delegation | per-artifact — the name names no owner | per-group — a group maps to an owner |
| Composition | transitive name clashes possible | clash-free — the group disambiguates |
| Onboarding cost | trivial | a group must be chosen up front |

**What each bought and paid.** Flat systems bought day-one ergonomics
and paid with a squatting arms race and a trust vacuum — `serde` reads
clean, but nothing in the coordinate tells you *who* stands behind it,
and two deep dependencies wanting the same bare name cannot both be
satisfied. Grouped systems paid verbosity up front — nobody enjoys
typing `org.apache.commons` — and bought it back twice: a group maps to
an owner (so trust is delegated wholesale, not re-earned per artifact),
and group-qualified coordinates compose without collision at any depth.

**Chosen: grouped.** The verbosity is real but bounded, and Fork 3
buys most of it back with a short name at the human boundary. The
squatting and trust properties are structural — they cannot be patched
onto a flat namespace after the fact. npm itself conceded the point by
bolting on `@scope/` once flat names ran out; starting grouped avoids
the migration.

## Fork 2 — enforce style versus recommend it {#enforce-vs-recommend}

Given a group grammar, how hard does the core enforce the reverse-FQDN
*convention*?

- **Option A — enforce.** Require the group to be a domain the
  publisher provably controls (DNS check, TXT record, the works).
- **Option B — recommend.** Enforce only the *grammar* (lowercase,
  dot-separated segments) and the *uniqueness* of the group; leave
  reverse-FQDN as a convention for humans and linters.

**Chosen: recommend.** Reverse-FQDN is worth recommending because it
piggybacks on DNS's existing global uniqueness — the trick Sun adopted
for Java packages in 1995 — so two independent authors almost never
collide by accident. But enforcing domain ownership buys little and
costs a lot: it couples publishing to DNS administration, breaks for
internal registries with no public domain, and still does not stop a
determined bad actor who *does* own a domain. The resolver's job is
narrow — check grammar, check uniqueness — and taste is left to
linters. Maven made the same call: it recommends reverse-FQDN groupIds
and enforces none of it. The grammar is the contract; the style is
guidance.

## Fork 3 — where do short names live {#short-names}

If a short name is a convenience, *how much* of the system may see it?

- **Everywhere.** Short names are first-class: manifests, lockfiles,
  and dependency edges may all carry them.
- **Nowhere.** Ban short names entirely; humans type fully-qualified
  coordinates always.
- **CLI boundary only.** Short names are legal solely as human-typed CLI
  input, resolved once, and never persisted.

**Chosen: boundary only.** "Everywhere" re-imports the flat namespace's
transitive-collision problem: a short name buried in a transitive
manifest is ambiguous at a point where no human is present to
disambiguate it. "Nowhere" is collision-safe but throws away the entire
ergonomic win of Fork 1's concession — nobody wants to type
`org.example.tools/widget` at a prompt.

"Boundary only" is the sweet spot, and its property is decisive:
short-name resolution happens *once*, for a human's argument, against an
index — then the qualified form is stored. Because persisted state is
qualified-only, the dependency graph is built entirely from qualified
names, and **a short name never recurses into the graph**. Transitive
collisions become impossible by construction rather than by a runtime
check. This is exactly the cargo/npm split — `add serde` on the command
line, `serde = "1"` in the manifest — generalised into a law.

## Fork 4 — rename: alias table versus new identity {#rename}

An author wants to rename a published package. What does the system do?

- **Alias table.** Keep a mapping `old → new`; resolve the old
  coordinate to the new artifact so existing consumers keep working.
- **New identity.** Treat the renamed package as a genuinely new
  package: it starts a fresh version line, and the old coordinate is
  frozen (yanked or left as-is), never repointed.

**Chosen: new identity.** The alias table is seductive — it seems to
spare consumers a migration — but it re-introduces precisely the
ambiguity groups were built to remove. Under an alias, two coordinates
now name one artifact, and every reader must consult the mapping to know
that `old/foo` and `new/foo` are the same bytes. Trust stops being
delegable: the coordinate no longer tells the whole truth about
ownership, because the *real* owner is one hop away through a table the
reader must know exists.

New identity keeps every coordinate honest. Identity is the tuple
`(group, name, version, content-hash)`; change the group or name and the
tuple changed, so the identity changed — this is a consequence of the
identity law, not an extra rule. The old `name@version` stays welded to
the bytes it always meant (no coordinate is *ever* reused for different
content), and the new name earns its own history from `0.1.0` forward. A
consumer migrates deliberately, by editing a qualified name they can
see — not silently, through a redirect they cannot.

## Summary {#summary}

- **Flat vs grouped → grouped.** Verbosity is bounded and bought back at
  the boundary; squatting-resistance and delegated trust are structural
  and cannot be retrofitted onto flat names.
- **Enforce vs recommend → recommend.** Enforce grammar and uniqueness;
  leave reverse-FQDN as style for humans and linters. Enforcing domain
  ownership couples publishing to DNS for little gain.
- **Where short names live → CLI boundary only.** Resolved once against
  an index, then stored qualified — which is what makes transitive
  collisions impossible.
- **Rename → new identity, not alias.** An alias re-introduces the
  ambiguity groups removed; a new identity keeps every coordinate
  honest and every version line attached to the bytes it named.
