# `flow:qualified-naming` — a namespace that scales {#root}

A `flow` package that installs the **qualified naming** discipline for
package ecosystems: a mandatory *group* on every artifact, identity as
the tuple `(group, name, version, content-hash)`, short names allowed
only at the human CLI boundary, and *collision* kept strictly distinct
from *conflict*.

**Audience: ecosystem designers** — anyone defining a namespace for
packages, plugins, extensions, or artifacts. This is a design-time
discipline, read once while shaping identifiers, not a per-session
rule. Get the namespace right before the first name is minted;
retrofitting a group onto a shipped flat registry is costly.

A flat namespace reads beautifully on day one and fails three ways:
squatting turns good short names into a land-grab, a bare name names no
owner so trust cannot be delegated, and two dependencies deep in a
graph can want one name meaning different things. Groups fix all three
structurally. This package is that fix, made into a standing contract.

This package ships three pieces of content plus a boot snippet:

- `spec/flows/qualified-naming/QUALIFIED-NAMING-PROTOCOL.md` — the full
  protocol: why flat names fail, the mandatory group, the identity
  tuple, rename-is-new-identity, short-names-at-the-boundary-only,
  collision versus conflict, and a re-derive prompt for adapting the
  practice to a concrete ecosystem.
- `spec/flows/qualified-naming/ref-grammar.md` — the reference grammar
  in EBNF-ish form, the forms table with where-legal per form, worked
  examples with invented groups, the qualified-only storage rule, and
  the shape a collision error must take.
- `spec/flows/qualified-naming/naming-forks.md` — the design lore
  condensed: flat vs grouped (the Cargo-vs-Maven precedent), enforce vs
  recommend, where short names live, and rename as alias vs new
  identity — each fork resolved, with reasons.
- `spec/boot/67-flow-qualified-naming.md` — boot snippet loaded at
  session start: when the practice applies, the laws in one breath, and
  the never-do list.

## Install {#install}

```bash
vibe install flow:qualified-naming
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:qualified-naming
```

Uninstalling removes every file the package wrote, including the boot
snippet. User-owned files are never touched.

## Composition {#composition}

- `flow:tool-design-lessons` — the sibling practice for the authors who
  *publish into* a namespace; this package designs the namespace they
  publish into. Together they cover both sides of the registry boundary.
- `flow:decision-records` — each fork in `naming-forks.md` is a decision
  a future maintainer will re-open; resolve it for *your* ecosystem and
  record it with a why and a revisit trigger, so the namespace's shape
  is not re-litigated.
- `flow:addressable-specs` — the same "one authoritative address per
  fact" instinct, applied to artifact identity: a `spec://` anchor names
  one fact unambiguously, exactly as `(group, name)` names one package.

## Philosophical background {#background}

The discipline is crystallized from the origin project's qualified-naming
law and its Cargo-vs-Maven precedent study — flat namespaces (Cargo,
npm-unscoped) hit squatting and trust problems that group-qualified
systems (Maven) structurally avoid, at the cost of verbosity, paid back
by delegated trust and collision-free composition. The reverse-FQDN
convention itself descends from Sun's 1995 Java package naming, which
borrowed DNS's global uniqueness by writing domains backwards.

The collection's spirit is the book *AI-native development*, which ships
in Russian inside `flow:redbook` at `spec/book/ru/`. Short version: a
name is the cheapest interface in a system, and the only one every other
component depends on — so it is the one worth getting right first.

## License {#license}

UPL-1.0. See [`LICENSE.md`](LICENSE.md).
