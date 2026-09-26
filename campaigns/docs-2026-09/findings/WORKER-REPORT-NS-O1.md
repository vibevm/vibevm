# WORKER-REPORT-NS-O1 — News & support: the page and the menu entry

Worktree: `C:\Users\olegc\git\v\vibevm\.wt\NS-O1` (detached HEAD from `main`,
at `5ec1611ed`). Nothing committed, nothing staged; git was read-only.
Package root below: `vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/` (`web/`).

## 1. Files

New:

- `web/site/src/news/paths.ts` — the address, as an address: `newsPath()`,
  `newsHref(locale)`, `isNewsPath(pathWithinLocale)`. The shape
  `vision/paths.ts` has, for the reason that file gives: `lib/href.ts` owns
  the manual's address map and a marketing route inside it would be one more
  meaning for `docSegments`.
- `web/site/src/news/i18n.ts` — the copy table in both languages, the owner's
  words verbatim, plus `shownAddress(href)`, which derives the address a card
  prints from the address it leads to.
- `web/site/src/news/meta.ts` — `NEWS_META`: `<title>` and `description` per
  language.
- `web/site/src/news/index.tsx` — the page component.
- `web/site/src/news/styles.css` — its stylesheet.
- `web/site/src/routes/news-and-support/index@landing.tsx` and
  `web/site/src/routes/ru/news-and-support/index@landing.tsx` — the two
  routes, on the landing layout, each composing its head from `pageHead`.
- `web/site/tests/news.spec.ts` — 26 end-to-end tests.

Modified:

- `web/site/src/landing/i18n.ts` — `navNews` on `Strings` and in both
  locales (`News & support` / `Новости и поддержка`).
- `web/site/src/landing/chrome.tsx` — the entry, last in the first row;
  two comments updated to stay true (the row is no longer only "the
  software and where it is kept").
- `web/site/src/landing/chrome.css` — the compact bar's shape (§4 below)
  and the counts in the comments (seven destinations → eight).
- `web/site/tests/chrome.spec.ts` — the header tests that count the first
  row, corrected explicitly (§5).
- `web/tools/lint-links.mjs` — `t.me`, `x.com` and `www.reddit.com` added to
  `ALLOWED_HOSTS` with their reasons.
- `web/tools/parity.mjs` — three new difference rules, `D-37`…`D-39` (§6).
- `web/specmap.json` — regenerated; +126 lines, all of them the six new
  source files. No other entry moved.

Nothing under `crates/**`, no specs, no manual packages, no `BACKLOG.md`, no
root `specmap.json`. No file under `web/site/src/reader/**` or
`web/design/src/components/**` was touched — no new design-system component
was needed, so none was added.

## 2. Addresses and the menu

- `/news-and-support/` and `/ru/news-and-support/`, on the landing layout,
  one slug in both languages. The slug spells the page out rather than
  naming it: a reader who wants the news channel and a reader who wants to
  report a bug arrive at the same page, and an address naming only one of
  them would send the other away.
- The menu entry stands in the **first** row, **last**, after GitVerse, in
  both languages, and leads to the edition being read (`newsHref(locale)`).
  On the page itself it carries `aria-current="page"`, the same way the
  second row's entries do, and it is the only entry that does — asserted.
- Tab order follows the rows: brand, the first row's four, the second row's
  four, then the field, the two letters and the three themes. The tail a
  reader carries between the two halves of the site (search → language →
  theme) is unchanged.
- The footer was **not** touched. The packet names one place for the link —
  the top menu, first row, rightmost — and that is where it is. The footer
  lists the three Why pages, the essay and the two mirrors; whether the
  channels page joins them is the owner's call, one line either way
  (`landing-footer__links` in `chrome.tsx`).

## 3. How the page is built

`landing-shell`, not `landing-full`: the Why pages and the essay bleed to
the window because they are long-form editorial; this is a list of five
destinations, which is a composition inside the landing's own column. That
is also why the chrome needed no layout change — only the nav entry.

Then a heading, the owner's one line under it, and three groups in his
order, each a `<section aria-labelledby>` with a mono group heading over a
short accent rule, holding a `<ul>` of cards.

**Each card is one `<a>` and the whole card is it**, with `rel="noopener"`.
A card with a title link inside it gives a reader two targets of different
sizes for one destination — the small one being the only one that works —
and gives a keyboard reader either two stops or a stop where the ring is not
drawn. The ring is the design system's global `:focus-visible` outline; the
card restates its own `border-radius` under `:focus-visible` so the ring
follows the card's corner instead of the global 3px.

The card is a four-row grid: the platform as a small mono label, the name,
the line, and — pushed to the foot by the `1fr` above it — the address. Four
rows rather than four elements in a row, so the four stand at the same
heights in every card of a group whatever length either language gives them.

- **No third-party logo enters the tree.** The platform is a word
  (`Telegram channel`, `X`, `Reddit`), in the mono face the site labels
  everything else in. Five links would otherwise carry four foreign brands
  with their own colour, clear space and licence, to tell a reader one thing
  a word already tells them.
- **The visible address is derived, never written twice.** `shownAddress`
  strips the scheme, `www.` and the trailing slash: `t.me/vibevm`,
  `x.com/1red2black`, `t.me/vibevm_chat`, `reddit.com/r/vibevm`,
  `t.me/chat_1red2black`. A card whose printed address disagreed with its
  `href` is the one defect on this page a reader cannot catch by reading, so
  the two come from one value.
- **No new colour.** Every value is a semantic token: `--bg-raise` /
  `--bg-sink` for the card, `--line` / `--line-strong` for its hairline,
  `--text` / `--text-2` / `--text-3` for the three text roles, `--accent` /
  `--accent-hover` for the address, `--radius-md` and `--speed`. Measured
  against the package's own APCA thresholds (interactive ≥ 45), using the
  audit's own constants: `--accent` on `--bg-raise` is |Lc| 46.44 dark /
  55.46 light, and on hover `--accent-hover` on `--bg-sink` is 50.73 dark /
  58.49 light. The gated pairs the audit itself measures are unchanged and
  green.
- The grid is `repeat(auto-fill, minmax(16rem, 1fr))` and **not**
  `auto-fit`: `auto-fit` collapses the tracks a group does not fill and
  stretches what is left over them, which turned the group of two into two
  half-page cards and the group of one into a card the width of the column
  (seen and rejected on the first render). `auto-fill` keeps every card the
  same size in every group and lets a group of two end where it ends.
- Nothing hydrates; the only motion is the hover the family already has, and
  it is off under `prefers-reduced-motion`.

## 4. The compact bar — one shape, measured

A fourth entry does not fit across a phone. Measured at 13px over the built
bytes, with 40px of the window going to the header's own padding: the first
row needs **372px of Russian** and **329px of English** (`Новости и
поддержка` alone is 140px), the second row 374px of Russian — and a 390px
phone offers 350, a 320px phone 280. Before this change the first row's
three entries fit any phone; now neither row does, in either language.

Left alone, `flex-wrap` reported that as a 3+1 wrap in Russian and four
across in English **at the same width**: two compositions, neither chosen,
and a bar whose shape depends on the language it is read in. So the declared
two-column shape the second row already had at ≤600px now belongs to both
rows — `repeat(2, max-content)` — and the bar is the same five lines in both
languages at every phone width. Widest column pair is 249px of Russian,
which fits the narrowest phone sold.

It costs a line: the phone bar is 170px rather than 142px, and it is sticky,
so 28px of every screen a reader scrolls. That is the price of the fourth
destination, paid where it is cheapest; it is written down in the stylesheet
beside the rule.

Desktop is untouched in shape and stays within every geometric promise the
header tests make (measured at 1920/1440/1280/1024/834, both languages):
the two rows still share one axis (max observed difference 0.5px, tolerance
8), the cluster still leans toward the brand by 15.5–33.5px (bounds 4–64),
and the air after the brand is 74–253px (floor 40).

## 5. The header tests, corrected explicitly

`web/site/tests/chrome.spec.ts`:

- `ROWS.en[0]` / `ROWS.ru[0]` gain the entry before `search`.
- A `destinations(row)` helper drops the controls, replacing the inline
  filter the axis assertion used.
- The tab-order test: `order.slice(0, 9)` is now brand + the first row's
  four + the second row's four; `order.slice(9)` is the unchanged tail.
- The phone branch: five lines instead of four, every line after the brand's
  is two wide, and the two lines of each row concatenate back to that row in
  order — a grid fills row-major, so the shape and the order are both
  pinned.
- Three comments that counted seven destinations or "the first row's three"
  now say eight and four.

`web/site/tests/news.spec.ts` (26 tests, both languages): both pages answer
200 with the owner's heading, `<title>` and `lang`; the three group headings
in order; canonical, the `hreflang` pair, `x-default`, `og:url`,
`description` and the `WebPage` graph; the first row's last entry checked on
the landing, a Why page and the essay in both languages; current on the page
itself; the twin offered by the language switch and followed; the five cards
with exact `href`s, `rel="noopener"` and exact printed addresses, and that
`main` holds exactly five links; a visible focus ring on a card; no sideways
scroll at 1440, 834 and 390; one `main`, an unbroken heading order and no
`aria-labelledby` pointing at a missing id.

The five channel addresses are written out **by hand** in the test rather
than imported from the copy table: a test that read the constant the page
renders would prove the page consistent with itself and nothing else, and a
wrong channel address is the one value here that looks right while being
wrong.

## 6. Sitemap, metadata, and the gates that list addresses

- **Sitemap.** Derived from what was built, so the pair entered
  `/sitemap.xml` by itself at the address-derived weights: `0.8` /
  `monthly` for `/news-and-support/`, `0.7` / `monthly` for
  `/ru/news-and-support/`. `feed.xml` and `llms-full.txt` pick the pages up
  the same way (both carry them). The documentation's own
  `/doc/sitemap.xml` is untouched.
- **Head.** From the shared builder, so nothing was composed by hand:
  `canonical`, `hreflang en|ru|x-default`, `og:*`, `twitter:*`,
  `theme-color`, the icon, the `llms.txt` alternate, the font preloads, and
  the `WebPage` graph naming the site — verified in the built HTML.
- **`<title>`** is the one value composed rather than quoted, because the
  owner named the page and not its tab: `News & support — VibeVM` /
  `Новости и поддержка — VibeVM`. The neighbours spell theirs as
  «name — argument» where the argument is the owner's own line about that
  page; inventing one here would put a sentence nobody wrote in the place a
  search result shows first.
- **Structured data** is the default `WebPage`, like every subpage. The five
  channels are deliberately **not** added to the root's
  `SoftwareApplication.sameAs`, although that is where they would earn their
  keep for a crawler: the root graph is compared byte for byte against the
  site it replaced, and `landing/head.ts` says in as many words that
  changing it is a decision to be taken deliberately rather than absorbed
  into another change. Named here as a candidate for the owner.
- **`llms.txt`** was left alone for the same reason — the `## Project`
  section names the three product arguments and the essay, and whether the
  channels belong in an index an agent reads before fetching is the owner's
  call. Candidate, one line in `tools/root-files.mjs` plus a parity rule.
- **`tools/lint-links.mjs`.** The three hosts are now in `ALLOWED_HOSTS`
  with their reasons rather than counted among "outbound links the prose
  cites": they are the site's own writing, so a channel address that stopped
  being ours is a defect of this site and the report should say so by name.
  The build now prints `6x t.me`, `2x www.reddit.com`, `2x x.com`.
- **`tools/parity.mjs`.** Three rules, in the shape D-33…D-36 established
  for the essay: `D-37` the new address pair, `D-38` the page's text and the
  menu entry, `D-39` the two sitemap entries. `D-38` matches against a set
  **computed from the page's own copy table** (`NEWS_FRAGMENTS`) rather than
  a hand-written list, so a second copy of the owner's words cannot drift
  inside a gate, and a card added tomorrow is neither an unexplained
  difference nor an unnoticed one.
  The parity gate itself could not be run for real: it takes a `dist/` of
  the Astro site built from a scratch copy of that repository, which this
  worktree does not have. It was smoke-tested against this build as both
  sides (`node tools/parity.mjs site/dist site/dist`, exit 1): the module
  loads, the new imports resolve and the fragment set builds (38 fragments);
  the two reported differences are the AI-Native slug pair `D-29` names,
  which has no counterpart when the reference IS this build. No rule of mine
  was needed in that degenerate run, as expected — the new addresses exist
  on both sides of it.
- `tools/visual-parity.mjs`, `tools/layout-parity.mjs` and
  `tools/visual-classify.mjs` list **paired** addresses for comparison
  against the Astro reference. A page that never existed there has no pair,
  which is why the essay is not in them either; nothing was added.

## 7. Self-check — verbatim

Run in `vibevm/vibepacks/org.vibevm.doc/web/v1.0.0/`, in the packet's order,
all five after the last source change. Per-test `✔` lines of the floor's
node-test step (141 of them) and the 229 `ok` lines of the Playwright run are
elided where marked; everything else is quoted as printed.

### `TYPESCRIPT_AI_NATIVE=… node tools/floor.mjs --keep-going`

```
=== prettier --check (floor perimeter: design/src, site/src) ===
Checking formatting...
All matched files use Prettier code style!

=== tsc --noEmit ===

=== tests (node --test) ===
[141 ✔ lines elided]
ℹ tests 141
ℹ suites 12
ℹ pass 141
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0

=== eslint (floor perimeter: design/src, site/src) ===

=== typescript-ai-native-conform check ===
typescript-ai-native-conform: policy conform.toml (loaded).
typescript-ai-native-conform: extracted 0 file(s), 172 cached (producer ts-tsc-2).
typescript-ai-native-conform check: 0 finding(s) in scope <workspace> ({}), 0 frozen in baseline, 0 new; SARIF at target\conform\report-typescript.sarif.
typescript-ai-native-conform: 0 cell(s) gated, 0 exempt — see conform.toml for the why of each.

=== typescript-ai-native-specmap --check ===
typescript-ai-native-specmap --check: clean (0 spec units, 168 tagged code items, 168 edges, 0 suspects, 168 warnings).
typescript-ai-native-specmap: ratchet gate — 0 orphan(s) (0 root(s) exempt).

=== test-gate (xfail-strict) ===
test-gate: running `node --test --test-reporter=tap` over the policy's TS roots …
test-gate: 153 results parsed (0 failed, 0 skipped), baseline entries: 0
test-gate: green (xfail-strict).

floor: all green (7 step(s) run, 0 disabled by policy).
EXIT=0
```

(The 168 specmap warnings are the package's standing state, not new: every
unit cites a requirement of PROP-057, which lives in the host repository and
is deliberately left unresolved here — `specmap.toml` says why. The six new
files add six such warnings and no suspects, no orphans.)

### `node design/audit/contrast.mjs`

```
[46 gated pairs and 6 reference pairs elided — all PASS/GOOD/EXCELLENT]

=== pairs: gated=46, reference=6; below threshold=0 ===
@media(prefers-color-scheme:dark) agrees with [data-theme="dark"]: OK (0 divergences)
tokens.css writes no colour of its own: OK (every value is a var() into palette.css)
EXIT=0
```

### `node tools/build.mjs static`

```
SSG results
- Generated: 28 pages
- Duration: 189.8 ms
- Average: 6.8 ms per page

[two vite "top layout feature is deprecated" notes and the SSG-skipped note
 elided — both pre-existing and unrelated]

build (static): generated 28 page(s), expected 28
build (static): removed dist/q-manifest.json from the output
build (static): 8 island(s) from the documentation trees, 0 from the package's own fixture page
build (static): 24 file(s) copied from 1 documentation tree(s) for 2 edition(s) of 1 library (2 page(s) in a language that does not carry them); 14 written — catalogue, manifests, 3 sitemap part(s) over 7 address(es), resolver, search index over 5 entries
build (static): root files robots.txt, llms.txt, llms-full.txt, sitemap.xml, feed.xml, og.png; 8 font file(s) at /fonts/, 13 page(s) repointed at the bundled faces, 17 crawler name(s) from 9 provider page(s)
build (static): csp.txt — 68 inline script hash(es) over 191 occurrence(s), no external source; .vibe-site/csp.conf written for the serving container
build (static): no trace of the previous framework — 4 mark(s) looked for, none found
documentation links — every address against the files behind it
    ok        29 page(s), 354 file(s) in the output
    ok        1125 link(s) followed, 38 hreflang pair(s) checked
    ok        9 page(s) name another page canonical and are read as it
    ok        36x github.com — the canonical source repository, linked by the landing
    ok        35x gitverse.ru — the source mirror, linked by the landing
    ok        6x t.me — the project's Telegram channel and its two chats, linked by /news-and-support/
    ok        2x www.reddit.com — the project's subreddit, linked by /news-and-support/
    ok        2x x.com — the author's account, linked by /news-and-support/
    L-01      4x — The island golden cites `media/diagram.svg` beside the DOCUMENT, and the pipeline publishes a package's media at the root of its tree under a content name — so the address misses at whatever depth the page is served, and the fixture package carries no such file in either place. SVG is not an allowed medium in this wave (D-20-6). Filed as an island-golden finding; the picture is the only broken one on the fixture page.
links: green — 1125 checked, 0 broken.
build (static): ok
EXIT=0
```

Page count 26 → 28: the two new routes, counted on disk by the gate itself.
`L-01` is the pre-existing, filed island-golden exception.

### `node node_modules/@playwright/test/cli.js test -c <copy of the config on port 4373>`

The copy was `site/tests/playwright.local.config.ts` — byte-identical to
`playwright.config.ts` except `PORT = 4373` — and it was **deleted after the
run**, as the packet allows. The whole suite ran with it.

```
[203 ok lines elided; the 32 concerning this task quoted]

  ok   9 [chromium] › site\tests\chrome.spec.ts:185:3 › the English landing composes its header at every width (368ms)
  ok  10 [chromium] › site\tests\chrome.spec.ts:185:3 › the Russian landing composes its header at every width (400ms)
  ok  11 [chromium] › site\tests\chrome.spec.ts:185:3 › an English Why page composes its header at every width (870ms)
  ok  12 [chromium] › site\tests\chrome.spec.ts:185:3 › a Russian Why page composes its header at every width (523ms)
  ok  13 [chromium] › site\tests\chrome.spec.ts:321:1 › the manual keeps its single-row header (192ms)
  ok  14 [chromium] › site\tests\chrome.spec.ts:371:1 › the bar is tabbed by kind, and marks where the reader stands (247ms)
  ok  76 [chromium] › site\tests\news.spec.ts:91:3 › /news-and-support/ is served and opens with its own heading (93ms)
  ok  77 [chromium] › site\tests\news.spec.ts:91:3 › /ru/news-and-support/ is served and opens with its own heading (109ms)
  ok  78 [chromium] › site\tests\news.spec.ts:114:3 › /news-and-support/ declares its own address (102ms)
  ok  79 [chromium] › site\tests\news.spec.ts:114:3 › /ru/news-and-support/ declares its own address (129ms)
  ok  80 [chromium] › site\tests\news.spec.ts:168:3 › / ends its first header row with the channels page (112ms)
  ok  81 [chromium] › site\tests\news.spec.ts:168:3 › /ru/ ends its first header row with the channels page (127ms)
  ok  82 [chromium] › site\tests\news.spec.ts:168:3 › /why/zap/ ends its first header row with the channels page (168ms)
  ok  83 [chromium] › site\tests\news.spec.ts:168:3 › /ru/why/zap/ ends its first header row with the channels page (151ms)
  ok  84 [chromium] › site\tests\news.spec.ts:168:3 › /vision/ ends its first header row with the channels page (125ms)
  ok  85 [chromium] › site\tests\news.spec.ts:168:3 › /ru/vision/ ends its first header row with the channels page (141ms)
  ok  86 [chromium] › site\tests\news.spec.ts:194:3 › /news-and-support/ marks itself current in the header (97ms)
  ok  87 [chromium] › site\tests\news.spec.ts:194:3 › /ru/news-and-support/ marks itself current in the header (99ms)
  ok  88 [chromium] › site\tests\news.spec.ts:207:3 › /news-and-support/ offers its twin, not the front door (148ms)
  ok  89 [chromium] › site\tests\news.spec.ts:207:3 › /ru/news-and-support/ offers its twin, not the front door (162ms)
  ok  90 [chromium] › site\tests\news.spec.ts:226:3 › /news-and-support/ carries the five channels the owner named (108ms)
  ok  91 [chromium] › site\tests\news.spec.ts:226:3 › /ru/news-and-support/ carries the five channels the owner named (117ms)
  ok  92 [chromium] › site\tests\news.spec.ts:257:3 › /news-and-support/ gives a card a visible focus ring (105ms)
  ok  93 [chromium] › site\tests\news.spec.ts:257:3 › /ru/news-and-support/ gives a card a visible focus ring (119ms)
  ok  94 [chromium] › site\tests\news.spec.ts:272:5 › /news-and-support/ does not scroll sideways at 1440px (79ms)
  ok  95 [chromium] › site\tests\news.spec.ts:272:5 › /news-and-support/ does not scroll sideways at 834px (78ms)
  ok  96 [chromium] › site\tests\news.spec.ts:272:5 › /news-and-support/ does not scroll sideways at 390px (79ms)
  ok  97 [chromium] › site\tests\news.spec.ts:272:5 › /ru/news-and-support/ does not scroll sideways at 1440px (87ms)
  ok  98 [chromium] › site\tests\news.spec.ts:272:5 › /ru/news-and-support/ does not scroll sideways at 834px (88ms)
  ok  99 [chromium] › site\tests\news.spec.ts:272:5 › /ru/news-and-support/ does not scroll sideways at 390px (96ms)
  ok 100 [chromium] › site\tests\news.spec.ts:288:3 › /news-and-support/ has one main and an unbroken heading order (101ms)
  ok 101 [chromium] › site\tests\news.spec.ts:288:3 › /ru/news-and-support/ has one main and an unbroken heading order (119ms)

  5 skipped
  229 passed (57.2s)
EXIT=0
```

The 5 skipped are the pre-existing conditional skips in
`local-reader.spec.ts` (`test.skip(missing !== "", missing)` — they need a
native local build this worktree does not carry). Nothing was skipped by
this change.

### `node tools/build.mjs embedded`

```
SSG results
- Generated: 14 pages
- Duration: 81.1 ms
- Average: 5.8 ms per page

[the SSG-skipped note and the cache-headers banner elided]

build (embedded): generated 14 page(s), expected 14
build (embedded): removed dist-embedded/q-manifest.json from the output
build (embedded): 14 page(s) carry none of the 7 public-only tags
build (embedded): no trace of the previous framework — 4 mark(s) looked for, none found
build (embedded): ok
EXIT=0
```

Unchanged at 14: the embedded build is the reader's shell and does not count
the landing routes, so the new pair does not enter it — which is right, and
is why its public-head check still passes.

## 8. Deviations

None from the packet's instructions.

Two things the packet did not decide, decided here and named so they can be
reversed in one line each:

1. **The compact bar gained a line** (§4). The packet asked for a fourth
   entry in the first row; on a phone that row no longer fits across, in
   either language, and something had to give. What was chosen is the
   declared two-column shape the row below already had, over a wrap that
   would differ per language. Cost: 28px of sticky header on a phone.
2. **No footer entry, no `llms.txt` line, no `sameAs`** (§2, §6). Each is
   one line; each is the owner's call; each is named above rather than
   taken quietly.

## 9. Environment — the packet's correction, applied

The packet's junction of `node_modules` onto the main tree was created,
found to break the build exactly as the coordinator's correction says (vite
could not resolve `@vibe-docs/design/base.css`, because that workspace link
lives in `site/node_modules` and not in the package root), and then
**removed as a link** — both it and a `site/node_modules/@vibe-docs/design`
junction of my own — with `(Get-Item … -Force).Delete()`. Nothing in the
main tree was touched. `pnpm install --frozen-lockfile --offline` then ran
in this worktree in 3.2s (319 packages, 0 downloaded), and
`site/node_modules/@vibe-docs/design` now points at **this worktree's**
`design/`. Every result in §7 was produced after that, with no path leading
outside this worktree.

Throwaway files, all removed: the port-4373 config copy, and three
measurement scripts kept in the session scratchpad rather than in the tree.
`git status` in the worktree is exactly the seven modified files and the
four new paths of §1.
