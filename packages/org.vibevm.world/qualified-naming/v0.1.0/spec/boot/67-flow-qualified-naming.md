# Flow: Qualified Naming {#root}

<status stage="impl" state="done"/>

##PROJECT-SHIPS-THE-QUALIFIED-NAMING-PRACTICE This project ships the **qualified-naming** practice for *ecosystem
designers* — anyone defining a namespace for packages, plugins,
extensions, or artifacts. @impl/done

##IT-IS-A-DESIGN-DISCIPLINE-NOT-A-RUNTIME-RULE It is a design discipline, not a runtime
rule: read it once while shaping identifiers, not on every session. @impl/done

## When this applies {#when}

##READ-THE-PROTOCOL-BEFORE-THE-FIRST-NAME-IS-MINTED When you design any user-facing namespace — a package registry, a
plugin id scheme, an artifact coordinate, an extension marketplace —
read [`QUALIFIED-NAMING-PROTOCOL.md`](../flows/qualified-naming/QUALIFIED-NAMING-PROTOCOL.md)
**before the first name is minted**. @impl/done

##retrofitting-a-group-is-a-migration Retrofitting a group onto a
shipped flat namespace is a migration; getting it right first is free. @spec/done

## The laws in one breath {#laws}

- ##LAW-EVERY-ARTIFACT-CARRIES-A-GROUP Every artifact carries a **group**; identity is the tuple
  `(group, name, version, content-hash)`, and `(group, name)` is
  globally unique. @impl/done
- ##LAW-A-RENAME-IS-A-NEW-IDENTITY A **rename is a new identity** — versions never transfer, and no
  `name@version` coordinate is ever reused for different content. @impl/done
- ##LAW-SHORT-NAMES-RESOLVE-ONLY-AT-THE-BOUNDARY **Short names resolve only at the human CLI boundary**, once,
  against an index; manifests and lockfiles store the qualified form. @impl/done
- ##LAW-COLLISION-AND-CONFLICT-ARE-DISTINCT A **collision** (one short name, two groups) and a **conflict** (a
  version contradiction) are distinct failures with distinct
  machine-readable identities. @impl/done

##grammar-and-forms-pointer Grammar and forms: [`ref-grammar.md`](../flows/qualified-naming/ref-grammar.md). @impl/done

##fork-by-fork-rationale-pointer Fork-by-fork rationale: [`naming-forks.md`](../flows/qualified-naming/naming-forks.md). @impl/done

## Never {#never}

- ##NEVER-STORE-A-SHORT-NAME-IN-PERSISTED-STATE Never store a short (unqualified) name in a manifest, lockfile, or
  dependency graph — it is CLI sugar, nothing more. @impl/done
- ##NEVER-REUSE-A-COORDINATE-FOR-DIFFERENT-CONTENT Never reuse a `name@version` coordinate for different content: a
  coordinate that meant one artifact must never mean another. @impl/done
- ##NEVER-RESOLVE-A-NAMING-AMBIGUITY-INTERACTIVELY Never resolve a naming ambiguity interactively — fail with the
  candidate list and let a human record the qualified form. @impl/done
- ##NEVER-TREAT-A-CHANGE-OF-GROUP-OR-NAME-AS-A-RENAME Never treat a change of group or name as a rename — it is a new
  package, and versions do not carry over. @impl/done
