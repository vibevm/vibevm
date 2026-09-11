# Authoring a `flow` package

A **flow** is a discipline / process module — a set of conventions, protocols, and reminders an AI agent reads at session start so it follows the team's working agreements. Flows are the "how we work" half of vibevm; feats and stacks are the "what we build" half.

Examples shipped today:
- `flow:wal` — the Write-Ahead Log discipline.
- `flow:sync-from-code` — the protocol for reconciling spec drift from code.
- `flow:atomic-commits` — one-commit-per-idea + Conventional Commits format.

A flow is *content*. There is no executable component, no LLM call, no build artefact — at install time a flow's published tree is materialised verbatim into a `vibedeps/` slot, and its boot snippet is folded into each consuming node's computed boot sequence so the AI session reads it.

## Anatomy of a flow package

```
flow-<name>/                 # the per-package repo on the registry
├── vibe.toml                # required; manifest, carries a [package] table
├── README.md                # required; human-readable description
├── boot/
│   └── flow-<name>.md       # the boot-snippet content
└── spec/
    └── flows/
        └── <name>/
            ├── PROTOCOL.md         # canonical "what is this discipline"
            ├── <subprotocol-1>.md  # supporting docs, broken out by topic
            └── <subprotocol-2>.md
```

After `vibe install flow:<name>`, the package's whole published tree is materialised verbatim into a slot under the workspace-root `vibedeps/` tree:

```
<workspace-root>/
└── vibedeps/
    └── flow-<name>/
        └── <version>/                  # the flow's published tree, verbatim
            ├── vibe.toml
            ├── boot/flow-<name>.md
            └── spec/flows/<name>/
                ├── PROTOCOL.md
                └── …
```

This is the **loading model** ([PROP-009](../spec/modules/vibe-workspace/PROP-009-loading-model.md), [docs/loading-model.md](loading-model.md)): a materialised package *is* its verbatim subtree under its `vibedeps/` slot. `vibe install` never writes into a consuming node's authored `spec/` — the C++-`#include` rule. There is no per-file write list to author: drop `[writes]` entirely. Cross-references inside your package must be package-relative or `spec://` URIs.

## The boot snippet

A flow contributes one boot snippet, declared in the `[boot_snippet]` table (see the manifest below). You declare a **`category`**, not a numbered filename — `vibe` owns boot ordering, computing each consuming node's boot sequence from the resolution graph. The two-digit `NN-` filename prefix and the flat numeric-order `spec/boot/` directory are **retired**.

A flow's snippet is conventionally `category = "flow"`. Within a node's computed boot sequence the order is `foundation` → the node's own authored boot → dependency boot (topological — a dependency before its dependents) → `user-override`. Prefix collisions are impossible by construction — there is no prefix — so there is no longer an exit-3 prefix-conflict check to design around. The snippet's `source` field gives the path to the boot file inside your package; name it whatever you like (`boot/flow-<name>.md` is the convention).

## Manifest: `vibe.toml`

A publishable package carries a `vibe.toml` with a `[package]` table. Minimal:

```toml
[package]
name = "atomic-commits"
kind = "flow"
version = "0.1.0"
authors = ["You <you@example.com>"]
license = "EULA"
description = "One commit = one idea. Conventional Commits format."
keywords = ["commits", "discipline", "conventional-commits"]

[compatibility]
min_vibe_version = "0.1.0"

[boot_snippet]
category = "flow"
source = "boot/flow-atomic-commits.md"
# Optional: a suggested default link type. The consumer can override
# it in their own [requires.packages] entry; absent both, it is `static`.
# link = "static"
```

`[boot_snippet]` declares a `category` (`foundation` / `flow` / `stack` / `user-override`) and a `source` (the path to the boot file inside the package). There is **no `filename` field** and **no `[writes]` section** — `vibe` owns boot ordering and a materialised package is simply its verbatim `vibedeps/` subtree.

`[provides]`, `[requires]`, `[[requires_any]]`, `[obsoletes]`, `[conflicts]` are all optional. A typical flow has none of them — it's self-contained content. Use them when:

- Your flow advertises a capability another package may consume (`[provides].capabilities`).
- Your flow assumes the project also follows another flow it depends on (`[requires].packages`).
- Your flow supersedes an older one (`[obsoletes].packages`).

See [`VIBEVM-SPEC.md` §7.3](../VIBEVM-SPEC.md) for the full manifest schema.

## Writing the boot snippet

The boot snippet is the *only* file from your package that the AI agent is guaranteed to read every session — `vibe` folds it into the consuming node's computed boot sequence, read before the rest of the spec. Treat it as the front page.

Good boot snippet shape:

1. **One sentence describing the discipline** — what does this flow ask of the agent?
2. **A pointer to `PROTOCOL.md`** — `For the full protocol, see spec://<project>/flows/<name>/PROTOCOL`.
3. **The non-negotiable rules**, terse and numbered.
4. **Any links to subprotocols** worth pulling in for specific tasks.

Keep it under ~80 lines. Boot snippets compete for the agent's attention budget; brevity wins.

## Writing the protocol

`spec/flows/<name>/PROTOCOL.md` carries the full discipline. Sections worth thinking about:

- **What problem this flow solves** — why does the team adopt it?
- **The protocol** — concrete steps, with examples.
- **Anti-patterns** — what this flow forbids and why.
- **Edge cases** — recurring questions and the agreed answers.
- **References** — books, prior art, original sources.

Subprotocols (`spec/flows/<name>/<topic>.md`) hold detail for specific situations — pull them out when `PROTOCOL.md` would otherwise grow past comfortable reading length.

## Versioning

Versions are git tags on the per-package repo, prefixed `v` (e.g. `v0.1.0`). Bump rules follow [SemVer](https://semver.org):

- **Patch** (`0.1.0` → `0.1.1`): wording fixes, typo corrections, no semantic change to the protocol.
- **Minor** (`0.1.0` → `0.2.0`): additive changes — new optional subsections, new examples, expanded protocol that doesn't remove existing rules.
- **Major** (`0.1.0` → `1.0.0`): breaking change — rules removed or replaced, the boot snippet's `category` changed, `[requires]` added.

Pre-1.0 (`0.x`), even minor bumps may break consumers — that's the SemVer convention for unstable APIs.

## Publishing

Once your package directory has a manifest, README, boot snippet, and content files, publish through the maintainer command:

```bash
vibe registry publish ./path/to/your/flow-package
```

See [`vibe registry publish`](commands/registry-publish.md) for the full token / authentication / error model. The first publish creates the per-package repo under your registry's organization; subsequent versions reuse it and push new tags.

## Tips

- **Read existing flows first.** [`flow:wal`](https://gitverse.ru/anarchic/vibespecs) is short and well-formed; use it as a structural template.
- **Test with the local-fixture path before publishing.** `vibe install flow:<name> --registry ./path/to/your/dir` against a directory laid out in the M0 monorepo shape lets you iterate without going through `git push` round-trips.
- **Boot snippets are user-facing prose.** Write them like you're briefing a teammate. The AI agent is the immediate reader, but humans review the boot files; make both happy.
- **Don't put runtime logic in a flow.** A flow is content. If your idea needs a build step or a tool invocation, it's a `feat` or a `tool`, not a `flow`.

## Related

- [`vibe install`](commands/install.md) — installing a flow.
- [The loading model](loading-model.md) — how a boot snippet's `category` and `link` feed the computed boot sequence.
- [PROP-009](../spec/modules/vibe-workspace/PROP-009-loading-model.md) — the loading-model contract.
- [authoring-feat.md](authoring-feat.md) — feats are the runtime-y counterpart of flows.
