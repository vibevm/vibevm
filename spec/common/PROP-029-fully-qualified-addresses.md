# PROP-029 — Fully-qualified addresses and mechanical refactoring {#root}

**Status:** accepted 2026-07-12 (owner-ratified). **Builds on:**
[`spec://vibevm/modules/vibe-registry/PROP-008#group`](../modules/vibe-registry/PROP-008-qualified-naming.md#group)
(the `group` field) and the `addressable-specs` protocol's reverse-DNS
module rule.

## 1. Every address is fully qualified {#decision}

A package address MUST carry its full coordinate — `group` **and** `name` —
in **every** occurrence across the project: manifests, lockfiles, `spec://`
citations, `scope!` markers, `DEVIATES` lines, code comments, and docs. Short
or bare names survive only as a one-time human CLI input, resolved to the
qualified form at the boundary (PROP-008 §2.6); nothing on disk stores a bare
name.

Two textual carriers, one identity:

| Carrier | Form | Example |
|---|---|---|
| pkgref (manifests, lockfiles, prose) | `[<kind>:]<group>/<name>` | `stack:org.vibevm.ai-native/rust-ai-native-lang` |
| `spec://` authority (the `<module>` segment) | `<group>.<name>` | `spec://org.vibevm.ai-native.rust-ai-native-lang/GUIDE#anchor` |

The separator differs by carrier only because `/` already delimits the URI
path — the dotted form is exactly the FQDN repository name (PROP-008 §2.5). A
consumer parsing either recovers the same `(group, name)`.

## 2. Why — structure-independence enables mechanical refactoring {#rationale}

A fully-qualified address does not depend on where a package sits in the tree,
which group currently owns it, or how its spec is filed. It is a stable global
symbol. That is the precondition for **deterministic, non-LLM refactoring**:
because every reference to a unit is the same self-contained string, a rename
is a pure textual substitution an algorithm performs exactly — change a
package name, a group, or an anchor, then rewrite every occurrence, and the
inverse. Short or relative addresses need a resolver carrying ambient context
("which module am I in?") — a judgment call, LLM-shaped and fallible. Full
addresses turn address refactoring from judgment into a table lookup.

## 3. The mechanical-refactoring foundation {#mechanical}

Address refactors — rename a package, move a group, rename a cited anchor —
MUST be expressible as deterministic substitutions over fully-qualified
strings, verified by grep-zero of the old coordinate. The reference
implementation today is a scripted `sed` transform with grep verification and
a specmap re-mint; the target is a first-class **rename engine** (a future
FEAT) that takes `(old-coordinate → new-coordinate)` and rewrites every
manifest, lockfile, spec URI, and marker, then regenerates the specmap and the
derived lockfiles/vibedeps. The engine is only possible while §1 holds — the
day one bare name lands on disk, a rename needs a resolver again.

## 4. Scope and exceptions {#scope}

- The **host vibevm project's own** specs keep the project authority
  `spec://vibevm/…` — the root project is not a package with a group; §1 binds
  packages.
- **Test fixtures and grammar examples** (`spec://demo/…`,
  `spec://com.example.shop/…`, and the like) are illustrative, not real
  packages; they are out of scope and stay as written.
- Changing a package's `group` is a **new package**, not a rename (PROP-008
  §2.2). This PROP governs how an address is written and how a migration is
  performed mechanically — not the identity semantics, which PROP-008 owns.

## Changelog {#changelog}

- [2026-07-12] Created — ratified alongside the `org.vibevm` →
  `org.vibevm.ai-native` / `org.vibevm.world` group restructure, the first
  refactor performed under §3.
