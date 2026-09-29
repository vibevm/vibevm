# `flow:qualified-naming` — a namespace that scales {#root}

<status stage="doc" state="done" audience="user"/>

@fact:package-installs-the-qualified-naming-discipline A `flow` package that installs the **qualified naming** discipline for
package ecosystems: @status:impl/done

- @fact:LAW-A-MANDATORY-GROUP-ON-EVERY-ARTIFACT a mandatory *group* on every artifact, @status:impl/done
- @fact:LAW-IDENTITY-IS-THE-TUPLE identity as
  the tuple `(group, name, version, content-hash)`, @status:impl/done
- @fact:LAW-SHORT-NAMES-ONLY-AT-THE-CLI-BOUNDARY short names allowed
  only at the human CLI boundary, @status:impl/done
- @fact:LAW-COLLISION-KEPT-DISTINCT-FROM-CONFLICT and *collision* kept strictly distinct
  from *conflict*. @status:impl/done

@fact:AUDIENCE-IS-ECOSYSTEM-DESIGNERS **Audience: ecosystem designers** — anyone defining a namespace for
packages, plugins, extensions, or artifacts. @status:impl/done

@fact:IT-IS-A-DESIGN-TIME-DISCIPLINE-READ-ONCE This is a design-time
discipline, read once while shaping identifiers, not a per-session
rule. @status:impl/done

@fact:get-the-namespace-right-before-the-first-name-is-minted Get the namespace right before the first name is minted;
retrofitting a group onto a shipped flat registry is costly. @status:spec/done

@fact:a-flat-namespace-fails-three-ways A flat namespace reads beautifully on day one and fails three ways: @status:spec/done

- @fact:FAILURE-SQUATTING squatting turns good short names into a land-grab, @status:spec/done
- @fact:FAILURE-A-BARE-NAME-NAMES-NO-OWNER a bare name names no
  owner so trust cannot be delegated, @status:spec/done
- @fact:FAILURE-TRANSITIVE-NAME-COLLISION and two dependencies deep in a
  graph can want one name meaning different things. @status:spec/done

@fact:groups-fix-all-three-structurally Groups fix all three
structurally. @status:spec/done

@fact:THIS-PACKAGE-IS-THAT-FIX-AS-A-STANDING-CONTRACT This package is that fix, made into a standing contract. @status:impl/done

@fact:package-contents-lead This package ships three pieces of content plus a boot snippet: @status:impl/done

- @fact:CONTENT-THE-PROTOCOL `spec/flows/qualified-naming/QUALIFIED-NAMING-PROTOCOL.xml` — the full
  protocol: why flat names fail, the mandatory group, the identity
  tuple, rename-is-new-identity, short-names-at-the-boundary-only,
  collision versus conflict, and a re-derive prompt for adapting the
  practice to a concrete ecosystem. @status:impl/done
- @fact:CONTENT-THE-REF-GRAMMAR `spec/flows/qualified-naming/ref-grammar.xml` — the reference grammar
  in EBNF-ish form, the forms table with where-legal per form, worked
  examples with invented groups, the qualified-only storage rule, and
  the shape a collision error must take. @status:impl/done
- @fact:CONTENT-THE-NAMING-FORKS `spec/flows/qualified-naming/naming-forks.xml` — the design lore
  condensed: flat vs grouped (the Cargo-vs-Maven precedent), enforce vs
  recommend, where short names live, and rename as alias vs new
  identity — each fork resolved, with reasons. @status:impl/done
- @fact:CONTENT-THE-BOOT-SNIPPET `spec/boot/67-flow-qualified-naming.xml` — boot snippet loaded at
  session start: when the practice applies, the laws in one breath, and
  the never-do list. @status:impl/done

## Install {#install}

```bash
vibe install flow:qualified-naming
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:qualified-naming
```

@fact:UNINSTALL-REMOVES-EVERY-FILE-THE-PACKAGE-WROTE Uninstalling removes every file the package wrote, including the boot
snippet. @status:impl/done

@fact:USER-OWNED-FILES-ARE-NEVER-TOUCHED User-owned files are never touched. @status:impl/done

## Composition {#composition}

- @fact:COMPOSES-TOOL-DESIGN-LESSONS `flow:tool-design-lessons` — the sibling practice for the authors who
  *publish into* a namespace; this package designs the namespace they
  publish into. Together they cover both sides of the registry boundary. @status:spec/done
- @fact:COMPOSES-DECISION-RECORDS `flow:decision-records` — each fork in `naming-forks.xml` is a decision
  a future maintainer will re-open; resolve it for *your* ecosystem and
  record it with a why and a revisit trigger, so the namespace's shape
  is not re-litigated. @status:spec/done
- @fact:COMPOSES-ADDRESSABLE-SPECS `flow:addressable-specs` — the same "one authoritative address per
  fact" instinct, applied to artifact identity: a `spec://` anchor names
  one fact unambiguously, exactly as `(group, name)` names one package. @status:spec/done

## Philosophical background {#background}

@fact:discipline-crystallized-from-the-origin-projects-law The discipline is crystallized from the origin project's qualified-naming
law and its Cargo-vs-Maven precedent study — flat namespaces (Cargo,
npm-unscoped) hit squatting and trust problems that group-qualified
systems (Maven) structurally avoid, at the cost of verbosity, paid back
by delegated trust and collision-free composition. @status:spec/done

@fact:reverse-fqdn-descends-from-suns-java-package-naming The reverse-FQDN
convention itself descends from Sun's 1995 Java package naming, which
borrowed DNS's global uniqueness by writing domains backwards. @status:spec/done

@fact:collections-spirit-is-the-redbook The collection's spirit is the book *AI-native development*, which ships
in Russian inside `flow:redbook` at `spec/book/ru/`. @status:spec/done

@fact:A-NAME-IS-THE-CHEAPEST-INTERFACE-IN-A-SYSTEM Short version: a
name is the cheapest interface in a system, and the only one every other
component depends on — so it is the one worth getting right first. @status:spec/done

## License {#license}

@fact:license-line UPL-1.0. See [`LICENSE.md`](LICENSE.md). @status:impl/done

