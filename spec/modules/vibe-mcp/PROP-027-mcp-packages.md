# PROP-027 — `mcp` packages: the agent-server kind and its delivery {#root}

**Milestone:** M1.26 candidate («MCP sovereignty» —
[MCP-SOVEREIGNTY-PLAN-v0.1](../../terraforms/MCP-SOVEREIGNTY-PLAN-v0.1.md)).
**Status:** IMPLEMENTED — the kind and the manifest laws (§2.1–§2.3)
shipped with the plan's Wave 1; the servers themselves (Waves 3–4:
`mcp:org.vibevm/rust-ai-native-mcp`, `…/discipline-typescript`, both
live-chained vibe-free); the registration lifecycle (§2.4–§2.5) with
Wave 5 (`vibe mcp install/uninstall/status` speak package servers; the
pin-server fixture e2e pins the walk). §2.7's composition rows inherit
the kind-agnostic feature suites — no feature branches on `kind`, and
the mcp e2e exercises code-bearing + binaries + the pin end to end. Units typed at REQ grain; the code carries the matching
`scope!` / `#[spec(implements)]` / `#[verifies]` edges.
**Related:** [PROP-015](PROP-015-mcp-integration.md) (the product MCP
server + the agent-integration command family this PROP extends),
[PROP-025](../vibe-workspace/PROP-025-binary-delivery.md) (the binary
delivery an `[[mcp_server]]` rides), [PROP-024](../../common/PROP-024-code-bearing-packages.md)
(code-bearing packages; §2.4 is why mcp packages vendor),
[`VIBEVM-SPEC.md` §4.1](../../../VIBEVM-SPEC.md) (the kind register,
amended under owner sanction 2026-07-07).

---

## 1. Motivation {#motivation}

The discipline stacks ship engines, gates, CLIs, and type oracles —
everything an agent-facing toolchain needs EXCEPT the transport. The
prototype topology served them through vibevm's own MCP
(`vibe mcp serve` + the `tcg_*` adapters), which put the whole vibevm
product into every consumer's runtime path and closed an operational
cycle over vibevm itself (the tool served its own development). The
owner's resolution (2026-07-07): agent-server delivery is a first-class
package concern — a KIND, not a bolt-on surface — and vibe's job there
is install-time wiring, never serving.

## 2. Decisions {#decisions}

### 2.1 The `mcp` kind {#kind}

`req r1`

An **`mcp` package** is one whose primary deliverable is one or more
Model Context Protocol servers. `kind = "mcp"` joins the installable
register (VIBEVM-SPEC §4.1); slots materialise under
`vibedeps/mcp-<name>/<version>/` like every other kind. The
`[[mcp_server]]` table (§2.2) is **legal only in this kind** — the kind
IS the taxonomy, enforced by `Manifest::validate`, not advisory. An
`mcp`-kind manifest that declares NO `[[mcp_server]]` is refused: the
kind promises a server.

### 2.2 The `[[mcp_server]]` declaration {#manifest}

`req r1`

```toml
[[mcp_server]]
name = "discipline-rust"          # agent-visible server name, unique in the package
binary = "discipline-mcp-rust"    # must match a [[binary]] in this manifest
description = "AI-Native Rust discipline + type oracle over MCP"
args = ["--path", "{project_root}"]
```

The server IS a [PROP-025](../vibe-workspace/PROP-025-binary-delivery.md)
binary: delivery, consent, staleness, and slot residence come from that
machinery wholesale — `binary` must resolve to a `[[binary]]` declared
in the same manifest. `name` is what an MCP host shows as the tool
namespace; names are unique within the package. `args` may carry
substitution tokens ONLY from the closed set `{project_root}` (the
absolute, verbatim-free root of the consuming project, resolved at
registration time); unknown `{…}` tokens are refused at validation.

### 2.3 The exact-pin law {#exact-pin}

`req r1`

Cargo path-deps cannot cross package slots (PROP-024 §2.4), so an mcp
package VENDORS the crates of the toolchain it serves. Vendoring
re-opens the version skew the in-slot prototype excluded by
construction: a server built from engine copy X enriching against a
consumer whose gates run engine copy Y. The pin closes it: **every
`[requires.packages]` entry of an `mcp`-kind package MUST be an exact
`=X.Y.Z` requirement** — the resolver holds the served engines and the
consumer's gates to ONE version set; no runtime handshake exists or is
needed. `Manifest::validate` refuses any other requirement shape
(caret, bare, partial `=X.Y`, compound ranges). Git-source deps pin by
rev inherently; path-source deps are local-dev surfaces outside this
law. The operational consequence is accepted and priced by the plan:
the mcp package bumps in lockstep with the package it serves.

### 2.4 Registration: `vibe mcp install` learns packages {#registration}

`req r1`

`vibe mcp install` today writes vibevm's own server into agent configs
(PROP-015). It grows package discovery: every installed package of kind
`mcp` contributes its `[[mcp_server]]` entries, written into the target
agents' configs with

- `command` = the absolute, **verbatim-free** path to the slot-resident
  built artifact (a real executable — no shim, no `cmd /c` wrapper
  class), `args` with the closed-set substitutions resolved;
- a **managed sidecar**: a top-level `"vibevm": { "managed": [...] }`
  object in the JSON config names the entries vibevm owns (never a key
  INSIDE a server entry — hosts validate entry shapes), so re-installs
  rewrite ONLY vibevm-managed entries and operator-owned servers are
  never touched — the `<vibevm>` block convention of the boot files,
  applied to agent configs. Registration is PROJECT-scope only (the
  `{project_root}` substitution demands a project, and a project's
  servers belong in its committed config), and every project-scope
  agent config is JSON — so no TOML sidecar form exists;
- lifecycle: `vibe mcp install` re-run refreshes paths after a slot
  move; `vibe mcp status` reports each declared server's artifact state
  (an unbuilt artifact registers fine and fails at agent launch — the
  recipe names `vibe bin build <name>`); `vibe mcp uninstall` removes
  managed entries plus the emptied sidecar, and nothing else.

### 2.5 Consent: registration is the same trust act as building {#consent}

`req r1`

Registering a server schedules package code execution at agent-session
start; building its binary compiles package code. One trust model, two
verbs: registration inherits PROP-025's consent gate verbatim —
`org.vibevm` packages are allow-listed; any other origin requires the
explicit `--assume-yes` (or is refused with the recipe naming that
exact flag). Registration writes touch ONLY the target agent's config
files and only managed entries; server processes receive the project
root as cwd and NO secrets from vibe.

### 2.6 Serving is vibe-free {#vibe-free}

`req r1`

An mcp package's servers run without vibe: the artifact is launched by
the agent host directly from the slot path, links its vendored engines,
and speaks stdio MCP. `vibe` is required only to install, build, and
register. The acceptance form of this requirement: the server's live
chain passes with `vibe` absent from `PATH` and no vibevm process
running.

### 2.7 Composition: an mcp package is a full package {#composition}

`req r1`

Every package-role feature applies to `mcp` packages exactly as to the
other kinds; no feature branches on `kind`, so each row inherits its
feature's own kind-agnostic suite (the mcp-kind e2e adds the
code-bearing + binaries + exact-pin composition end to end):

| Feature | Spec | Composition rule |
|---|---|---|
| Code-bearing layout | PROP-024 | the package IS code-bearing by definition; `spec/` for prompt content, crates at root |
| Binaries | PROP-025 | `[[mcp_server]].binary` references them; `vibe bin list/build/exec` see them like any other |
| Skills | PROP-015 §2.8, PROP-018 §2.4 | `[[skill]]` legal; a server may ship its teaching skill |
| Boot snippet | PROP-009 | legal but not required; agents learn servers via registration, not boot text |
| Hooks | PROP-020 | pre/post-install hooks run in the slot as usual |
| Materialization modes | PROP-022 | snapshot/in-place per the standard rules |
| Bridges / submodules | PROP-023 / PROP-021 | no special-casing |
| Publish | PROP-002 §2.10 | standard registry publish; the exact-pin law travels in the manifest |
| In-workspace mutability | PROP-011 §2.6 | dev-loop re-materialisation applies |

## 3. Rejected alternatives {#rejected}

- **`[[mcp_server]]` as an any-kind surface** (the plan's original
  draft): rejected by the owner — the taxonomy should say what a
  package IS; embedded servers would blur the register and hide the
  vendoring/pinning obligations §2.3 makes explicit.
- **Cross-slot path-deps or manifest rewriting** instead of vendoring:
  PROP-025 v2 territory, specified-only; the reproducible-hash model
  (PROP-024 §2.2) forbids post-materialise rewriting today.
- **A runtime version handshake** instead of the exact pin: weaker (it
  detects skew instead of preventing it) and needs a wire surface;
  the resolver already enforces equality for free.
- **vibe as the server launcher** (`vibe mcp exec <name>` in agent
  configs): would keep vibe in the runtime path — the exact property
  this PROP removes.

## 4. Open questions {#open}

1. Multi-server packages (legal today) — does `vibe mcp install` offer
   per-server opt-out? v1: all-or-nothing per package.
2. The `app` kind (anticipated, VIBEVM-SPEC §4.1) — whether it reuses
   §2.4's managed-entry machinery for desktop-integration surfaces.
3. Stable artifact paths (PROP-025 v2 shims) would make managed entries
   survive version bumps without a re-install; deferred with it.
