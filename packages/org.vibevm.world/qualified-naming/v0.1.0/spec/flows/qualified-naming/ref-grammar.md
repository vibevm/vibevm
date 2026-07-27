# Reference grammar {#root}

<status stage="spec" state="done"/>

##scope-of-this-document **Scope of this document.** The concrete grammar for a package
reference, the table of legal forms and where each is legal, worked
examples with invented groups, the rule for what persisted state
stores, and the shape a collision error must take. @impl/done

##protocol-document-pointer The *why* behind
these choices lives in [`QUALIFIED-NAMING-PROTOCOL.md`](QUALIFIED-NAMING-PROTOCOL.md);
this file is the mechanics. @impl/done

## The grammar {#grammar}

##a-reference-has-up-to-four-parts A **reference** (the thing a manifest lists or a human types) has up to
four parts: @impl/done

- ##PART-AN-OPTIONAL-TYPE-TAG an optional type tag, @impl/done
- ##PART-AN-OPTIONAL-GROUP-QUALIFIER an optional group qualifier, @impl/done
- ##PART-A-MANDATORY-NAME a mandatory name, @impl/done
- ##PART-AN-OPTIONAL-VERSION-REQUIREMENT and an optional version requirement. @impl/done

```
ref      := [ kind ":" ] [ group "/" ] name [ "@" version ]

group    := segment ( "." segment )*
segment  := ( lower | digit | "_" | "-" )+
name     := ( lower | digit | "_" | "-" )+
kind     := ident                 ; type tag: "flow", "plugin", "lib", …
version  := <a version requirement in your scheme, e.g. semver>

lower    := "a" … "z"
digit    := "0" … "9"
```

##THREE-AXES-THREE-SEPARATORS **Three axes, three separators.** The type tag, the group, and the
version each attach with a *different* delimiter — `:` then `/` then
`@` — so the parser never has to guess which axis a token belongs to. @impl/done

##THE-DELIMITER-CHARACTERS-ARE-A-DESIGN-CHOICE The specific delimiter characters are a design choice; the load-bearing
rule is that the three axes stay lexically distinct. @impl/done

##PICK-THREE-THAT-DO-NOT-COLLIDE-AND-NEVER-OVERLOAD-ONE Pick three that do
not collide with your version syntax and never overload one. @impl/done

## The forms {#forms}

| Form | Example | Where legal |
|---|---|---|
| ##ROW-FORM-BARE-NAME **bare name** @impl/done | `wal` @impl/done | CLI input only — resolved once via the index @impl/done |
| ##ROW-FORM-KIND-AND-NAME **kind + name** @impl/done | `flow:wal` @impl/done | CLI input only — kind is validated after resolution @impl/done |
| ##ROW-FORM-GROUP-QUALIFIED **group-qualified** @impl/done | `org.vibevm.world/wal` @impl/done | **anywhere** — CLI, manifests, lockfiles @impl/done |
| ##ROW-FORM-KIND-AND-GROUP-QUALIFIED **kind + group-qualified** @impl/done | `flow:org.vibevm.world/wal` @impl/done | anywhere — the fully explicit form @impl/done |
| ##ROW-FORM-VERSIONED **versioned** @impl/done | `org.vibevm.world/wal@0.6.0` @impl/done | anywhere a specific release is meant @impl/done |

##two-rules-govern-the-table Two rules govern the table: @impl/done

- ##RULE-MANIFESTS-ACCEPT-QUALIFIED-FORMS-ONLY **Manifests and lockfiles accept the qualified forms only.** A bare
  or kind-only name is never written to persisted state (see
  [§storage](#storage)). @impl/done
- ##RULE-THE-CLI-ACCEPTS-ALL-FORMS **The CLI accepts all forms.** It is the one place a human is present,
  so it is the one place a short name may be resolved. @impl/done

##THE-KIND-TAG-VALIDATES-IT-NEVER-DISAMBIGUATES **The kind tag validates, it never disambiguates.** @impl/done

##THE-RESOLVER-CHECKS-THE-TYPE-AND-ERRORS-ON-A-MISMATCH If a reference
carries `kind:`, the resolver checks that the resolved package's type
matches, and errors on a mismatch. @impl/done

##A-REAL-AMBIGUITY-IS-ALWAYS-A-GROUP-COLLISION It cannot pick between two packages,
because `(group, name)` is already unique — a real ambiguity is always
a *group* collision, resolved by qualifying the group, never by adding a
kind. @impl/done

## Worked examples {#examples}

##invented-groups-lead Invented groups, to keep the mechanics product-neutral: @impl/done

```
cart                          # bare — CLI sugar; resolves if exactly
                              #   one group owns a package named "cart"

com.example.shop/cart         # qualified — the form a manifest stores

plugin:com.example.shop/cart  # fully explicit; "plugin" is checked
                              #   against the manifest after resolution

com.example.shop/cart@1.4.0   # a specific release

io.acme/logger@^2.1           # a version *requirement*, not a pin;
                              #   the lockfile records the pin it chose
```

##manifest-fragment-lead A manifest fragment, after the tool resolved a human's `add cart`: @impl/done

```toml
[requires]
"com.example.shop/cart" = "1.4.0"
"io.acme/logger"        = "^2.1"
```

##THE-HUMAN-TYPED-THE-SHORT-NAMES-THE-TOOL-STORED-THE-QUALIFIED-ONES Note what is *not* here: no bare `cart`, no `logger` — the human typed
those, the tool stored the qualified forms. @impl/done

## What gets stored {#storage}

##the-single-storage-rule-stated-once The single storage rule, stated once so it cannot drift: @impl/done

> ##PERSISTED-STATE-IS-QUALIFIED-ONLY **Persisted state is qualified-only.** Every reference written to a
> manifest, a lockfile, or a dependency edge carries its group. Short
> names exist solely as human CLI input and are rewritten to the
> qualified form the instant they are resolved. @impl/done

##consequences-worth-making-explicit Consequences worth making explicit: @impl/done

- ##CONSEQUENCE-A-LOCKFILE-ENTRY-CARRIES-THE-FULL-TUPLE A **lockfile** entry carries the full tuple: `group`, `name`, the
  resolved `version`, and the `content-hash` that pins the bytes. Two
  registries serving the same hash under the same `(group, name,
  version)` are the same locked entry — the registry is a fetch detail,
  not part of the lock's identity. @impl/done
- ##CONSEQUENCE-THE-GRAPH-IS-BUILT-FROM-QUALIFIED-NAMES The **dependency graph** is built entirely from qualified names, so a
  short name never recurses into it. This is the mechanism behind the
  protocol's "no transitive collisions" guarantee — it is enforced *by
  the storage rule*, not by a separate check. @impl/done
- ##CONSEQUENCE-A-PUBLISHED-PACKAGES-REQUIRES-IS-QUALIFIED A **published package's own `[requires]`** is qualified, because its
  author published through the same boundary. You never inherit another
  author's short name. @impl/done

## Error shapes {#errors}

##A-MACHINE-FACING-FAILURE-MUST-BE-ACTIONABLE-WITHOUT-PROSE A machine-facing failure is only useful if a script or an agent can act
on it without reading prose. @impl/done

##two-shapes-matter Two shapes matter. @impl/done

##a-collision-must **A collision** (one short name, several owners) must: @impl/done

1. ##COLLISION-EXIT-WITH-THE-COLLISION-CODE exit with the collision code — distinct from the conflict code, so a
   caller branches on the number alone; @impl/done
2. ##COLLISION-LIST-EVERY-CANDIDATE list **every** candidate, each with its exact qualified form; and @impl/done
3. ##COLLISION-TELL-THE-HUMAN-WHAT-TO-TYPE-NEXT tell the human precisely what to type or record next. @impl/done

```
"cart" is ambiguous — 2 packages match:
  1. com.example.shop/cart   (registry shop-public)
  2. io.acme/cart            (registry acme-internal)
Re-run with the qualified form, e.g.  install com.example.shop/cart
```

##THE-CANDIDATE-LIST-IS-THE-WHOLE-POINT The candidate list is the whole point: the human copies one line and
records it. @impl/done

##THERE-IS-NO-INTERACTIVE-MENU There is **no interactive menu** — a picked choice leaves no
record of *why* that group; a pasted qualified name is self-explaining. @impl/done

##A-CONFLICT-IS-A-DIFFERENT-FAILURE-WITH-A-DIFFERENT-CODE **A conflict** (unsatisfiable versions) is a *different* failure with a
*different* code. @impl/done

##a-conflict-names-the-incompatible-constraints It names the incompatible constraints and the packages
that imposed them, so the human can relax one: @impl/done

```
version conflict on io.acme/logger:
  com.example.shop/cart  requires  ^2.1
  org.example.tools/audit requires  <2.0
no version satisfies both.
```

##a-caller-that-cannot-tell-them-apart-will-retry-wrongly A caller that cannot tell these two apart will retry a collision as if
it were a conflict, or vice versa. @spec/done

##DISTINCT-STABLE-CODES-KEEP-AUTOMATION-CORRECT Distinct, stable codes are what keep
automation correct. @impl/done

## Summary {#summary}

- ##SUM-A-REFERENCE-IS-KIND-GROUP-NAME-VERSION A reference is `[kind:][group/]name[@version]` — three axes, three
  distinct separators; the delimiter characters are a choice, the
  distinctness is the law. @impl/done
- ##SUM-BARE-AND-KIND-ONLY-FORMS-ARE-CLI-SUGAR Bare and kind-only forms are CLI-input sugar; qualified forms are
  legal everywhere. @impl/done
- ##SUM-THE-KIND-TAG-VALIDATES-THE-RESOLVED-TYPE The kind tag validates the resolved type; it never disambiguates. @impl/done
- ##SUM-PERSISTED-STATE-IS-QUALIFIED-ONLY Persisted state is **qualified-only** — that storage rule is what
  makes transitive collisions impossible. @impl/done
- ##SUM-A-COLLISION-LISTS-CANDIDATES-A-CONFLICT-IS-SEPARATE A collision lists every candidate with copy-ready qualified forms and
  fails; a conflict is a separate failure with a separate, stable
  machine code. @impl/done
