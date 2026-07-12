# Reference grammar {#root}

**Scope of this document.** The concrete grammar for a package
reference, the table of legal forms and where each is legal, worked
examples with invented groups, the rule for what persisted state
stores, and the shape a collision error must take. The *why* behind
these choices lives in [`QUALIFIED-NAMING-PROTOCOL.md`](QUALIFIED-NAMING-PROTOCOL.md);
this file is the mechanics.

## The grammar {#grammar}

A **reference** (the thing a manifest lists or a human types) has up to
four parts: an optional type tag, an optional group qualifier, a
mandatory name, and an optional version requirement.

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

**Three axes, three separators.** The type tag, the group, and the
version each attach with a *different* delimiter — `:` then `/` then
`@` — so the parser never has to guess which axis a token belongs to.
The specific delimiter characters are a design choice; the load-bearing
rule is that the three axes stay lexically distinct. Pick three that do
not collide with your version syntax and never overload one.

## The forms {#forms}

| Form | Example | Where legal |
|---|---|---|
| **bare name** | `wal` | CLI input only — resolved once via the index |
| **kind + name** | `flow:wal` | CLI input only — kind is validated after resolution |
| **group-qualified** | `org.vibevm.world/wal` | **anywhere** — CLI, manifests, lockfiles |
| **kind + group-qualified** | `flow:org.vibevm.world/wal` | anywhere — the fully explicit form |
| **versioned** | `org.vibevm.world/wal@0.6.0` | anywhere a specific release is meant |

Two rules govern the table:

- **Manifests and lockfiles accept the qualified forms only.** A bare
  or kind-only name is never written to persisted state (see
  [§storage](#storage)).
- **The CLI accepts all forms.** It is the one place a human is present,
  so it is the one place a short name may be resolved.

**The kind tag validates, it never disambiguates.** If a reference
carries `kind:`, the resolver checks that the resolved package's type
matches, and errors on a mismatch. It cannot pick between two packages,
because `(group, name)` is already unique — a real ambiguity is always
a *group* collision, resolved by qualifying the group, never by adding a
kind.

## Worked examples {#examples}

Invented groups, to keep the mechanics product-neutral:

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

A manifest fragment, after the tool resolved a human's `add cart`:

```toml
[requires]
"com.example.shop/cart" = "1.4.0"
"io.acme/logger"        = "^2.1"
```

Note what is *not* here: no bare `cart`, no `logger` — the human typed
those, the tool stored the qualified forms.

## What gets stored {#storage}

The single storage rule, stated once so it cannot drift:

> **Persisted state is qualified-only.** Every reference written to a
> manifest, a lockfile, or a dependency edge carries its group. Short
> names exist solely as human CLI input and are rewritten to the
> qualified form the instant they are resolved.

Consequences worth making explicit:

- A **lockfile** entry carries the full tuple: `group`, `name`, the
  resolved `version`, and the `content-hash` that pins the bytes. Two
  registries serving the same hash under the same `(group, name,
  version)` are the same locked entry — the registry is a fetch detail,
  not part of the lock's identity.
- The **dependency graph** is built entirely from qualified names, so a
  short name never recurses into it. This is the mechanism behind the
  protocol's "no transitive collisions" guarantee — it is enforced *by
  the storage rule*, not by a separate check.
- A **published package's own `[requires]`** is qualified, because its
  author published through the same boundary. You never inherit another
  author's short name.

## Error shapes {#errors}

A machine-facing failure is only useful if a script or an agent can act
on it without reading prose. Two shapes matter.

**A collision** (one short name, several owners) must:

1. exit with the collision code — distinct from the conflict code, so a
   caller branches on the number alone;
2. list **every** candidate, each with its exact qualified form; and
3. tell the human precisely what to type or record next.

```
"cart" is ambiguous — 2 packages match:
  1. com.example.shop/cart   (registry shop-public)
  2. io.acme/cart            (registry acme-internal)
Re-run with the qualified form, e.g.  install com.example.shop/cart
```

The candidate list is the whole point: the human copies one line and
records it. There is **no interactive menu** — a picked choice leaves no
record of *why* that group; a pasted qualified name is self-explaining.

**A conflict** (unsatisfiable versions) is a *different* failure with a
*different* code. It names the incompatible constraints and the packages
that imposed them, so the human can relax one:

```
version conflict on io.acme/logger:
  com.example.shop/cart  requires  ^2.1
  org.example.tools/audit requires  <2.0
no version satisfies both.
```

A caller that cannot tell these two apart will retry a collision as if
it were a conflict, or vice versa. Distinct, stable codes are what keep
automation correct.

## Summary {#summary}

- A reference is `[kind:][group/]name[@version]` — three axes, three
  distinct separators; the delimiter characters are a choice, the
  distinctness is the law.
- Bare and kind-only forms are CLI-input sugar; qualified forms are
  legal everywhere.
- The kind tag validates the resolved type; it never disambiguates.
- Persisted state is **qualified-only** — that storage rule is what
  makes transitive collisions impossible.
- A collision lists every candidate with copy-ready qualified forms and
  fails; a conflict is a separate failure with a separate, stable
  machine code.
