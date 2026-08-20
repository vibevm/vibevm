# Flow: Qualified Naming {#root}

<status stage="impl" state="done"/>

@fact:PROJECT-SHIPS-THE-QUALIFIED-NAMING-PRACTICE This project ships the **qualified-naming** practice for *ecosystem
designers* — anyone defining a namespace for packages, plugins,
extensions, or artifacts. @status:impl/done

@fact:IT-IS-A-DESIGN-DISCIPLINE-NOT-A-RUNTIME-RULE It is a design discipline, not a runtime
rule: it binds the moment an identifier is minted, not every edit that
uses one. @status:impl/done

## When this applies {#when}

@fact:READ-THE-PROTOCOL-BEFORE-THE-FIRST-NAME-IS-MINTED When you design any user-facing namespace — a package registry, a
plugin id scheme, an artifact coordinate, an extension marketplace —
read @spec://org.vibevm.world/qualified-naming/flows/qualified-naming/QUALIFIED-NAMING-PROTOCOL#root
**before the first name is minted**. @status:impl/done

@fact:retrofitting-a-group-is-a-migration Retrofitting a group onto a
shipped flat namespace is a migration; getting it right first is free. @status:spec/done

## The laws in one breath {#laws}

- @fact:LAW-EVERY-ARTIFACT-CARRIES-A-GROUP Every artifact carries a **group**; identity is the tuple
  `(group, name, version, content-hash)`, and `(group, name)` is
  globally unique. @status:impl/done
- @fact:LAW-A-RENAME-IS-A-NEW-IDENTITY A **rename is a new identity** — versions never transfer, and no
  `name@version` coordinate is ever reused for different content. @status:impl/done
- @fact:LAW-SHORT-NAMES-RESOLVE-ONLY-AT-THE-BOUNDARY **Short names resolve only at the human CLI boundary**, once,
  against an index; manifests and lockfiles store the qualified form. @status:impl/done
- @fact:LAW-COLLISION-AND-CONFLICT-ARE-DISTINCT A **collision** (one short name, two groups) and a **conflict** (a
  version contradiction) are distinct failures with distinct
  machine-readable identities. @status:impl/done

@fact:grammar-and-forms-pointer Grammar and forms: @spec://org.vibevm.world/qualified-naming/flows/qualified-naming/ref-grammar#root. @status:impl/done

@fact:fork-by-fork-rationale-pointer Fork-by-fork rationale: @spec://org.vibevm.world/qualified-naming/flows/qualified-naming/naming-forks#root. @status:impl/done

## Never {#never}

- @fact:NEVER-STORE-A-SHORT-NAME-IN-PERSISTED-STATE Never store a short (unqualified) name in a manifest, lockfile, or
  dependency graph — it is CLI sugar, nothing more. @status:impl/done
- @fact:NEVER-REUSE-A-COORDINATE-FOR-DIFFERENT-CONTENT Never reuse a `name@version` coordinate for different content: a
  coordinate that meant one artifact must never mean another. @status:impl/done
- @fact:NEVER-RESOLVE-A-NAMING-AMBIGUITY-INTERACTIVELY Never resolve a naming ambiguity interactively — fail with the
  candidate list and let a human record the qualified form. @status:impl/done
- @fact:NEVER-TREAT-A-CHANGE-OF-GROUP-OR-NAME-AS-A-RENAME Never treat a change of group or name as a rename — it is a new
  package, and versions do not carry over. @status:impl/done
