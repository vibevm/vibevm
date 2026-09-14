# WORKER-REPORT-W1-O3 — manifest fields: prose authorship, navigation, featured, projections and a bridge's origin

Packet: `campaigns/docs-2026-09/findings/PACKET-W1-O3.md`. Branch `research-preview-1-docs`,
worktree `C:\Users\olegc\git\v\vibevm-docs`. Four commits, nothing pushed, no trailers,
`specmap.json` untouched.

**Hashes read from `git log` at the time of writing.** The branch was rewritten under this
task (the coordinator's note: six older commits lost `Co-Authored-By` trailers, every commit
after `818247cd` has a new hash, trees identical). The four commits below are the
post-rewrite hashes; their file counts were re-verified against what was committed
(16 / 20 / 8 / 10 files).

## The four commits

| hash | subject | files |
| --- | --- | --- |
| `ad9aeb39` | `feat(doc): let a documentation package say who wrote its prose` | 16 |
| `11ac451d` | `feat(doc): let a documentation package pin pages and name its sections` | 20 |
| `f950953c` | `feat(doc): name the featured documentations in the site configuration` | 8 |
| `8aa68d2a` | `feat(doc): mark level-zero renderings and carry a bridge's two authorships` | 10 |

Other commits are interleaved in the log: the W1-O4 web worker committed concurrently
(`4639a3e3`, `824403b9`, `49b5d1a7`) and the central session committed the campaign record.
Every commit here was made with `git commit -m … -- <explicit paths>`, so no file of the web
worker's in-flight work was ever staged by this task.

## A — prose authorship (`ad9aeb39`)

**Exact new names.**

- Schema `schemas/doc_manifest.jtd.json`: definition `authorship`, enum
  `["human", "ai", "mixed"]`, `x-vocabulary: "closed"`. Member
  `doc_package.optionalProperties.authorship` → `{"ref": "authorship"}` with
  `"x-default": null`; placed in `x-wire-order` between `lang` and `status`.
- Rust `vibe_wire::generated::doc_manifest::Authorship { Ai, Human, Mixed }`;
  `DocPackage.authorship: Option<Authorship>`.
- TypeScript `Authorship` (erasable const object + union) and `DocPackage.authorship?`.
- Manifest grammar `vibe_core::manifest::Authorship { Human, Ai, Mixed }` (serde
  `rename_all = "lowercase"`, method `as_str`), `PackageMeta.authorship: Option<Authorship>`.
- Both doc packages declare `authorship = "ai"` with a comment naming `##CARD-AUTHORSHIP`
  and stating that the repository's authorship law (PROP-000 `##commits`) is unchanged.

**Refusals.** A word outside the three is refused by the manifest grammar, and the message
names the field and the three words — proven by
`an_unknown_authorship_is_refused_by_name` (serde's spanned TOML error carries
`authorship` and `human`/`ai`/`mixed`), so no second hand-written check was added. A
non-`doc` package declaring it is refused in `validate_documentation` with the repository's
`(violates … #CARD-AUTHORSHIP; fix: …)` form.

**Projection.** `vibe_doc::manifest::Card` reads the word as data and maps it; a word outside
the three reads as ABSENT there rather than refusing a whole render, because that reader is
permissive by design and `vibe check` is the place that refuses. Level 0 carries
`authorship` across into the synthesised card, so a doc package rendered by the site keeps
its badge.

## B — navigation (`11ac451d`)

**Exact new names.**

- Schema: definitions `navigation` (`properties.pinned: string[]` with `x-empty: "emit"`,
  `properties.sections: navigation_section[]` with `x-empty: "emit"`,
  `x-wire-order: ["pinned", "sections"]`) and `navigation_section`
  (`properties.id`, `properties.title`). Root member
  `DocManifest.optionalProperties.navigation` → `{"ref": "navigation"}`; root `x-wire-order`
  is now `["schema_version", "package", "navigation", "pages"]`.
- Rust `Navigation { pinned: Vec<String>, sections: Vec<NavigationSection> }`,
  `NavigationSection { id, title }`, `DocManifest.navigation: Option<Navigation>`; the same
  three in TypeScript.
- Manifest grammar `vibe_core::manifest::NavigationDecl { pinned: Vec<String>, sections:
  Vec<NavigationSectionDecl> }` (the field is `#[serde(rename = "section")]`, so the TOML is
  `[[navigation.section]]`), `NavigationSectionDecl { id, title }`,
  `Manifest.navigation: Option<NavigationDecl>` plus the `ManifestWire` member and both
  conversions.
- Both doc packages declare `pinned = ["start/what-vibevm-is", "start/index"]` and eleven
  sections. English: Start, Model, How to, Agent, Lifecycle, Authoring, Reference,
  Architecture, Diagnostics, Questions, Glossary. Russian: Старт, Модель, Как сделать, Агент,
  Жизненный цикл, Авторам, Справочник, Архитектура, Диагностика, Вопросы, Глоссарий.
  (`skills/` is not a section: it holds `SKILL.md`, not pages.)

**Where each rule lives.** The manifest grammar checks FORM only — a pinned path must be
relative, forward-slashed, without an extension and without `.`/`..`; a section id must be one
path segment; a title must not be blank. Whether the page EXISTS is a question about a tree,
so it is `vibe check`: `doc_package_contract` now errors per empty pin, naming the path as the
pin spells it and `…#NAV-PINNED`. The page order is untouched: `pages` keeps the layer law's
order and the projection applies the pins to nothing.

**Level 0** carries `navigation` across (added to `CARRIED`).

## C — featured (`f950953c`)

**Exact new names.**

- Schema `schemas/doc_site_config.jtd.json`: `site_table.optionalProperties.featured`,
  `elements: string`, `x-empty: "omit"` → Rust `SiteTable.featured: Vec<String>` with
  `#[serde(default, skip_serializing_if = "Vec::is_empty")]`.
- `vibe_doc::site::Site.featured: Vec<String>` (trimmed, blanks dropped, order preserved) and
  `Site::featured_absent(&BTreeSet<String>) -> Vec<&str>`.
- `Site::render()` gained a `featured …` / `featured none` line, so the run's first report
  shows the resolved value like every other.
- `vibe-cli` `web.rs`: `FEATURED = "VITE_SITE_FEATURED"`, joined by `FEATURED_SEPARATOR = ","`,
  set on the child beside `VITE_SITE_DEFAULT_THEME`.
- `vibe doc build-site` prints, after the polled sources:
  `  warn   featured <coordinate> — no source publishes it, so the front of the site shows one
  documentation fewer`. A warning, never a refusal.
- `docker/site.toml` and `site.example.toml` carry
  `featured = ["org.vibevm.core/vibevm", "org.vibevm.core/vibevm-docs"]` with the paragraph
  explaining why featuring lives with the deployment and why an unknown coordinate warns.

A coordinate that is malformed rather than merely unknown is not refused: it can match
nothing, so the same warning covers it. This is the one judgement call in C; the packet asked
only that an unknown coordinate warn.

## D — projections and a bridge's two authorships (`8aa68d2a`)

**Exact new names.**

- Schema: `doc_package.optionalProperties.projection`, `type: boolean`, `"x-default": false`
  → Rust `DocPackage.projection: bool` with
  `#[serde(default, skip_serializing_if = "std::ops::Not::not")]`, TypeScript
  `projection?: boolean`. Optional-with-a-false-default rather than required, so no document
  written before today stops parsing and the wire never carries `projection: false`.
- `doc_package.optionalProperties.bridge` → `{"ref": "bridge_authorship"}`; definition
  `bridge_authorship` with `properties.maintainers: string[]` (`x-empty: "emit"`),
  `properties.upstream_authors: string[]` (`x-empty: "emit"`),
  `optionalProperties.upstream_license: string` (`"x-default": null`),
  `x-wire-order: ["maintainers", "upstream_authors", "upstream_license"]`. Rust
  `BridgeAuthorship`, `DocPackage.bridge: Option<BridgeAuthorship>`; the same in TypeScript.
  `x-wire-order` of `doc_package` now reads `… lang, authorship, projection, bridge, status …`.

**How `projection` is decided.** `projection = card.kind != "doc"`, read off the manifest the
render is built from. No marker is written by the composition, so the rule holds wherever a
manifest is built: a `doc` package (read directly or composed by level 0) is false; every
other kind composed by level 0 is true; the synthetic «this version did not render» manifest
names no kind and is therefore true, which is honest — that page is the site's own.

**How `bridge` is filled.** Only when `[package].bridge = true`. `maintainers` is
`[package].authors` verbatim; `upstream_authors` is the union of the `upstream_authors` of
every `[[embedded_source]]` in declaration order, each name once; `upstream_license` is
carried only when every source states the same one, and is absent when they disagree.
Level 0 now carries `bridge`, `authors` and the whole `[[embedded_source]]` table across —
without that the two authorships would be lost exactly where they matter, since bridges are
rendered at level 0 and are not `doc` packages.

**Facts.** `##LEVEL-ZERO-MARKED` beside `##LEVEL-ZERO`. No PROP-023 edit: `##AUTHORSHIP-
SEPARATION` already says the lists are kept apart, as the packet noted.

## The three new PROP-057 facts

`CARD-AUTHORSHIP` (§7, beside `CARD-FIELDS`), `NAV-PINNED` (§ the reader, beside
`READER-META-AND-PRINT`), `LEVEL-ZERO-MARKED` (§8, beside `LEVEL-ZERO`). Text verbatim as the
packet dictated. All three carry `fact="true" status="impl/done"` and **no**
`action`/`actionstage`/`audience`.

This is the packet's own contingency and it fired: authored with
`action="continue" actionstage="doc" audience="author"`, the coverage gate went red —

```
  UNCOVERED author — vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml#CARD-AUTHORSHIP
    vibevm/vibespecs/common/PROP-057-documentation-packages-and-site.xml:202 — no page cites it
coverage: 531 of 532 obligation(s) told, 99% of 587 audience pair(s) (threshold 100%)
```

— because an `actionstage="doc"` fact with an audience is an obligation a page must cite, and
the prose is the central session's to write. The markers were removed and coverage returned to
100%. **If the central session later writes the pages for these three fields, the markers
should go back on the facts.**

## Verification — verbatim

```
$ cargo fmt --all -- --check
fmt EXIT=0

$ cargo xtask check-codegen
xtask check-codegen: clean.
check-codegen EXIT=0

$ cargo clippy -p vibe-doc -p vibe-cli -p vibe-check -p vibe-core --all-targets -- -D warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 46.31s
clippy EXIT=0

$ cargo test -p vibe-doc
test result: ok. 566 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.83s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 64 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.43s
vibe-doc EXIT=0

$ cargo test -p vibe-core manifest
test result: ok. 353 passed; 0 failed; 0 ignored; 0 measured; 145 filtered out; finished in 0.99s
vibe-core EXIT=0

$ cargo test -p vibe-check
test result: ok. 94 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.44s
vibe-check EXIT=0

$ cargo build -p vibe-cli
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 40.01s
build EXIT=0

$ target/debug/vibe.exe doc check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0 \
    --citations --derived --coverage --media --style --min 100
EXIT=0
citations: 782 rule(s), 790 address(es) to resolve, 2 self-address(es), 0 unresolved, 9 placeholder(s), 2 teaching, 0 unreadable page(s)
style: 49 of 49 page(s) clean, 100% (threshold 100%), 0 error(s), 124 warning(s), 0 unreadable page(s) [en]
media: 0 declared (none), 0 error(s), 0 warning(s), 3 role(s) generated
coverage: 531 of 531 obligation(s) told, 100% of 586 audience pair(s) (threshold 100%), 0 unreadable page(s)
derived: 76 unchanged, 0 moved

$ target/debug/vibe.exe doc check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs-ru/v1.0.0 \
    --citations --derived --translations --style --min 100
EXIT=0
citations: 782 rule(s), 783 address(es) to resolve, 2 self-address(es), 0 unresolved, 9 placeholder(s), 2 teaching, 0 unreadable page(s)
translations: adapting org.vibevm.core/vibevm-docs found in the project's own in-tree registry, 49 page(s), 0 problem(s), 0 unreadable page(s)
style: 49 of 49 page(s) clean, 100% (threshold 100%), 0 error(s), 32 warning(s), 0 unreadable page(s) [ru]
derived: 76 unchanged, 0 moved

$ target/debug/vibe.exe check --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0
vibe check: clean — every check passed against `C:\Users\olegc\git\v\vibevm-docs\vibevm\vibepacks\org.vibevm.core\vibevm-docs\v0.1.0`
EXIT=0

$ target/debug/vibe.exe doc manifest --path vibevm/vibepacks/org.vibevm.core/vibevm-docs/v0.1.0 --json | head -60
{
  "schema_version": 1,
  "package": {
    "group": "org.vibevm.core",
    "name": "vibevm-docs",
    "version": "0.1.0",
    "publisher": "org.vibevm.core",
    "title": "VibeVM Manual",
    "description": "The VibeVM manual: …",
    "abstract": "What it covers: …",
    "lang": "en",
    "authorship": "ai",
    "status": "primary",
    …
  },
  "navigation": {
    "pinned": [
      "start/what-vibevm-is",
      "start/index"
    ],
    "sections": [
      { "id": "start",  "title": "Start" },
      { "id": "model",  "title": "Model" },
      { "id": "howto",  "title": "How to" },
      …
    ]
  },
  …
}

$ cargo xtask specmap --check
Error: `C:\Users\olegc\git\v\vibevm-docs\specmap.json` is out of date relative to the tree.
  drift: units added: 3
Run `rust-ai-native-specmap` (or your project's wrapper), review the drift, and commit the result.
specmap EXIT=1
```

`specmap --check` is red for exactly the three new facts (`units added: 3` =
`CARD-AUTHORSHIP`, `NAV-PINNED`, `LEVEL-ZERO-MARKED`). **The central session regenerates the
map**, as the packet says. Every other gate is green with a real exit code of 0.

`projection` is not visible in `vibe doc manifest --json` above, and that is correct: the
manual is not a projection, and `false` is skipped on the wire. It appears as `"projection":
true` for a level-zero rendering of any non-`doc` package (unit-tested in
`a_rendering_says_whether_the_site_derived_it`).

## Anomalies

1. **`cargo xtask check-codegen` only passes once the regenerated tree is COMMITTED.** It runs
   `codegen` and then `git diff --exit-code` over the generated trees, so a legitimate
   uncommitted schema change reads as drift with the same message as a wrong generator
   version. It was run after each of the four commits and is clean on the current HEAD.
2. **`check-codegen` failed once with `Access is denied. (os error 5)`** while installing the
   fresh generated tree (`restored the complete old tree`). A file-lock race on Windows —
   nothing was left broken and an immediate retry was clean.
3. **`--derived` moved on every atom and had to be accepted.** Changing
   `schemas/doc_manifest.jtd.json` moves the `jtd-schema:schemas/doc_manifest.jtd.json` block
   on `reference/machine-formats.xml`, and changing the two `vibe.toml`s moves the
   `manifest-field:` block on `reference/manifest.xml`. Each atom therefore also carries the
   two packages' `maintenance/derived.json` (the hash record — **not** a page, and no prose
   was touched). This file is one step outside the packet's stated perimeter; the prose around
   both blocks is generic ("the fields of every schema in the table…", "the manifest of this
   manual… shows a `doc` package with a card, a subject and a skill") and stays true, so
   nothing needed rewriting. If the reviewer prefers those records to move in their own
   commit, they are trivially separable.
4. **The branch was rewritten mid-task** (six older commits lost `Co-Authored-By` trailers).
   All four commits survived with identical file counts; the hashes in this report are the
   post-rewrite ones.
5. The W1-O4 worker was editing `web/v0.1.0/**` throughout. Explicit-path commits kept the two
   tasks apart; the only web-package files this task touched are the three the packet names
   (`site/src/generated/doc-manifest.ts`, generated; `docker/site.toml`;
   `site.example.toml`).

## Not done, and why

- **`formats/REGISTRY.toml` — no edit.** Every addition is an optional member, so the
  `doc-manifest` and `doc-site-config` records are unchanged: same epoch, same schema path,
  same `recoverable`/`foreign_parsers`/`corpus`/`sunset`. The registry holds no per-change
  history to append to. Making `projection` a required member would have been an epoch
  question; it is optional with `x-default: false` instead, which yields the same
  `projection: bool` in Rust while keeping every published document readable.
- **`formats/corpora/doc-manifest/e1/*.json` — unchanged, deliberately.** The fixture packages
  declare none of the new fields, and the one required-looking addition (`projection`) is
  skipped when false, so the writer still emits the corpus bytes. `doc_manifest_wire` passes
  untouched. The new members are covered by unit tests instead. If the reviewer wants the
  corpus to exercise them, the fixture needs the fields and a re-bless
  (`VIBE_DOC_BLESS=1 cargo test -p vibe-doc --test doc_manifest_wire`).
- **No page was written and no manual page was touched**, per the perimeter. The three new
  facts therefore carry no `audience`/`action` (see above).
- **`specmap.json` not regenerated**, per the packet.
- **Nothing pushed.**
- **The web package's `site/src/config.ts` was not taught `VITE_SITE_FEATURED`** — that is
  W1-O4/W1-O6's side of the contract. The name and the comma-separated, space-free format are
  fixed here as `web.rs`'s `FEATURED` constant.
