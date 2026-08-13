# PROP-029 — Fully-qualified addresses and mechanical refactoring {#root}

<status stage="spec" state="done" comment="B0 2026-07-24: accepted 2026-07-12, owner-ratified; re-marked at fact grain (re-pilot) same day"/>

@fact:status-line **Status:** accepted 2026-07-12 (owner-ratified). **Builds on:** [`spec://org.vibevm.core/vibevm/modules/vibe-registry/PROP-008#group`](../modules/vibe-registry/PROP-008-qualified-naming.md#group) (the `group` field) and the **addressable-specs** flow, whose `#modules` unit defines the fully-qualified module authority and the joiner-never-`.` rule this PROP applies: `spec://org.vibevm.world/addressable-specs/flows/addressable-specs/ADDRESSABLE-SPECS-PROTOCOL#modules`. @status:spec/done

## 1. Every address is fully qualified {#decision}

@fact:ADDR-LAW A package address MUST carry its full coordinate — `group` **and** `name` — in **every** occurrence across the project: @status:spec/done

- @fact:ADDR-SURFACES The occurrences it binds: manifests, lockfiles, `spec://` citations, `scope!` markers, `DEVIATES` lines, code comments, and docs. @status:spec/done
- @fact:ADDR-SHORT-NAMES Short or bare names survive only as a one-time human CLI input, resolved to the qualified form at the boundary (PROP-008 §2.6). @status:spec/done
- @fact:ADDR-NO-BARE-ON-DISK Nothing on disk stores a bare name. @status:spec/done

@fact:joiner-why Why the full coordinate is a self-contained global symbol is the addressable-specs `#modules` unit. On the **joiner** vibevm now diverges from that unit's letter by owner ruling (2026-08-13): the flat carrier joins with `.`, because the composite must obey real domain rules, and the unit's premise — «a dotted `<group>.<name>` hides the boundary» — does not hold here: the name grammar guarantees a single dot-free LDH label, so the **last dot** splits the boundary deterministically. The upstream `#modules` unit still states joiner-never-`.`; refreshing it is a named follow-up of the 2026-08-13 landing, not silently ignored. @status:spec/done

@fact:carriers-lead In vibevm the coordinate takes three textual carriers, one identity: @status:spec/done

| Carrier | Form | Example |
|---|---|---|
| @fact:CARRIER-PKGREF pkgref (manifests, lockfiles, prose) @status:spec/done | `[<kind>:]<group>/<name>` @status:spec/done | `stack:org.vibevm.ai-native/rust-ai-native-lang` @status:spec/done |
| @fact:CARRIER-SPEC-URI `spec://` authority (the `<module>` segment) @status:spec/done | `<group>/<name>` — the name is the first path segment @status:spec/done | `spec://org.vibevm.ai-native/rust-ai-native-lang/GUIDE#anchor` @status:spec/done |
| @fact:CARRIER-REPO-NAME repo name (flat, one segment) @status:spec/done | `<group>.<name>` — `/` is illegal in a repo name; the name is the last label @status:spec/done | `org.vibevm.ai-native.rust-ai-native-lang` @status:spec/done |

- @fact:CARRIER-BYTE-IDENTITY Where the surface allows a `/` (pkgref, `spec://`) the `<group>/<name>` coordinate is byte-identical, so one substitution renames both. @status:spec/done
- @fact:CARRIER-REPO-SWAP The flat repo-name carrier swaps `/`→`.` (owner ruling 2026-08-13): GitHub / GitVerse names allow `[A-Za-z0-9._-]`, the dot is legal there, and with both halves LDH the whole composite is a valid reversed FQDN — a real domain string, split back at the last dot. The pre-ruling joiner was `_` (see PROP-008 §2.5 for the record and the rename of live `_`-joined repositories). @status:spec/done

## 2. Why — mechanical refactoring {#rationale}

- @fact:ADDR-STRUCTURE-FREE A fully-qualified address is structure-independent: it does not depend on where a package sits in the tree, which group currently owns it, or how its spec is filed — the stable global symbol the addressable-specs `#modules` unit describes. @status:spec/done
- @fact:ADDR-REFACTOR-PRECONDITION For vibevm that is the precondition for **deterministic, non-LLM address refactoring**: because every reference to a unit is the same self-contained string, a rename is a pure textual substitution. @status:spec/done
- @fact:ADDR-RENAME-IS-LOOKUP Change a name, a group, or an anchor, then rewrite every occurrence, and the inverse — a table lookup, not the judgment call a resolver-dependent short address would need. @status:spec/done

## 3. The mechanical-refactoring foundation {#mechanical}

- @fact:REFACTOR-LAW Address refactors — rename a package, move a group, rename a cited anchor — MUST be expressible as deterministic substitutions over fully-qualified strings, verified by grep-zero of the old coordinate. @status:spec/done
- @fact:REFACTOR-TODAY The reference implementation today is a scripted `sed` transform with grep verification and a specmap re-mint. @status:spec/done
- @fact:REFACTOR-TARGET The target is a first-class **rename engine** (a future FEAT) that takes `(old-coordinate → new-coordinate)` and rewrites every manifest, lockfile, spec URI, and marker, then regenerates the specmap and the derived lockfiles/vibedeps. @status:spec/done
- @fact:REFACTOR-DEPENDS-ON-LAW The engine is only possible while §1 holds — the day one bare name lands on disk, a rename needs a resolver again. @status:spec/done

## 4. Scope and exceptions {#scope}

- @fact:SCOPE-HOST <status stage="spec" state="void">Retired 2026-08-04 by B-031 (owner-approved): the host exemption is gone — **the root project IS a package coordinate**, `group = "org.vibevm.core"`, `name = "vibevm"` in the root `vibe.toml`, addressed `spec://org.vibevm.core/vibevm/…` like every other package; §1 binds it too. The retired short authority `spec://vibevm/…` parses (undotted-authority grammar survives for fixtures and legibility) and **never resolves** — the resolver answers `LegacyHostAuthority` with a rename hint naming the self coordinate. The self coordinate resolves to the workspace's own authored `spec/` tree, never a `vibedeps/` slot. Migration record: 1 893 living-surface occurrences rewritten in one pass, 2026-08-04; rationale: [`spec/design/host-as-package.md`](../design/host-as-package.md). This tombstone stays so the old sentence's name is never reused and inbound links do not break.</status> @status:spec/void
- @fact:SCOPE-SELF-COORDINATE **The self coordinate.** The root project declares its package identity in `[project]` (`group` + `name`, PROP-008 semantics); its authored `spec/` tree answers to `spec://<group>/<name>/…` — the *self coordinate*, matched by the resolver before any slot lookup and never versioned (`@version` on it is an error). A project that declares no `group` has no self coordinate, and its authored tree is unreachable by address. @status:spec/work
- @fact:SCOPE-FIXTURES **Test fixtures and grammar examples** (`spec://demo/…`, `spec://com.example.shop/…`, and the like) are illustrative, not real packages; they are out of scope and stay as written. @status:spec/done
- @fact:SCOPE-GROUP-CHANGE Changing a package's `group` is a **new package**, not a rename (PROP-008 §2.2). <status stage="spec" state="done">This PROP governs how an address is written and how a migration is performed mechanically — not the identity semantics, which PROP-008 owns.</status> @status:spec/done

## Changelog {#changelog}

- @fact:CHANGELOG-CREATED [2026-07-12] Created — ratified alongside the `org.vibevm` → `org.vibevm.ai-native` / `org.vibevm.world` group restructure, the first refactor performed under §3. @status:spec/done
- @fact:CHANGELOG-EXTRACTED [2026-07-14] The addressing principle (fully-qualified module authority, joiner-never-`.`) was extracted to the `addressable-specs` flow's `#modules` unit (reaching vibevm through redbook); §1–2 now cite it and keep only vibevm's concrete carriers, the mechanical-refactoring foundation, and the scope rules. @status:spec/done
- @fact:CHANGELOG-B031 [2026-08-04] **The host exemption retired (B-031, owner-approved).** The root project became the package coordinate `org.vibevm.core/vibevm`; `##SCOPE-HOST` is a tombstone, `##SCOPE-SELF-COORDINATE` carries the live rule, and the authority rename (`spec://vibevm/…` → `spec://org.vibevm.core/vibevm/…`, 1 893 living-surface occurrences) was performed under §3's mechanical-refactoring foundation in one scripted pass. Design record: [`spec/design/host-as-package.md`](../design/host-as-package.md). @status:spec/work
- @fact:CHANGELOG-DOT-JOINER [2026-08-13] **The flat carrier re-ruled to the dot join.** The owner's identity rulings («настоящие домены») narrowed group segments to LDH labels and made the repo-name carrier `<group>.<name>` — a valid reversed FQDN, split at the last dot. `##joiner-why` and `##CARRIER-REPO-SWAP` record the divergence from the addressable-specs `#modules` letter and why its premise no longer holds; PROP-008 §2.1/§2.5 carry the grammar and the convention. Live `_`-joined repositories in `vibespecs` rename as a follow-up (pre-public, host redirects cover old names). @status:spec/done
