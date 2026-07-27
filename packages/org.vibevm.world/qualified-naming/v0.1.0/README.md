# `flow:qualified-naming` — a namespace that scales {#root}

<status stage="doc" state="done" audience="user"/>

##package-installs-the-qualified-naming-discipline A `flow` package that installs the **qualified naming** discipline for
package ecosystems: @impl/done

- ##LAW-A-MANDATORY-GROUP-ON-EVERY-ARTIFACT a mandatory *group* on every artifact, @impl/done
- ##LAW-IDENTITY-IS-THE-TUPLE identity as
  the tuple `(group, name, version, content-hash)`, @impl/done
- ##LAW-SHORT-NAMES-ONLY-AT-THE-CLI-BOUNDARY short names allowed
  only at the human CLI boundary, @impl/done
- ##LAW-COLLISION-KEPT-DISTINCT-FROM-CONFLICT and *collision* kept strictly distinct
  from *conflict*. @impl/done

##AUDIENCE-IS-ECOSYSTEM-DESIGNERS **Audience: ecosystem designers** — anyone defining a namespace for
packages, plugins, extensions, or artifacts. @impl/done

##IT-IS-A-DESIGN-TIME-DISCIPLINE-READ-ONCE This is a design-time
discipline, read once while shaping identifiers, not a per-session
rule. @impl/done

##get-the-namespace-right-before-the-first-name-is-minted Get the namespace right before the first name is minted;
retrofitting a group onto a shipped flat registry is costly. @spec/done

##a-flat-namespace-fails-three-ways A flat namespace reads beautifully on day one and fails three ways: @spec/done

- ##FAILURE-SQUATTING squatting turns good short names into a land-grab, @spec/done
- ##FAILURE-A-BARE-NAME-NAMES-NO-OWNER a bare name names no
  owner so trust cannot be delegated, @spec/done
- ##FAILURE-TRANSITIVE-NAME-COLLISION and two dependencies deep in a
  graph can want one name meaning different things. @spec/done

##groups-fix-all-three-structurally Groups fix all three
structurally. @spec/done

##THIS-PACKAGE-IS-THAT-FIX-AS-A-STANDING-CONTRACT This package is that fix, made into a standing contract. @impl/done

##package-contents-lead This package ships three pieces of content plus a boot snippet: @impl/done

- ##CONTENT-THE-PROTOCOL `spec/flows/qualified-naming/QUALIFIED-NAMING-PROTOCOL.md` — the full
  protocol: why flat names fail, the mandatory group, the identity
  tuple, rename-is-new-identity, short-names-at-the-boundary-only,
  collision versus conflict, and a re-derive prompt for adapting the
  practice to a concrete ecosystem. @impl/done
- ##CONTENT-THE-REF-GRAMMAR `spec/flows/qualified-naming/ref-grammar.md` — the reference grammar
  in EBNF-ish form, the forms table with where-legal per form, worked
  examples with invented groups, the qualified-only storage rule, and
  the shape a collision error must take. @impl/done
- ##CONTENT-THE-NAMING-FORKS `spec/flows/qualified-naming/naming-forks.md` — the design lore
  condensed: flat vs grouped (the Cargo-vs-Maven precedent), enforce vs
  recommend, where short names live, and rename as alias vs new
  identity — each fork resolved, with reasons. @impl/done
- ##CONTENT-THE-BOOT-SNIPPET `spec/boot/67-flow-qualified-naming.md` — boot snippet loaded at
  session start: when the practice applies, the laws in one breath, and
  the never-do list. @impl/done

## Install {#install}

```bash
vibe install flow:qualified-naming
```

## Uninstall {#uninstall}

```bash
vibe uninstall flow:qualified-naming
```

##UNINSTALL-REMOVES-EVERY-FILE-THE-PACKAGE-WROTE Uninstalling removes every file the package wrote, including the boot
snippet. @impl/done

##USER-OWNED-FILES-ARE-NEVER-TOUCHED User-owned files are never touched. @impl/done

## Composition {#composition}

- ##COMPOSES-TOOL-DESIGN-LESSONS `flow:tool-design-lessons` — the sibling practice for the authors who
  *publish into* a namespace; this package designs the namespace they
  publish into. Together they cover both sides of the registry boundary. @spec/done
- ##COMPOSES-DECISION-RECORDS `flow:decision-records` — each fork in `naming-forks.md` is a decision
  a future maintainer will re-open; resolve it for *your* ecosystem and
  record it with a why and a revisit trigger, so the namespace's shape
  is not re-litigated. @spec/done
- ##COMPOSES-ADDRESSABLE-SPECS `flow:addressable-specs` — the same "one authoritative address per
  fact" instinct, applied to artifact identity: a `spec://` anchor names
  one fact unambiguously, exactly as `(group, name)` names one package. @spec/done

## Philosophical background {#background}

##discipline-crystallized-from-the-origin-projects-law The discipline is crystallized from the origin project's qualified-naming
law and its Cargo-vs-Maven precedent study — flat namespaces (Cargo,
npm-unscoped) hit squatting and trust problems that group-qualified
systems (Maven) structurally avoid, at the cost of verbosity, paid back
by delegated trust and collision-free composition. @spec/done

##reverse-fqdn-descends-from-suns-java-package-naming The reverse-FQDN
convention itself descends from Sun's 1995 Java package naming, which
borrowed DNS's global uniqueness by writing domains backwards. @spec/done

##collections-spirit-is-the-redbook The collection's spirit is the book *AI-native development*, which ships
in Russian inside `flow:redbook` at `spec/book/ru/`. @spec/done

##A-NAME-IS-THE-CHEAPEST-INTERFACE-IN-A-SYSTEM Short version: a
name is the cheapest interface in a system, and the only one every other
component depends on — so it is the one worth getting right first. @spec/done

## License {#license}

##license-line UPL-1.0. See [`LICENSE.md`](LICENSE.md). @impl/done
