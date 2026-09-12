# Индекс страниц и якорей руководства ядра {#root}

<status stage="doc" state="work" comment="генерируется скриптом campaigns/docs-2026-09/tasks/page-index.py из страниц пакета vibevm-docs; руками не править"/>

Страниц: 44; секций с якорями: 275; примеров: 59; промптов: 19; ссылок `rule`: 381; блоков `derived`: 72; фигур: 0.

| Страница | Якорь | Заголовок | Первые слова |
|---|---|---|---|
| `agent/ask-your-agent` | `root` | Ask your agent to do the work | |
| `agent/ask-your-agent` | `the-prompt-blocks` | The requests on these pages | A task page opens with a block you can copy into any |
| `agent/ask-your-agent` | `how-vibe-knows` | How vibe knows an agent is calling | vibe behaves the same whether a person or an agent types the |
| `agent/ask-your-agent` | `two-transports` | Two ways for an agent to reach vibe | An agent can run vibe as a command, one process per call, |
| `agent/ask-your-agent` | `the-relay` | When vibe hands the job back | vibe has no model inside it. When an operation needs reasoning, such |
| `agent/ask-your-agent` | `edge-cases` | Edge cases and rules | Running `vibe command` with an empty mailbox prints that nothing is pending; |
| `agent/give-your-agent-the-skill` | `root` | Give your agent the vibevm skill | |
| `agent/give-your-agent-the-skill` | `what-happens` | What happens | The agent runs `vibe mcp install --auto --yes`. vibe detects the agents |
| `agent/give-your-agent-the-skill` | `by-hand` | By hand | 1. See the plan first; nothing is written: |
| `agent/give-your-agent-the-skill` | `skills-from-packages` | Skills that packages bring | Packages can declare skills of their own, for any kind of package: |
| `agent/give-your-agent-the-skill` | `servers-from-packages` | Servers that packages bring | A package of the `mcp` kind delivers a server built from its |
| `agent/give-your-agent-the-skill` | `edge-cases` | Edge cases and rules | `vibe mcp upgrade` refreshes existing integrations to the shape shipped by the |
| `agent/how-agents-read-this-manual` | `root` | How an agent reads this manual | |
| `agent/how-agents-read-this-manual` | `the-files` | The machine files |  |
| `agent/how-agents-read-this-manual` | `offline` | Without a network | The same pages live in the machine [store](../glossary/index.xml#store) once `vibe cache add |
| `agent/how-agents-read-this-manual` | `citing` | Citing a place | Every block on a page carries a number, `p12` and so on, |
| `agent/how-agents-read-this-manual` | `procedure` | The procedure the skill teaches | 1. On an error, read the address the message names, then the |
| `agent/how-agents-read-this-manual` | `edge-cases` | Edge cases and rules | Text written for agents is never part of a project's [boot lane](../glossary/index.xml#boot-lane); |
| `architecture/how-vibe-is-built` | `root` | How vibe is built | |
| `architecture/how-vibe-is-built` | `five-layers` | Five layers | Read the product bottom-up. *Identity*: a package is a [coordinate](../glossary/index.xml#coordinate) plus a |
| `architecture/how-vibe-is-built` | `the-crates` | The crates | The dependency direction is fixed: a surface calls an orchestrator, an orchestrator |
| `architecture/how-vibe-is-built` | `the-install-path` | The path of an install | 1. Discover the workspace root and read the [manifests](../glossary/index.xml#manifest), the lock file, |
| `architecture/how-vibe-is-built` | `the-seams` | The seams | `GitBackend` isolates git: the production implementation shells out to the system git, |
| `architecture/how-vibe-is-built` | `wire-and-authored` | Wire formats and authored formats | Two kinds of text cross the product's boundary. Machine formats, the JSON |
| `architecture/how-vibe-is-built` | `reading-order` | Where to read next | The specifications are the authority: `PROP-000` for the foundational decisions, `PROP-009` for |
| `architecture/traceability` | `root` | Traceability: specs, code and the map | |
| `architecture/traceability` | `marks` | Marks in the code | A Rust item that implements a rule carries an attribute naming the |
| `architecture/traceability` | `the-map` | The map | `cargo xtask specmap` walks the crates for marks and the specification tree |
| `architecture/traceability` | `asking` | Asking the map | The commands below run in a checkout of vibe itself, whose specifications |
| `architecture/traceability` | `documentation-edges` | Documentation in the map | This manual enters the same map. Every `rule` block on a page |
| `architecture/traceability` | `edge-cases` | Edge cases and rules | Units come in four kinds, and each has a default edge: a |
| `architecture/what-the-lifecycle-epic-delivered` | `root` | What the lifecycle epic delivered | |
| `architecture/what-the-lifecycle-epic-delivered` | `the-route` | The route, in eight stages | The work ran as eight numbered stages, each landed as atomic commits |
| `architecture/what-the-lifecycle-epic-delivered` | `decisions` | Decisions that hold | One nine-phase line: dependency materialisation is `install`, placement outside the project is |
| `architecture/what-the-lifecycle-epic-delivered` | `boundaries` | Compatibility boundaries and migrations | The dependency slot record and the [lifecycle](../glossary/index.xml#lifecycle) state are strict, versioned machine |
| `architecture/what-the-lifecycle-epic-delivered` | `deferred` | Deliberately left for later | Deploy targets beyond the first genres, a WebAssembly extension tier, per-language and |
| `architecture/what-the-lifecycle-epic-delivered` | `retired` | Retired lanes, kept as history | The campaign also ran execution lanes that are no longer current: a |
| `architecture/what-the-lifecycle-epic-delivered` | `edge-cases` | Where the evidence lives | The stage-by-stage ledger with commit hashes is `campaigns/packages-2026-09/LIFECYCLE-EXTENSIONS-IMPLEMENTATION-LEDGER.md` in the repository; the |
| `authoring/ship-tools-and-mcp-servers` | `root` | Ship tools and MCP servers | |
| `authoring/ship-tools-and-mcp-servers` | `what-happens` | What happens | The agent scaffolds the package slot with `vibe init package`, puts the |
| `authoring/ship-tools-and-mcp-servers` | `code-in-a-package` | Code in a package | A package is a project made installable, so it may carry arbitrary |
| `authoring/ship-tools-and-mcp-servers` | `binaries` | Binaries | Each tool is one `[[binary]]` entry: a `name`, unique in the package, |
| `authoring/ship-tools-and-mcp-servers` | `servers` | MCP servers | A package of kind `mcp` delivers a server an agent talks to: |
| `authoring/ship-tools-and-mcp-servers` | `edge-cases` | Edge cases and rules | A tool's artifact belongs to the exact version installed; after an update |
| `authoring/specs-agents-can-cite` | `root` | Write specs an agent can cite | |
| `authoring/specs-agents-can-cite` | `two-processes` | Why an address | A person and an agent share one repository and nothing else: no |
| `authoring/specs-agents-can-cite` | `the-address` | The address | `spec://<group>/<name>[@<version>]/<path>/<document>#<anchor>`: the package [coordinate](../glossary/index.xml#coordinate), an optional version, the document's path inside `vibevm/vibespecs/` |
| `authoring/specs-agents-can-cite` | `the-unit` | The unit | A unit is one anchored heading and the text under it, up |
| `authoring/specs-agents-can-cite` | `the-dialect` | Two serialisations, one model | A specification is written in Markdown or in the project's XML dialect, |
| `authoring/specs-agents-can-cite` | `immutable` | An address never moves | Once published, an anchor is immutable. Renaming a section or a rule |
| `authoring/specs-agents-can-cite` | `edge-cases` | Edge cases and rules | A generated file is never a citation target: cite the source document, |
| `authoring/translate-documentation` | `root` | Translate documentation | |
| `authoring/translate-documentation` | `what-happens` | What happens | The agent copies the source's page tree and translates the prose of |
| `authoring/translate-documentation` | `the-manifest` | The manifest | The language of the package is the existing `[i18n] canonical` field, a |
| `authoring/translate-documentation` | `the-rules` | The rules of a translation | A translation mirrors blocks, not sentences: within a block the translator writes |
| `authoring/translate-documentation` | `staleness` | When the source moves | A translation records no revision or hash of the source. When the |
| `authoring/translate-documentation` | `edge-cases` | Edge cases and rules | Sidecar files inside the source package, `README.ru.md` beside `README.md`, are how specifications |
| `authoring/write-a-feat-or-stack` | `root` | Write a feat or a stack | |
| `authoring/write-a-feat-or-stack` | `what-happens` | What happens | The agent creates both slots with `vibe init package`, sets their kinds |
| `authoring/write-a-feat-or-stack` | `a-feat` | A feat | A feat describes what a feature does for its user, in terms |
| `authoring/write-a-feat-or-stack` | `a-stack` | A stack | A stack is a technology context: it says how the abstract abilities |
| `authoring/write-a-feat-or-stack` | `capabilities` | Capabilities | A capability is an abstract interface: a namespace, a colon, a name, |
| `authoring/write-a-feat-or-stack` | `edge-cases` | Edge cases and rules | The word `stack` also names a family bundle: a package of kind |
| `authoring/write-a-flow` | `root` | Write a flow package | |
| `authoring/write-a-flow` | `what-happens` | What happens | The agent runs `vibe init package org.acme/review-notes`, which adds a package slot |
| `authoring/write-a-flow` | `by-hand` | By hand | 1. Create the package slot: |
| `authoring/write-a-flow` | `the-snippet` | The snippet is the expensive part | A snippet is paid for on every session start by every consumer. |
| `authoring/write-a-flow` | `layout` | What goes where | Paths are relative to the package root, the slot `vibevm/vibepacks/org.acme/review-notes/v0.1.0/`. |
| `authoring/write-a-flow` | `edge-cases` | Edge cases and rules | Cross-references inside the package are addresses, `spec://org.acme/review-notes/flows/review-notes/PROTOCOL#anchor`, never relative file paths; they |
| `authoring/write-a-lang-package` | `root` | Write a lang package | |
| `authoring/write-a-lang-package` | `what-happens` | What happens | The agent creates the package skeleton, writes the guide and its snippet |
| `authoring/write-a-lang-package` | `what-a-lang-is` | What a lang package is, and is not | The genre is wider than programming languages: a guide to a query |
| `authoring/write-a-lang-package` | `families` | When a language brings tools | A language guide that also ships tools, a checker, a formatter, a |
| `authoring/write-a-lang-package` | `by-hand` | By hand | 1. Create the package slot, then set `kind = "lang"` in its |
| `authoring/write-a-lang-package` | `edge-cases` | Edge cases and rules | What a language brought that can be run is answered by `vibe |
| `authoring/write-documentation` | `root` | Write documentation for a package | |
| `authoring/write-documentation` | `what-happens` | What happens | The agent creates the package with the `doc` kind, fills the card, |
| `authoring/write-documentation` | `the-manifest` | The manifest | A `doc` package must name at least one subject and carry a |
| `authoring/write-documentation` | `the-vocabulary` | The vocabulary of a page | Pages are written in the project's XML dialect, extended for documentation with |
| `authoring/write-documentation` | `the-shape` | The shape of a page | A concept page has a noun as its title, opens with a |
| `authoring/write-documentation` | `checks` | Checking and publishing | `vibe doc check --examples --citations --derived --media --style` runs every check; the |
| `authoring/write-documentation` | `edge-cases` | Edge cases and rules | Any group may document any package; the site shows such a manual |
| `diagnostics/errors` | `root` | From an error message to the rule | |
| `diagnostics/errors` | `how-to-read` | How to read an error | An error has three parts: what vibe refused, why, and where the |
| `diagnostics/errors` | `install-errors` | Installing |  |
| `diagnostics/errors` | `resolution-errors` | Resolving |  |
| `diagnostics/errors` | `git-errors` | Fetching |  |
| `diagnostics/errors` | `publish-errors` | Publishing |  |
| `diagnostics/errors` | `lifecycle-errors` | Running the lifecycle |  |
| `diagnostics/errors` | `edge-cases` | When the message is not vibe's | A message you do not find here is probably from a layer |
| `faq/index` | `root` | Questions | |
| `faq/index` | `q-commit-vibedeps` | Do I commit the dependency tree? | Yes. `vibevm/vibedeps/` is committed on purpose, so an agent that clones the |
| `faq/index` | `q-conflict` | A dependency deep in my tree clashes on a version. How do I fix it? | First read the resolver's explanation: it names the two constraints that disagree, |
| `faq/index` | `q-confirm` | Why does install ask me to confirm? | Because an install writes into your repository: the dependency tree, the [lock |
| `faq/index` | `q-llm` | Does vibe call a language model? | Not on its own. vibe has no model inside it and every |
| `faq/index` | `q-two-versions` | Can two versions of one package coexist in a project? | No. Resolution picks one version per package across the workspace, and the |
| `faq/index` | `q-registry-gone` | A registry I used has disappeared. Is my project stuck? | Not while the machine [store](../glossary/index.xml#store) holds the packages: a stored version is |
| `faq/index` | `q-pin` | How do I pin a package to an exact version? | Install it with `@=1.2.0`, or pass `--exact` to `vibe install` or `vibe |
| `faq/index` | `q-edit-lock` | Can I edit vibe.lock by hand to silence a hash error? | No. A [fingerprint](../glossary/index.xml#fingerprint) mismatch means the bytes served are not the bytes |
| `faq/index` | `q-why-eight-kinds` | Why so many package kinds? | Because a tool that knows what a package is for before opening |
| `faq/index` | `q-offline` | I am on a plane. What works? | Everything that reads the store and the project: install from the store |
| `faq/index` | `q-remove-vibe` | The project is done. How do I ship it without vibe? | `vibe scrape` removes the vibe layer by contract and proves the native |
| `faq/index` | `q-docs-install` | Why can I not install the manual into my project? | Documentation is read, not executed, and it must not enter the lane |
| `glossary/index` | `root` | Glossary | |
| `glossary/index` | `adaptation` | adaptation | A version of documentation in another language: a separate package that mirrors |
| `glossary/index` | `agent-session` | agent session | One run of a coding agent in a project, from the moment |
| `glossary/index` | `anchor` | anchor | The identifier of a section or a fact inside a specification, the |
| `glossary/index` | `block-number` | block number | The ordinal `pNN` every block of a documentation page receives at build |
| `glossary/index` | `boot-lane` | boot lane | The ordered reading list an agent follows at the start of a |
| `glossary/index` | `boot-snippet` | boot snippet | A package's contribution to the boot lane: one short text declared in |
| `glossary/index` | `capability` | capability | An abstract ability a package provides or requires, written `namespace:name` with an |
| `glossary/index` | `community` | community documentation | Documentation of a package that declares its subject but is not named |
| `glossary/index` | `companion` | companion | A package tied to another by name for the default case of |
| `glossary/index` | `contribution` | contribution | A binding of a handler to an extension point, declared as an |
| `glossary/index` | `coordinate` | coordinate | The name of a package: a group, a slash and a name, |
| `glossary/index` | `deploy-profile` | deploy profile | A named, ordered list of targets and the providers that apply packaged |
| `glossary/index` | `embedded-registry` | embedded registry | The in-tree packages of a source-built vibe, consulted automatically as a registry: |
| `glossary/index` | `extension-point` | extension point | A named place in the lifecycle a contribution binds to: a phase, |
| `glossary/index` | `section` | fact | One anchored unit of a specification with a status: a rule, a |
| `glossary/index` | `family` | family | A set of packages sharing a name stem that move in unison: |
| `glossary/index` | `feature` | feature | An optional, additive content set of a package, declared in `[features]` and |
| `glossary/index` | `fingerprint` | fingerprint | The hash of a package's shippable tree, the identity half of a |
| `glossary/index` | `freshness-fingerprint` | freshness fingerprint | The hash of a phase run's declared inputs, recorded under `.vibe/`; a |
| `glossary/index` | `git-source` | git source | A dependency declared by a git repository and a tag, commit or |
| `glossary/index` | `handler` | handler | What a contribution runs: a built-in, a script, a binary, a native |
| `glossary/index` | `hook` | hook | A package's `pre-install` or `post-install` script, run in the package's slot; installing |
| `glossary/index` | `index-registry` | index (of a registry) | A repository beside a registry's packages that records every published version's summary |
| `glossary/index` | `kind` | kind | What a package is for, one of eight: `flow`, `feat`, `stack`, `tool`, |
| `glossary/index` | `lifecycle` | lifecycle | The fixed order of build steps vibe runs: the one-phase `clean` lifecycle |
| `glossary/index` | `link-type` | link type | How a dependency's boot snippet enters a consumer's lane: `static`, compiled into |
| `glossary/index` | `lock-file` | lock file | `vibe.lock`: the recorded resolution, one per workspace, with every package's exact version, |
| `glossary/index` | `managed-block` | managed block | The region between the lines `<vibevm>` and `</vibevm>` at the end of |
| `glossary/index` | `manifest` | manifest | `vibe.toml`: the one file you write to describe a project, a package |
| `glossary/index` | `mcp-server` | MCP server | A program an agent talks to over the Model Context Protocol. vibe's |
| `glossary/index` | `mirror` | mirror | An alternative address for the same registry, tried for availability and verified |
| `glossary/index` | `official` | official documentation | Documentation whose edges converge: the subject names the package in `[documentation]` or |
| `glossary/index` | `override` | override | A replacement source for one coordinate that short-circuits the registries; marked in |
| `glossary/index` | `package` | package | The unit vibe installs: a folder with a manifest and the text |
| `glossary/index` | `phase` | phase | One step of a lifecycle: `validate`, `install`, `generate`, `build`, `test`, `create`, `verify`, |
| `glossary/index` | `project` | project | Any folder with a `vibe.toml`; a consumer of packages, marked by a |
| `glossary/index` | `provider` | provider | What answers how a mechanism runs: a built-in or a package-supplied implementation |
| `glossary/index` | `receipt` | receipt | The record a deploy writes for every resource it created: what, where, |
| `glossary/index` | `registry` | registry | A hosting organisation where packages are published, one repository per package; a |
| `glossary/index` | `relay` | relay | The mailbox under `.vibe/agentic/` where vibe parks an instruction it cannot execute |
| `glossary/index` | `scrape` | scrape | The terminal removal of the vibe layer from a project, by contract, |
| `glossary/index` | `skill` | skill | A file a package declares in `[[skill]]` that teaches an agent something; |
| `glossary/index` | `specification` | specification | The normative text of a package or a project under `vibevm/vibespecs/`: addressable |
| `glossary/index` | `store` | store | The machine-wide cache of fetched package versions under `~/.vibe/cache/`, keyed by identity, |
| `glossary/index` | `subject` | subject | A package that a documentation package documents, named in its `[[documents]]` table |
| `glossary/index` | `subskill` | subskill | A selectable cut of a package's content, activated by the consumer's context, |
| `glossary/index` | `traceability-map` | traceability map | `specmap.json`: the generated graph of specification units, tagged code items and the |
| `glossary/index` | `translation` | translation | A separate documentation package in another language that names its source in |
| `glossary/index` | `version-constraint` | version constraint | What a manifest asks for: a range such as `^1.0`, an exact |
| `glossary/index` | `workspace` | workspace | A repository developing several packages together, declared by a `[workspace]` table listing |
| `howto/install-a-package` | `root` | Install a package | |
| `howto/install-a-package` | `what-happens` | What happens | The agent runs `vibe install org.vibevm.world/wal`. vibe walks the project's [registries](../glossary/index.xml#registry) in |
| `howto/install-a-package` | `by-hand` | By hand | 1. Install by [coordinate](../glossary/index.xml#coordinate). Add `@` and a constraint to ask for |
| `howto/install-a-package` | `constraints` | Asking for a version | `vibe install org.vibevm.world/wal@^1.0` accepts any 1.x; `@=1.0.0` accepts exactly one; `--exact` writes |
| `howto/install-a-package` | `after-a-clone` | After cloning a project | `vibe install` with no package names installs what the manifest already requires, |
| `howto/install-a-package` | `edge-cases` | Edge cases and rules | Installing a package by name re-resolves the whole graph, but every dependency |
| `howto/publish-a-package` | `root` | Publish a package | |
| `howto/publish-a-package` | `what-happens` | What happens | The agent runs `vibe registry publish vibevm/vibepacks/org.acme/notes/v0.1.0 --dry-run` first and shows you |
| `howto/publish-a-package` | `by-hand` | By hand | 1. Put a publish token where vibe reads it. That is the |
| `howto/publish-a-package` | `versions` | Versions never move | A published version is immutable: a tag that already exists is refused, |
| `howto/publish-a-package` | `workspaces` | Several packages at once | A repository that develops several packages publishes them with `vibe workspace publish`. |
| `howto/publish-a-package` | `edge-cases` | Edge cases and rules | `--repo-url` pushes straight to an existing git repository with your local git |
| `howto/read-documentation-locally` | `root` | Read documentation locally | |
| `howto/read-documentation-locally` | `what-happens` | What happens | The agent runs `vibe cache add org.vibevm.core/vibevm-docs`. A documentation package is never |
| `howto/read-documentation-locally` | `by-hand` | By hand | 1. Warm the store. Inside a project, its [registries](../glossary/index.xml#registry) are the source; |
| `howto/read-documentation-locally` | `private-packages` | Private packages | The same reader shows the documentation of packages that live in a |
| `howto/read-documentation-locally` | `the-shell` | The reader's shell | A released `vibe` carries the reader's interface inside the binary. A `vibe` |
| `howto/read-documentation-locally` | `edge-cases` | Edge cases and rules | Warming a documentation package warms its subjects too, so the rules a |
| `howto/remove-a-package` | `root` | Remove a package | |
| `howto/remove-a-package` | `what-happens` | What happens | The agent runs `vibe uninstall org.vibevm.world/wal`. vibe shows what will leave: the |
| `howto/remove-a-package` | `by-hand` | By hand | 1. Remove by [coordinate](../glossary/index.xml#coordinate); the version is not needed: |
| `howto/remove-a-package` | `derived-state` | Removing derived state without removing packages | Sometimes you want a clean slate rather than a smaller graph: before |
| `howto/remove-a-package` | `edge-cases` | Edge cases and rules | Uninstalling a package that another installed package requires leaves the shared dependency |
| `howto/set-up-a-workspace` | `root` | Set up a workspace | |
| `howto/set-up-a-workspace` | `what-happens` | What happens | The agent adds a `[workspace]` table to the root [manifest](../glossary/index.xml#manifest) naming the |
| `howto/set-up-a-workspace` | `by-hand` | By hand | 1. In the root `vibe.toml`, declare the members; globs are allowed: |
| `howto/set-up-a-workspace` | `one-manifest` | One manifest, three roles | Every node has a file named `vibe.toml`, and what the file contains |
| `howto/set-up-a-workspace` | `members-referring` | Members referring to each other | A member requires a sibling by path rather than by [registry](../glossary/index.xml#registry), with |
| `howto/set-up-a-workspace` | `edge-cases` | Edge cases and rules | Workspaces nest: a member may itself carry a `[workspace]` table. Nesting groups |
| `howto/update-packages` | `root` | Update packages | |
| `howto/update-packages` | `what-happens` | What happens | The agent first runs `vibe outdated`, which compares every pin in the |
| `howto/update-packages` | `by-hand` | By hand | 1. See what is behind: |
| `howto/update-packages` | `the-constraint` | The constraint stays where you put it | An update moves the pin in the lock file within the constraint |
| `howto/update-packages` | `recovery` | Recovering after a breaking update | If an update leaves the project in a state that no longer |
| `howto/update-packages` | `edge-cases` | Edge cases and rules | `vibe outdated` reads the registries declared in the project's manifest, and for |
| `howto/use-a-private-registry` | `root` | Use a private registry | |
| `howto/use-a-private-registry` | `what-happens` | What happens | The agent runs `vibe registry add acme git@github.com:acme-specs --auth ssh --position primary`, |
| `howto/use-a-private-registry` | `by-hand` | By hand | 1. Add the registry to the project. The address is the organisation |
| `howto/use-a-private-registry` | `machine-wide` | For every project on a machine | Put the same registry block into `~/.vibe/registry.toml`. It is merged after each |
| `howto/use-a-private-registry` | `redirects` | Delegating a package to another registry | A registry may point at a package that lives elsewhere. Instead of |
| `howto/use-a-private-registry` | `tokens` | Tokens | A token never lands in a file vibe writes and never appears |
| `howto/use-a-private-registry` | `edge-cases` | Edge cases and rules | A private registry without an [index](../glossary/index.xml#index-registry) still works; searches skip it and |
| `howto/work-offline` | `root` | Work offline | |
| `howto/work-offline` | `what-happens` | What happens | The agent runs `vibe cache add` for the extra package, which resolves |
| `howto/work-offline` | `by-hand` | By hand | 1. Warm the store with a package and its dependencies. Inside a |
| `howto/work-offline` | `switching-it-on` | Three ways to switch offline mode on | The flag `--offline` on any command; the environment variable `VIBE_OFFLINE=1`; or the |
| `howto/work-offline` | `air-gapped` | A whole team without a network | For a machine that never sees the registry, `vibe registry vendor` writes |
| `howto/work-offline` | `edge-cases` | Edge cases and rules | If an offline install refuses a package the store already holds, the |
| `lifecycle/build-package-deploy` | `root` | Build, package and deploy a project | |
| `lifecycle/build-package-deploy` | `what-happens` | What happens | `vibe deploy --plan` runs the whole default [lifecycle](../glossary/index.xml#lifecycle) in planning mode and |
| `lifecycle/build-package-deploy` | `by-hand` | By hand | 1. Declare what to build, what to deploy and where. A target |
| `lifecycle/build-package-deploy` | `artifacts` | Artifacts and targets | The [manifest](../glossary/index.xml#manifest) declares what `build` produces and what `package` assembles, as artifact |
| `lifecycle/build-package-deploy` | `edge-cases` | Edge cases and rules | Two deploys of the same profile do not race: the engine takes |
| `lifecycle/extensions-and-providers` | `root` | Extensions and providers | |
| `lifecycle/extensions-and-providers` | `points-and-contributions` | Points and contributions | The [lifecycle](../glossary/index.xml#lifecycle) exposes named *[extension points](../glossary/index.xml#extension-point)*, strings of the form `family:name`. The |
| `lifecycle/extensions-and-providers` | `handlers` | Five kinds of handler | Every handler receives one context envelope, a versioned JSON document that names |
| `lifecycle/extensions-and-providers` | `activation` | Switching contributions on | Installing a package is the consent to run its contributions: a dependency's |
| `lifecycle/extensions-and-providers` | `seeing` | Seeing what ran | `vibe extensions` lists every declared contribution in the installed world with its |
| `lifecycle/extensions-and-providers` | `providers` | Providers | A contribution of kind `agent` needs someone to run its prompt. Under |
| `lifecycle/extensions-and-providers` | `edge-cases` | Edge cases and rules | A contribution may carry a selector that limits it to matching files |
| `lifecycle/phases` | `root` | The lifecycle: from validate to deploy | |
| `lifecycle/phases` | `the-phases` | The nine phases | vibe has two [lifecycles](../glossary/index.xml#lifecycle). `clean` has one phase and removes derived state. |
| `lifecycle/phases` | `fresh` | Nothing runs twice for nothing | Every phase run records a [fingerprint](../glossary/index.xml#fingerprint) of the inputs it declared; the |
| `lifecycle/phases` | `the-plan` | Seeing before doing | `--plan` reports what a phase run would do and changes nothing; every |
| `lifecycle/phases` | `where-steps-come-from` | Where the steps come from | The phases are fixed; what runs inside them is contributed by packages. |
| `lifecycle/phases` | `edge-cases` | Edge cases and rules | A failing step stops the chain; the phases before it keep their |
| `lifecycle/scrape` | `root` | Remove VibeVM from a project | |
| `lifecycle/scrape` | `what-happens` | What happens | The agent creates the contract with `vibe scrape contract init` if the |
| `lifecycle/scrape` | `by-hand` | By hand | 1. Create the contract. The default is conservative: it names vibe's trees, |
| `lifecycle/scrape` | `the-contract` | The contract | What counts as vibe's and what counts as yours is not guessed |
| `lifecycle/scrape` | `edge-cases` | Edge cases and rules | Scrape is not clean. `vibe clean` removes what vibe can regenerate and |
| `model/boot-lane` | `root` | The boot lane: how an agent reads a project | |
| `model/boot-lane` | `the-order` | The order of reading | An [agent session](../glossary/index.xml#agent-session) begins with the instruction file its vendor reads, `CLAUDE.md`, |
| `model/boot-lane` | `where-it-comes-from` | Where the list comes from | Every package may contribute one [boot snippet](../glossary/index.xml#boot-snippet): a short text meant to |
| `model/boot-lane` | `cost` | Why it is short | Every word in the lane is paid for on every session start, |
| `model/boot-lane` | `edge-cases` | Edge cases and rules | A snippet that declares a condition is always a dynamic entry, whatever |
| `model/lock-and-store` | `root` | The lock file and the machine store | |
| `model/lock-and-store` | `identity` | Identity is the content, not the address | A package version is identified by four things: its group, its name, |
| `model/lock-and-store` | `the-lock-file` | The lock file | `vibe.lock` lists every package in the resolved graph, direct and transitive, with |
| `model/lock-and-store` | `the-store` | The machine store | Every package vibe fetches, for any project, lands in one [store](../glossary/index.xml#store) under |
| `model/lock-and-store` | `offline` | Offline | With `--offline`, or `VIBE_OFFLINE=1` in the environment, vibe touches no network at |
| `model/lock-and-store` | `edge-cases` | Edge cases and rules | The settings folder, including the store, is `~/.vibe/` on every platform; the |
| `model/packages-and-kinds` | `root` | Packages and their kinds | |
| `model/packages-and-kinds` | `a-package` | What a package is | A package is a project made installable. It has the same layout |
| `model/packages-and-kinds` | `the-kinds` | The eight kinds | The set is closed and grows only by an amendment to the |
| `model/packages-and-kinds` | `families` | Families and companions | Some [capabilities](../glossary/index.xml#capability) arrive as several packages that share a name stem: the |
| `model/packages-and-kinds` | `edge-cases` | Edge cases and rules | Changing a package's group or name creates a new package, not a |
| `model/registries` | `root` | Registries and the index | |
| `model/registries` | `what-a-registry-is` | What a registry is | A [registry](../glossary/index.xml#registry) is not a server vibe runs. It is a hosting |
| `model/registries` | `the-index` | The index | Cloning a repository to learn what is in it is slow, and |
| `model/registries` | `mirrors-and-overrides` | Mirrors, overrides and git sources | A *mirror* is another address for the same registry, tried first for |
| `model/registries` | `authentication` | Authentication | A public registry needs no credentials, and vibe sends none: it silences |
| `model/registries` | `edge-cases` | Edge cases and rules | A vibe built from a source checkout treats that checkout's in-tree packages |
| `model/two-trees` | `root` | Two trees: what you write and what vibe writes | |
| `model/two-trees` | `the-rule` | The founding rule | Think of how a C++ program uses a library: you write `#include`, |
| `model/two-trees` | `why-commit` | Why the copies are committed | The copied tree is committed to your repository, which surprises people who |
| `model/two-trees` | `what-regenerates` | What regenerates, and when | Three things in a project are derived from the [manifest](../glossary/index.xml#manifest) and the |
| `model/two-trees` | `edge-cases` | Edge cases and rules | A package's folder in the dependency tree is named by its group, |
| `model/versions` | `root` | Versions and updates | |
| `model/versions` | `asking` | Asking for a version | Package versions follow semantic versioning: three numbers, where the first changes when |
| `model/versions` | `moving` | Moving the pin | `vibe outdated` reads the lock file and the [registry](../glossary/index.xml#registry) and lists the |
| `model/versions` | `what-a-version-promises` | What a version promises | A version number is a contract: version 1 does what version 1 |
| `model/versions` | `vibe-itself` | Versions of vibe itself | The program manages its own versions with `vibe self`: `self install` builds |
| `model/versions` | `edge-cases` | Edge cases and rules | Two packages with the same coordinate and version but different bytes are |
| `reference/commands` | `root` | Command reference | |
| `reference/commands` | `global` | Global options | Every command takes `--json` for machine-readable output, `--quiet` for a one-line summary, |
| `reference/commands` | `projects` | Projects and packages |  |
| `reference/commands` | `registries` | Registries, search and the store |  |
| `reference/commands` | `lifecycle` | The lifecycle |  |
| `reference/commands` | `agents` | Agents |  |
| `reference/commands` | `specs` | Specifications and traceability |  |
| `reference/commands` | `machine` | The machine and vibe itself |  |
| `reference/commands` | `documentation` | Documentation |  |
| `reference/lock-file` | `root` | The lock file: vibe.lock | |
| `reference/lock-file` | `where` | Where it lives and who writes it | There is one `vibe.lock` per workspace, at the absolute root, beside the |
| `reference/lock-file` | `meta` | [meta] |  |
| `reference/lock-file` | `package-entries` | [[package]] |  |
| `reference/lock-file` | `reading-a-diff` | Reading a diff | A changed `version` with a changed `content_hash` is an update. A changed |
| `reference/lock-file` | `edge-cases` | Edge cases and rules | An unchanged manifest against an unchanged [lock file](../glossary/index.xml#lock-file) makes `vibe install` skip |
| `reference/machine-formats` | `root` | Machine formats and JSON reports | |
| `reference/machine-formats` | `the-rule` | One rule for every format | Anything a foreign parser reads, a report a script parses, a file |
| `reference/machine-formats` | `the-envelope` | The envelope | A `--json` document is one JSON object per command, or a stream |
| `reference/machine-formats` | `the-documents` | The documents | The fields of every schema in the table, generated from the schema |
| `reference/machine-formats` | `edge-cases` | Edge cases and rules | A document that no schema describes is a defect, not a feature; |
| `reference/manifest` | `root` | The manifest: vibe.toml | |
| `reference/manifest` | `one-file` | One file, three roles | Every node, whether a consumer project, a publishable package or a workspace |
| `reference/manifest` | `package-table` | [package] | `[project]` carries the same descriptive fields for a consumer, without a version |
| `reference/manifest` | `requirements` | [requires] and its neighbours |  |
| `reference/manifest` | `sources` | Where packages come from |  |
| `reference/manifest` | `deliveries` | What a package delivers |  |
| `reference/manifest` | `documentation-tables` | Documentation and its subjects |  |
| `reference/manifest` | `example-manifest` | A complete example | The manifest of this manual, generated from the package itself, shows a |
| `reference/settings-and-environment` | `root` | Settings, paths and environment | |
| `reference/settings-and-environment` | `the-folder` | The folder: ~/.vibe/ | On Windows the folder is `%USERPROFILE%\.vibe\`. The older location under `~/.vibevm/` is |
| `reference/settings-and-environment` | `inside-a-project` | Inside a project | Preferences merge layer by layer: a scalar from a higher layer replaces, |
| `reference/settings-and-environment` | `variables` | Environment variables |  |
| `reference/settings-and-environment` | `precedence` | Precedence | For the same setting, a flag on the command line wins over |
| `reference/settings-and-environment` | `edge-cases` | Edge cases and rules | Token files are surface secrets: restrict them to your user, never commit |
| `start/first-project` | `root` | Create your first project | |
| `start/first-project` | `what-happens` | What happens | The agent runs `vibe init hello-vibe`, which creates the folder with a |
| `start/first-project` | `by-hand` | By hand | 1. Create the project. The name becomes the folder: |
| `start/first-project` | `what-appeared` | What appeared on disk | Open `hello-vibe/vibe.toml`: it names the project and, after the install, lists `org.vibevm.world/wal` |
| `start/first-project` | `edge-cases` | Edge cases and rules | A repeated `vibe install` with no package names installs whatever the manifest |
| `start/index` | `root` | The newcomer's route | |
| `start/index` | `step-1` | Step 1: understand what you are installing | Read [What VibeVM is](what-vibevm-is.xml) first. It takes five minutes and gives you |
| `start/index` | `step-2` | Step 2: install vibe | Follow [Install vibe](install-vibe.xml). On Windows it is an archive and a script; |
| `start/index` | `step-3` | Step 3: create a project and install one package | Follow [Create your first project](first-project.xml). Give the prompt to your agent, or |
| `start/index` | `step-4` | Step 4: look at what appeared | Read [What a project contains](what-a-project-contains.xml) with the new folder open beside it. |
| `start/index` | `step-5` | Step 5: see how the agent reads it | Read [The boot lane](../model/boot-lane.xml). The agent opens the instruction file, reads one |
| `start/index` | `after` | After the route | From here the manual branches. To work day to day, the pages |
| `start/install-vibe` | `root` | Install vibe | |
| `start/install-vibe` | `what-happens` | What happens | On Windows the agent downloads the release archive, unpacks it, and runs |
| `start/install-vibe` | `by-hand` | By hand |  |
| `start/install-vibe` | `where-things-go` | Where things go | vibe keeps everything it owns under one folder in your home directory, |
| `start/install-vibe` | `edge-cases` | Edge cases and rules | If `vibe --version` prints nothing in the terminal where you ran the |
| `start/what-a-project-contains` | `root` | What a project contains | |
| `start/what-a-project-contains` | `the-files` | The files, one by one | Start with the two files at the root, because everything else is |
| `start/what-a-project-contains` | `the-boot-files` | The boot files | The exception in your tree is `vibevm/vibespecs/boot/`. Two files there are yours: |
| `start/what-a-project-contains` | `who-writes-what` | Who writes what | The last row is the project's scratch space: caches and internal state, |
| `start/what-a-project-contains` | `edge-cases` | Edge cases and rules | If you delete `vibevm/vibedeps/` or the generated boot files, `vibe reinstall` rebuilds |
| `start/what-vibevm-is` | `root` | What VibeVM is | |
| `start/what-vibevm-is` | `the-problem` | The problem it solves | Every serious project has rules that never make it into the code: |
| `start/what-vibevm-is` | `how-it-works` | How it works | The unit vibe installs is a *package*: a folder with a short |
| `start/what-vibevm-is` | `what-it-is-not` | What it is not | VibeVM is not an agent and has no model inside it. It |
| `start/what-vibevm-is` | `where-next` | Where to go next | To see it on your own machine, install vibe and create a |
