# PROP-023 — Bridge packages {#root}

<status stage="impl" state="done" comment="C 2026-07-25: [package].bridge parses in vibe-core and all three composed mechanisms ship; motivation, rejected and out-of-scope facts stay spec-stage; fact grain 2026-07-24"/>

@fact:status-line **Status: IMPLEMENTED** (specified 2026-06-24 in an owner-requested design
session; verified against the tree 2026-07-25 by the spec-actualization
campaign). The umbrella's own addition — `[package].bridge`, defaulting to
false — parses in `vibe-core` with a doctest, and the three mechanisms it
composes all ship: [PROP-020](../vibe-workspace/PROP-020-install-hooks.md)
install hooks, [PROP-021](PROP-021-submodule-sources.md) submodule sources,
[PROP-022](../vibe-workspace/PROP-022-materialization-modes.md) materialization
modes — so every composition law recorded here describes live machinery. It
still adds little of its own: a flag and a packaging convention. Each mechanism
stands alone; bridges are where they line up. @status:impl/done

@fact:related **Related:** [PROP-002](PROP-002-decentralized-registry.md) (a bridge is an
ordinary package + identity), [PROP-008](PROP-008-qualified-naming.md) (the
maintainer's group, distinct from the upstream author),
[PROP-015 §2.6](../vibe-mcp/PROP-015-mcp-integration.md#skill) +
[PROP-015 #skill-include](../vibe-mcp/PROP-015-mcp-integration.md#skill-include)
(projecting a skill out of the bridged subtree, selectively),
[PROP-000 §16](../../common/PROP-000.md) (the installable kinds a bridge
still belongs to). @status:spec/done

---

## 1. Motivation {#motivation}

### 1.1 The problem — good work that nobody packaged {#problem}

- @fact:unpackaged-work People publish skills and projects to GitHub / GitVerse without ever making a
  vibevm package — out of disinterest, or because their repo's layout has nothing
  to do with vibevm conventions. @status:spec/done
- @fact:unreachable That work is then unreachable through
  `vibe install`, and the original author has no incentive to change. @status:spec/done

- @fact:BRIDGE-DEF A **bridge package** closes the gap without the author's involvement: a
  *maintainer* volunteers to steward someone else's repository and publishes an
  ordinary vibevm package that **wraps** it. @status:impl/done
- @fact:BRIDGE-MEANS The bridge brings the foreign repo
  in by one of git's two means and, where needed, runs hooks to shape it into
  something vibevm can consume. @status:impl/done

### 1.2 What this is — a thin convention over three mechanisms {#what}

@fact:thin-convention A bridge is not a new kind of package or a new subsystem. It is: @status:impl/done

- @fact:conv-ordinary-package an ordinary package (still one of `flow` / `feat` / `stack` / `tool`), @status:impl/done
- @fact:conv-embedded-content carrying the foreign repo as **vendored** content or a **submodule**
  ([PROP-021](PROP-021-submodule-sources.md)), @status:impl/done
- @fact:conv-hooks optionally **prepared** by install hooks ([PROP-020](../vibe-workspace/PROP-020-install-hooks.md)), @status:impl/done
- @fact:conv-in-place optionally **materialised** as `in-place` when the upstream is a giant
  ([PROP-022](../vibe-workspace/PROP-022-materialization-modes.md)), @status:impl/done
- @fact:conv-skill-projection with any skill projected selectively from the bridged subtree
  ([PROP-015 #skill-include](../vibe-mcp/PROP-015-mcp-integration.md#skill-include)). @status:impl/done

@fact:plus-flag …plus one flag that says "this is a bridge." @status:impl/done

## 2. Decisions {#decisions}

### 2.1 A bridge is marked by a flag, not a kind {#flag}

@fact:req-flag `req r1` @status:impl/done

- @fact:BRIDGE-FLAG `[package].bridge = true` marks a package as a bridge. @status:impl/done
- @fact:FLAG-NOT-KIND It does **not** change
  the package's `kind` (a bridged skill is still a `feat`/`tool` as appropriate)
  or its identity. @status:impl/done
- @fact:FLAG-JOBS The flag is metadata with two jobs: it documents that the
  package's substantive content is *foreign* (stewarded, not authored, by the
  maintainer), and it is the hook the registry/UI uses to surface provenance
  (§2.4). @status:impl/done
- @fact:FLAG-DEFAULT Default `false`; the overwhelming majority of packages are not bridges. @status:impl/done

### 2.2 Two classes — vendored and submodule-backed {#classes}

@fact:req-classes `req r1` @status:impl/done

@fact:TWO-CLASSES A bridge embeds the upstream repo one of two ways: @status:impl/done

- @fact:CLASS-VENDORED **Vendored ("git in git")** — the maintainer copied the upstream tree into
  the package and committed it. This needs **none** of PROP-021/022 machinery:
  it is plain files in a `snapshot` package. A vendored bridge is therefore the
  cheapest case — the flag (§2.1) plus, if the layout needs shaping, hooks. @status:impl/done
- @fact:CLASS-SUBMODULE **Submodule-backed** — the package references the upstream via a git
  submodule ([PROP-021](PROP-021-submodule-sources.md)), fetched on install and
  updated on update, so the bridge tracks upstream without re-vendoring. @status:impl/done

@fact:CLASS-TRADEOFF The maintainer chooses per trade-off: vendored is self-contained and frozen;
submodule-backed stays current but depends on upstream remaining reachable. @status:spec/done

### 2.3 Composition — every mechanism is optional {#composition}

@fact:req-composition `req r1` @status:impl/done

@fact:COMPOSE-OPTIONAL A bridge is the point where the three orthogonal mechanisms compose, but it
mandates none of them: @status:impl/done

- @fact:OPT-NO-HOOKS a bridge **without hooks** is valid (the upstream layout already fits); @status:impl/done
- @fact:OPT-NO-SUBMODULE a bridge **without a submodule** is valid (vendored); @status:impl/done
- @fact:OPT-NO-IN-PLACE a bridge **without `in-place`** is the norm (`in-place` is only for giant
  upstreams). @status:impl/done

- @fact:CANONICAL-FULL-CASE The canonical full case — submodule-backed + `pre-install` hook to shape the
  tree + selective skill projection — is the union of the four specs, but each
  piece is independently usable outside any bridge. @status:impl/done
- @fact:FOUR-SPECS-WHY This is why they are four
  specs and four test sets, not one (the owner's orthogonality requirement). @status:spec/done

### 2.4 The maintainer model {#maintainer-model}

@fact:req-maintainer `req r1` @status:impl/done

- @fact:MAINTAINER-VS-AUTHOR The bridge's **maintainer** is distinct from the upstream **author**. @status:impl/done
- @fact:GROUP-PROVENANCE The
  package's `group` ([PROP-008](PROP-008-qualified-naming.md)) is the
  maintainer's namespace; the upstream is recorded as provenance (its repository
  URL, and — where applicable — the `describes` PURL the descriptor already
  supports, [PROP-015 §0](../vibe-mcp/PROP-015-mcp-integration.md)). @status:impl/done
- @fact:PROVENANCE-VISIBLE A consumer
  can therefore always see *who stewards this* and *what it wraps*. @status:impl/done
- @fact:SKILL-VIA-INCLUDE A skill that
  lives inside the bridged subtree is projected through the normal skill
  machinery, using the `include` selector
  ([PROP-015 #skill-include](../vibe-mcp/PROP-015-mcp-integration.md#skill-include))
  to pick the relevant files out of an upstream tree full of unrelated content. @status:impl/done

## 3. Rejected alternatives {#rejected}

- @fact:REJ-BRIDGE-KIND **A `bridge` package kind** (a kind of its own beside the §4.1 register) —
  rejected: the kinds describe *what the package is for*; "bridge"
  describes *where its content came from*. They are orthogonal axes, so bridge
  is a flag, not a kind. (The reasoning survives the register later growing
  `mcp` — that kind, too, says what a package is FOR.) @status:spec/done
- @fact:REJ-AUTO-IMPORT **Auto-importing a foreign repo with no maintainer** — rejected: someone must
  take responsibility for shaping, updating, and vouching for the wrapped code;
  an unowned auto-bridge has no one to fix it when upstream moves or breaks. @status:spec/done

## 4. Out of scope {#out-of-scope}

- @fact:OOS-AUTO-CONVERSION **Automatic conversion of foreign layouts** into vibevm conventions — bridges
  shape upstream with explicit, maintainer-written hooks, not inferred magic. @status:spec/done
- @fact:OOS-SECURITY-SCAN **Security scanning of wrapped third-party code** — the LLM "antivirus" is
  the same far-backlog item as for hooks
  ([PROP-020 §4](../vibe-workspace/PROP-020-install-hooks.md#out-of-scope)); a
  bridge's trust posture is its maintainer plus the hook allow-list/consent
  gate, an explicitly accepted risk for now. @status:spec/done
- @fact:OOS-LICENSE-TRACKING **License / legal provenance tracking** of wrapped upstreams — beyond the
  recorded URL/PURL, no compliance machinery is specified here. @status:spec/done

## 5. Acceptance {#acceptance}

- @fact:ACC-FLAG-PARSES `[package].bridge` parses as a boolean, defaults `false`, and does not alter
  `kind` or identity. @status:impl/done
- @fact:ACC-VENDORED-PLAIN A vendored bridge installs as a plain `snapshot` package (flag + optional
  hooks), with no submodule/materialization machinery engaged. @status:impl/done
- @fact:ACC-SUBMODULE-FULL A submodule-backed bridge fetches upstream ([PROP-021](PROP-021-submodule-sources.md)),
  runs declared hooks ([PROP-020](../vibe-workspace/PROP-020-install-hooks.md)),
  and projects its skill via the `include` selector. @status:impl/done
- @fact:ACC-PROVENANCE Provenance (maintainer group + upstream URL/PURL) is recoverable for a bridge
  package. @status:impl/done
- @fact:ACC-FLOOR-GREEN Full `self-check.sh` green; conform 0/0/0; specmap clean. @status:spec/done
