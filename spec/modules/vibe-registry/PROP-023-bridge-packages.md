# PROP-023 — Bridge packages {#root}

**Status:** proposed 2026-06-24 — owner-requested design session. The umbrella
spec of the four; it composes three orthogonal mechanisms —
[PROP-020](../vibe-workspace/PROP-020-install-hooks.md) install hooks,
[PROP-021](PROP-021-submodule-sources.md) submodule sources,
[PROP-022](../vibe-workspace/PROP-022-materialization-modes.md) materialization
modes — but adds little of its own: a flag and a packaging convention. Each
mechanism stands alone; bridges are where they line up.
**Related:** [PROP-002](PROP-002-decentralized-registry.md) (a bridge is an
ordinary package + identity), [PROP-008](PROP-008-qualified-naming.md) (the
maintainer's group, distinct from the upstream author),
[PROP-015 §2.6](../vibe-mcp/PROP-015-mcp-integration.md#skill) +
[PROP-015 #skill-include](../vibe-mcp/PROP-015-mcp-integration.md#skill-include)
(projecting a skill out of the bridged subtree, selectively),
[PROP-000 §16](../../common/PROP-000.md) (the installable kinds a bridge
still belongs to).

---

## 1. Motivation {#motivation}

### 1.1 The problem — good work that nobody packaged {#problem}

People publish skills and projects to GitHub / GitVerse without ever making a
vibevm package — out of disinterest, or because their repo's layout has nothing
to do with vibevm conventions. That work is then unreachable through
`vibe install`, and the original author has no incentive to change.

A **bridge package** closes the gap without the author's involvement: a
*maintainer* volunteers to steward someone else's repository and publishes an
ordinary vibevm package that **wraps** it. The bridge brings the foreign repo
in by one of git's two means and, where needed, runs hooks to shape it into
something vibevm can consume.

### 1.2 What this is — a thin convention over three mechanisms {#what}

A bridge is not a new kind of package or a new subsystem. It is:

- an ordinary package (still one of `flow` / `feat` / `stack` / `tool`),
- carrying the foreign repo as **vendored** content or a **submodule**
  ([PROP-021](PROP-021-submodule-sources.md)),
- optionally **prepared** by install hooks ([PROP-020](../vibe-workspace/PROP-020-install-hooks.md)),
- optionally **materialised** as `in-place` when the upstream is a giant
  ([PROP-022](../vibe-workspace/PROP-022-materialization-modes.md)),
- with any skill projected selectively from the bridged subtree
  ([PROP-015 #skill-include](../vibe-mcp/PROP-015-mcp-integration.md#skill-include)).

…plus one flag that says "this is a bridge."

## 2. Decisions {#decisions}

### 2.1 A bridge is marked by a flag, not a kind {#flag}

`req r1`

`[package].bridge = true` marks a package as a bridge. It does **not** change
the package's `kind` (a bridged skill is still a `feat`/`tool` as appropriate)
or its identity. The flag is metadata with two jobs: it documents that the
package's substantive content is *foreign* (stewarded, not authored, by the
maintainer), and it is the hook the registry/UI uses to surface provenance
(§2.4). Default `false`; the overwhelming majority of packages are not bridges.

### 2.2 Two classes — vendored and submodule-backed {#classes}

`req r1`

A bridge embeds the upstream repo one of two ways:

- **Vendored ("git in git")** — the maintainer copied the upstream tree into
  the package and committed it. This needs **none** of PROP-021/022 machinery:
  it is plain files in a `snapshot` package. A vendored bridge is therefore the
  cheapest case — the flag (§2.1) plus, if the layout needs shaping, hooks.
- **Submodule-backed** — the package references the upstream via a git
  submodule ([PROP-021](PROP-021-submodule-sources.md)), fetched on install and
  updated on update, so the bridge tracks upstream without re-vendoring.

The maintainer chooses per trade-off: vendored is self-contained and frozen;
submodule-backed stays current but depends on upstream remaining reachable.

### 2.3 Composition — every mechanism is optional {#composition}

`req r1`

A bridge is the point where the three orthogonal mechanisms compose, but it
mandates none of them:

- a bridge **without hooks** is valid (the upstream layout already fits);
- a bridge **without a submodule** is valid (vendored);
- a bridge **without `in-place`** is the norm (`in-place` is only for giant
  upstreams).

The canonical full case — submodule-backed + `pre-install` hook to shape the
tree + selective skill projection — is the union of the four specs, but each
piece is independently usable outside any bridge. This is why they are four
specs and four test sets, not one (the owner's orthogonality requirement).

### 2.4 The maintainer model {#maintainer-model}

`req r1`

The bridge's **maintainer** is distinct from the upstream **author**. The
package's `group` ([PROP-008](PROP-008-qualified-naming.md)) is the
maintainer's namespace; the upstream is recorded as provenance (its repository
URL, and — where applicable — the `describes` PURL the descriptor already
supports, [PROP-015 §0](../vibe-mcp/PROP-015-mcp-integration.md)). A consumer
can therefore always see *who stewards this* and *what it wraps*. A skill that
lives inside the bridged subtree is projected through the normal skill
machinery, using the `include` selector
([PROP-015 #skill-include](../vibe-mcp/PROP-015-mcp-integration.md#skill-include))
to pick the relevant files out of an upstream tree full of unrelated content.

## 3. Rejected alternatives {#rejected}

- **A `bridge` package kind** (a kind of its own beside the §4.1 register) —
  rejected: the kinds describe *what the package is for*; "bridge"
  describes *where its content came from*. They are orthogonal axes, so bridge
  is a flag, not a kind. (The reasoning survives the register later growing
  `mcp` — that kind, too, says what a package is FOR.)
- **Auto-importing a foreign repo with no maintainer** — rejected: someone must
  take responsibility for shaping, updating, and vouching for the wrapped code;
  an unowned auto-bridge has no one to fix it when upstream moves or breaks.

## 4. Out of scope {#out-of-scope}

- **Automatic conversion of foreign layouts** into vibevm conventions — bridges
  shape upstream with explicit, maintainer-written hooks, not inferred magic.
- **Security scanning of wrapped third-party code** — the LLM "antivirus" is
  the same far-backlog item as for hooks
  ([PROP-020 §4](../vibe-workspace/PROP-020-install-hooks.md#out-of-scope)); a
  bridge's trust posture is its maintainer plus the hook allow-list/consent
  gate, an explicitly accepted risk for now.
- **License / legal provenance tracking** of wrapped upstreams — beyond the
  recorded URL/PURL, no compliance machinery is specified here.

## 5. Acceptance {#acceptance}

- `[package].bridge` parses as a boolean, defaults `false`, and does not alter
  `kind` or identity.
- A vendored bridge installs as a plain `snapshot` package (flag + optional
  hooks), with no submodule/materialization machinery engaged.
- A submodule-backed bridge fetches upstream ([PROP-021](PROP-021-submodule-sources.md)),
  runs declared hooks ([PROP-020](../vibe-workspace/PROP-020-install-hooks.md)),
  and projects its skill via the `include` selector.
- Provenance (maintainer group + upstream URL/PURL) is recoverable for a bridge
  package.
- Full `self-check.sh` green; conform 0/0/0; specmap clean.
