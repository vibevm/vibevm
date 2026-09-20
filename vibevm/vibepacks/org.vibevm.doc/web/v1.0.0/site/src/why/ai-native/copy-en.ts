/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The English copy of `/why/ai-native`.
 *
 * Every value is the owner's, moved byte for byte from the Astro
 * component that carried it (`src/components/why/WhyAiNative.astro`).
 * One file per edition because that is how the content is worked on:
 * a translator reads one of them, and a reviewer diffs one of them.
 */

import type { Strings } from "./strings.ts";

export const COPY: Strings = {
  eyebrow: "org.vibevm.ai-native · rust · typescript · go",
  badge: "Research beta",
  headlineHtml: "Code that AI agents <em>actually understand</em>.",
  lead: "Every line of code carries intent — and that intent is machine-traceable. The AI-Native Code Discipline is a set of rules and runnable gates for Rust, TypeScript, and Go: code written the way agents will maintain it — easily, safely, cheaply — not the way humans find it comfortable to read. The code itself stays ordinary idiomatic code under the ordinary compiler; the added strictness lives around it, machine-checkable: cells and seams, spec-to-code traceability, deviation testimony, deterministic conformance. Built for the codebase whose next maintainer is often a weaker model than the author.",
  ctaTry: "Try it on a real repo",
  ctaVibevm: "Why VibeVM",
  heroAlt:
    "A suprematist blueprint: one gold core square projects three drafting rays onto three language stations — a solid oxide square for Rust, an outlined slate-blue circle for TypeScript, a translucent teal triangle for Go — each clamped by gold envelope brackets, all standing on one thin verification floor that ends in a single gate notch.",
  heroCaption:
    "Left to right: the neutral core; Rust, TypeScript, Go. The gold brackets are the discipline — the language interior stays ordinary, the envelope gets stricter. Everything stands on one floor with one exit code.",
  /* The essay doorway — the port's own words, not the Astro copy's. */
  visionK: "The essay",
  visionHead: "The Big Vision",
  visionSub:
    "The worldview behind this discipline: two sources of intention, an expensive probabilistic layer over a cheap deterministic one — and traceable edges between them.",

  problemK: "The problem",
  problemH: "Green build, unreadable change",
  problemBody:
    "Your toolchain proves compilation, types, and style. It does not prove what a safe change actually depends on: which module a requirement lives in, where the seams are, whether that comment is still true, which contract an error just broke, what a given diff was allowed to touch. Frontier models already generate good code; the unsolved part is the rest of the lifecycle — a different, often weaker model reading and changing that code later. Humans re-derive the missing architecture on every review. Agents guess it.",
  problemPoints: [
    "The crate compiles and clippy is silent — and the reviewer still cannot tell which invariants the diff endangered.",
    "The architecture lives in a document that stopped being true two refactors ago, and nothing ever compares it to the code.",
    "The error says what failed, never which contract it violated — so every debugging session starts from zero.",
  ],

  lawK: "The central law",
  lawQuoteHtml:
    "Keep the code surface inside the training distribution. Put the strictness into the type system, the contracts, the metadata, and the verification loop — <em>around</em> the code, not in its syntax.",
  lawBody:
    "Models collapse on out-of-distribution surface syntax, and they recover through verification loops and executable helpers — the two dated, third-party results the discipline is built on and cites in its manifesto. So AI-Native Rust is not a dialect: at the token level it reads as ordinary idiomatic Rust, and the same holds for TypeScript and Go. What changes is the envelope — and every rule in it ships a checker, because a rule a model must remember decays, while a rule a machine enforces holds.",
  lawNote:
    "The manifesto tags each of its own claims [E-strong], [E-mid], or [E-hyp] — benchmark-backed, adjacent evidence, or hypothesis the pilot must validate. A discipline that names its failure modes is more trustworthy than one that hides them.",
  envelopeAlt:
    "A source file whose interior is ordinary code lines, wrapped by a dashed envelope ring with four carrier nodes and clamp brackets.",
  envelopeCaption:
    "The interior stays in-distribution. The envelope carries the strictness: types · contracts · metadata · the verification loop.",
  contractHead: "Spec-driven, honestly",
  contractBodyHtml:
    "“True” Spec-Driven Development — where every change regenerates the whole codebase from the specification — is an unreachable dream: nobody has that much money. This discipline works the other way around. The specification sets the contract; the code is its technical realization — and for the low-level detail, <strong>there is no better contract than the code itself</strong>. That is why spec and code are joined by traceable, checkable edges, not by regeneration.",

  archK: "Architecture",
  archH: "One neutral core, projected per language",
  archBody:
    "The language-independent core ships the manifesto, the pattern-card format, the nine-class catalog of executable scaffolds, the operating playbooks, and the neutral engines: conformance, traceability, the inert tag macro. Each language stack projects that core through its own fact extractor and four gate binaries, adds a generation-time type oracle, the language guide, the nine cards in language shape, and two agent skills. Your project keeps the policy — which cells are gated, which are exempt and why. The packages ship engines, never policy.",
  archAlt:
    "Architecture: the neutral core package feeds three language stacks — Rust, TypeScript, Go — each with an extractor, gates, and an oracle; the stacks install into a consumer project that owns its policy files, committed traceability index, and ratchet baseline.",
  archCoreHead: "core-ai-native",
  archCoreTag: "flow · neutral",
  archCoreBody:
    "manifesto · card format · scaffold catalog · playbooks · engines: conform, specmap, specmark",
  archStacks: [
    {
      mark: "rust",
      head: "rust-ai-native-lang",
      body: "syn extractor · oracle over rust-analyzer · guide, 9 cards, 2 skills",
    },
    {
      mark: "ts",
      head: "typescript-ai-native-lang",
      body: "Compiler-API sidecar ts-extract · oracle over your typescript · guide, 9 cards, 2 skills",
    },
    {
      mark: "go",
      head: "go-ai-native-lang",
      body: "stdlib-only go-extract · oracle over gopls · guide, 9 cards, 2 skills",
    },
  ],
  archProjHead: "your repository",
  archProjTag: "policy stays here",
  archProjBody:
    "conform.toml + specmap.toml · committed specmap.json · ratchet baseline · debt & intent registries",
  archNote:
    "Each stack also ships as an MCP server (*-ai-native-mcp) — the same command surface plus the type oracle for any MCP-capable agent; the Rust server alone exposes eighteen tools. Distribution is vibe: one aggregator package per language pins stack, server, and core to one resolved version set.",

  compareK: "The dividing line",
  compareH: "What your toolchain proves — and what the gate adds",
  compareBody:
    "The discipline deletes nothing. Each language’s floor runs your existing tools first, in a fixed order, then appends three gates of its own: conform for structure, specmap for traceability, test-gate for the baseline. One command, one exit code.",
  compareYours: "already yours",
  compareAdded: "the gate adds",
  compareAdds: {
    rust: [
      "Cell isolation: cells import seams and the core, never sibling cells.",
      "No unwrap in domain logic — a panic path needs an error contract or recorded deviation.",
      "Error enums cite the requirement they enforce, in the error text itself.",
      "File budget, and invariants may not drown in a file’s middle third.",
    ],
    ts: [
      "The unsafe-set census: as, any, non-null !, @ts-ignore become findings unless carried by a recorded deviation.",
      "Cell isolation over import specifiers.",
      "Environment reads are legal only in the composition root.",
      "Seam error types cite their spec:// requirement.",
    ],
    go: [
      "Cell isolation at package grain, plus the ban census with deviation testimony.",
      "//spec: directives feed a committed index with a package-grain orphan ratchet.",
      "Closed-set switches ride exhaustive — Go has no sum types, so the gate leans on the ecosystem’s carrier.",
      "test-gate runs xfail-strict over go test -json.",
    ],
  },

  mechK: "Mechanisms",
  mechH: "Five things you can hold in your hands",
  m1H: "Traceability that survives refactors",
  m1Body:
    "Code carries inert tags — at most three edges per item, typed implements | verifies | documents | deviates | informs. The specmap engine compiles them into a committed, deterministic index: spec units on one side, code items on the other. Invalidation is asymmetric by design: bump a spec unit’s revision and its edges turn suspect; edit the code and the edges hold. An item that loses its anchor is an orphan the ratchet refuses to accept. That is the difference between architecture you re-derive and architecture you query.",
  m1BeforeLabel: "before",
  m1AfterLabel: "under the discipline",
  m1Caption:
    "Abridged from the pilot’s own vibe-core. The failure now names the contract it broke and the fix surface — for the reviewer and for the next agent alike; the error-enum-cites-req gate keeps it that way.",
  m1TagsLabel: "The same edge, in each language:",
  m2H: "The floor: one command, one exit code",
  m2Body:
    "rust-ai-native floor, typescript-ai-native floor, go-ai-native floor. Formatters, compilers, linters, and tests run first; the discipline’s gates run after, in the same panel. Every policy line prints its origin, and a Defaulted policy announces itself — never trust a green you didn’t configure. Conformance is deterministic: same inputs, byte-identical SARIF, no model anywhere in the checking path.",
  m3H: "The ratchet, or how a real repo adopts this",
  m3Body:
    "init writes the pre-adoption posture: every crate or cell exempt, each with a recorded reason; the only hard precondition is that the workspace compiles. From there the direction is one-way — drain a unit to zero findings, flip it into the gated list, and a flip must never widen the baseline. Pre-existing findings freeze in conform-baseline.json; new ones fail the gate; the baseline only shrinks. The test gate is xfail-strict: it fails on a new failure and on an unexpected pass nobody promoted, so the baseline shrinks truthfully.",
  m3Caption:
    "vibevm’s own conform-baseline.json today, in full: every gated crate entered at zero findings.",
  m4H: "Bans carry escape hatches",
  m4Body:
    "Forbidden-by-default constructs stay legal with machinery and a recorded reason. A matching deviation downgrades the finding to deviation-acknowledged — visible, counted, reviewable — instead of silencing it. The discipline states its own symmetry: a ban with no escape hatch is a bug in the discipline; a deviation with no reason is a bug in the code.",
  m4Caption:
    "A real deviation shape from the pilot: the reason travels with the code, and the gate reads it.",
  m5H: "A type oracle at generation time",
  m5Body:
    "Before writing a nontrivial edit, an agent can validate the hypothetical file content — unsaved, over in-memory overlays — against the language’s own analyzer: rust-analyzer, your project’s typescript, gopls. The oracle merges the gate’s conform rules into its answers (validate / scope / complete / type), so a discipline violation surfaces before the diff exists. The honesty is in the brief: an analyzer is not the compiler; a clean validate shortens the distance to green — the floor stays the truth.",
  m5Caption:
    "Also served over MCP as tcg_validate / tcg_scope / tcg_complete / tcg_type.",

  familyK: "The family",
  familyH: "Three projections, one bar",
  familyBody:
    "Parity is a law of the discipline, not a hope: no language projection may enforce the discipline more weakly than another without a recorded reason. Where a language genuinely lacks an idiom’s analogue, the gap is recorded — with the reason as the escape hatch, never in silence.",
  family: [
    {
      mark: "rust",
      name: "AI-Native Rust",
      tag: "the pilot",
      body: "The borrow checker is already a verifier, so the projection leans on it: typestate and newtypes at seams, panic-free domain logic, compiled doctests per public seam. Furthest along — the reference bar the other projections are measured against.",
    },
    {
      mark: "ts",
      name: "AI-Native TypeScript",
      tag: "second projection",
      body: "TypeScript adds a hazard Rust does not have: a type system you can defeat in two words. So the gate takes the escape hatches themselves under census — as, any, !, @ts-ignore — and untyped data enters as unknown, narrowed by runtime validators at the erasure boundary. In exchange: the most mature codemod and Compiler-API tooling of any mainstream language.",
    },
    {
      mark: "go",
      name: "AI-Native Go",
      tag: "third, newest",
      body: "No sum types, so closed-set exhaustiveness rides the ecosystem’s own carrier beside gofmt, vet, and staticcheck. The fact extractor is pure Go stdlib — the only external process on the critical path is the language’s official tooling. Worked pilot: a miniature reconciler with the whole chain green.",
    },
  ],
  familyNote:
    "Beyond these three: C++ and Python exist in the engine spec as rows explicitly marked «specified, not built». They become stacks when a product decision says so — not silently.",

  dogK: "Dogfood",
  dogH: "Piloted on the code that ships it",
  dogBody:
    "The discipline is a product in its own right, and VibeVM is its first pilot — not its scope boundary. The vibe CLI, a Rust workspace of a few dozen crates, runs the gates on itself: most crates gated, the rest exempt with recorded reasons, and the ratchet baseline is empty today. Its committed traceability index carries thousands of spec units and edges. In the same family, Zap’s Rust engine and its TypeScript client run the same gates — the client alone gates dozens of cells across a six-figure line count. The lock file pins the exact discipline version the pilot runs, and the adoption campaign’s paper trail — baseline, log, report — is in the tree.",
  dogPoints: [
    "The sweep idioms in the Rust guide — tests-out splits, restructure-beats-testify, flip-only-after-drain — are distilled from the pilot’s own campaigns, each with its gotchas recorded.",
    "The spec corpus audits itself: claims that outran the code are annotated «specified, not built» in place, down to file and line. The traceability machinery is what makes that check possible at all.",
    "What we do not claim: no performance numbers, no adoption statistics, no measured productivity delta. The discipline’s evidence base is cited and tagged in the manifesto; its own pilot metrics are still ahead.",
  ],

  statusK: "Status, plainly",
  statusH: "A research beta that names its failure modes",
  statusCols: [
    {
      head: "Verified today",
      items: [
        "The neutral engines, the three language stacks, and the MCP servers ship and run.",
        "The gates hold on three real codebases — the vibe CLI, Zap’s engine, Zap’s client.",
        "Worked demo projects for all three languages sit in the pilot’s tree; the Go demo documents its whole chain green.",
        "The brownfield registries — debt with sunsets, xfail-strict test baselines, intent with a carry-over guarantee — are in use on the pilot.",
      ],
    },
    {
      head: "Being explored",
      items: [
        "Transfer is the central open question: the executable-scaffold evidence is about generation; whether the advantage carries to modification is exactly what the pilot must falsify.",
        "Card delivery is a boot instruction a session follows — not yet a computed activation match.",
        "Deeper semantic rules wait, by recorded ruling, for the second type-requiring check.",
      ],
    },
    {
      head: "Not promised",
      items: [
        "Stability. This is a beta: rules, names, and the set of checks will change.",
        "Languages beyond Rust, TypeScript, and Go.",
        "Benchmarks of our own — every card carries a falsifiable prediction in place of a measurement, and reading the maturity tags is part of using the discipline.",
      ],
    },
  ],
  statusTail:
    "The invitation is to try the discipline, read its manifesto, and argue with it — not to believe a stability promise.",

  startK: "Start",
  startH: "Adopt it the way the protocol says: gradually",
  steps: [
    {
      kind: "cmd",
      cmd: "vibe install org.vibevm.ai-native/rust-ai-native",
      body: "One aggregator per language — typescript-ai-native and go-ai-native are its siblings — pins the stack, its MCP server, and the neutral core to one resolved version set.",
      linkText: "Need vibe first? Install VibeVM",
    },
    {
      kind: "cmd",
      cmd: "rust-ai-native init",
      body: "Writes the pre-adoption posture into your repo: conform.toml with every crate exempt-with-a-reason, specmap.toml with external spec resolution, the debt and intent registries. Nothing gates yet — and every tool is already useful at 0% adoption.",
    },
    {
      kind: "cmd",
      cmd: "rust-ai-native floor",
      body: "Your existing tools first, the discipline’s gates after, one exit code. Green means: no regression against the inventory — the honest definition for a repository that is mid-flight.",
    },
    {
      kind: "head",
      head: "Drain one cell, flip it",
      body: "Pick one crate or cell, drain its findings to zero, flip it into the gated list — the baseline only shrinks. The terraform and sweep skills carry the procedures as your agents’ slash commands.",
    },
  ],
  ctaGithub: "View on GitHub",
  ctaGitverse: "Browse on GitVerse",
  tieVibevmHead: "It installs like a dependency",
  tieVibevmBody:
    "Pinned, fingerprinted, materialised into your tree, computed into the boot lane — the discipline arrives the way VibeVM delivers everything.",
  tieVibevmLink: "Why VibeVM",
  tieZapHead: "It runs under Zap’s hood",
  tieZapBody:
    "The family workspace’s Rust engine and TypeScript client are gated by this discipline.",
  tieZapLink: "Why Zap",

  summaryK: "In one paragraph",
  summaryHtml:
    "AI-Native Rust, TypeScript, and Go are one engineering discipline projected onto ordinary languages: the code stays idiomatic and in-distribution, while cells, contracts, spec-to-code traceability, and deviation testimony live in a machine-checked envelope — enforced by deterministic gates beside your formatter, compiler, and linter, and adopted crate by crate under a ratchet that only shrinks. Research beta, piloted on VibeVM itself: <code>vibe install org.vibevm.ai-native/rust-ai-native</code>.",
};
