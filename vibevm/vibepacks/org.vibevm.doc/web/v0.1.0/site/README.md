# site/

The Qwik application: the landing routes, the documentation routes under
`/doc/`, and the island the Rust pipeline fills.

**Two configurations, one source.** `vite.config.ts` builds the whole site
at base `/`; `vite.config.embedded.ts` builds the documentation routes
alone at base `/doc/`, with `src/routes/doc` as the route root. Each has an
adapter configuration beside it under `adapters/`, and both are driven by
`tools/build.mjs`, which counts the pages the generator reports against the
pages the manifest declares.

**The build order is not a preference.** The client environment is built
first, from the base configuration; the adapter build then reads the client
manifest it wrote. Running only the adapter build — even through Vite's app
builder, which the adapter's own diagnostic suggests — leaves the client
manifest stale, and every page then fails to serialize with `Code(Q14)`,
naming a symbol it cannot resolve.

**`public/`** is copied into the output verbatim, so nothing that is not
served belongs in it. It holds `manifest.json`, the web app manifest,
rewritten from the Qwik starter's: the starter's version opens with a
`$schema` pointing at `schemastore.org` — an address on someone else's host,
in a file this site serves, for no benefit to any reader — and names icon
files this site does not have. Both are gone. The two colours in it are the
light theme's ground and accent, written as literals because a browser reads
that file before any stylesheet and cannot resolve a token
(PROP-057 `##STACK-BUILD-HYGIENE`).

## The documentation reader

`src/routes/doc/[...path]/` is one catch-all route for three addresses —
a page, a package's own page, and a language's catalogue — because they
are the same address at three lengths, and a second route would be a
second opinion about where a language ends and a group begins. The door
at `/doc/` is the only documentation route file beside it.

**The behaviours are plain TypeScript in `src/reader/`, not components,
and that is forced.** The page a reader reads is an island: finished HTML
the Rust pipeline rendered, inserted as a string, never turned into a
component tree and never re-rendered. Nothing inside it can carry a Qwik
handler. So each behaviour listens once on a region, narrows what was
clicked, and returns its own teardown; `mount.ts` composes them and the
route starts it from a visible task.

Two things about that arrangement are worth knowing before changing it.
The chrome the reader ATTACHES to the island — the fence toolbars, the
table regions, the contents — is added before the behaviours that listen
over it, because a listener attached before its element exists never
fires. And every read and write of `localStorage` is wrapped: a browser
with site data blocked throws on the property access itself, and a reader
with cookies off should lose their font size rather than their page.

**Nothing moves the reader except the reader.** That needed an inline
script in the head, which the route declares: two mechanisms move a
reloaded page before any module of it runs — the browser's own scroll
restoration and Qwik Router's copy in `history.state` — and the second is
the one that was actually doing it. Back and forward are exempt, told
apart by the navigation type. The cost is a second inline script for a
CSP to carry the hash of; the cheaper home for those two lines is
`design/theme-init.js`, which already runs first for the same kind of
reason.

**A language never 404s.** Every language materialises every page of the
source: one an adaptation has not reached yet is written at its address
with the source's text, `rel=canonical` to the source, `noindex`, a
bilingual notice shown once per session, and its internal links
re-pointed so a reader does not silently fall back into the source
language. The build drops those pages from the sitemap, because a sitemap
saying «index this» beside a page saying «do not» costs a crawler a fetch
to be told to go away.

**The door chooses a language in the browser, and that is a limitation
rather than a shortcut.** `Accept-Language` is a request header and a
directory of files has nothing that reads one; `navigator.languages` is
the same preference list seen from the other side of the request. The
order is the vision's — what the reader chose before, then what the
browser asks for, then the documentation's own language — and the
decision is taken once per session and never against an explicit click.
Served one day by something that can read a header, it becomes a 302 and
`src/reader/catalogue.ts` goes away.

## The agent surfaces

Under every package address lie the files a machine reads: `<page>.md` and
`<page>.xml` beside each page, the four `llms` tiers, `manifest.json` and
the card images. **They are copied, never computed** — the pipeline wrote
them from the package's own source, with the block numbers it assigned
(R-26) and the citations it resolved, and a site build producing its own
Markdown out of a rendered island would be a second renderer disagreeing
with the first.

`VIBE_DOC_OUT` names the `vibe doc build` outputs to copy, separated by
the platform's path delimiter; one package rendered in three projections
is three directories sharing a coordinate, and all of them are read. A
tree for a language is placed under the SOURCE package's coordinate with
the language segment in front (D-06); a page an adaptation does not carry
takes the source's surfaces, exactly as its page takes the source's text.
The default, when the environment names nothing, is
`src/fixtures/doc-build` — which is what makes a plain `pnpm build:static`
produce a site whose every link resolves.

Two documents are the SITE's own rather than any package's, and are
composed from the manifests: `/doc/llms.txt`, the arXiv-shaped catalogue
of the editions carried, and `/doc/manifest.json`, the same for a machine
that will act on it. `/doc/resolve/` is the resolver: on a static host a
citation cannot be answered with a 302, so the build publishes the map as
`/doc/resolve.json` and one hand-written page that reads it in the browser
and follows it (F-15). The local reader, which has a server, keeps the
route instead.

## The landing

`/` and `/ru/` are the landing the domain has served since it opened, moved
here from Astro one string at a time (D-28). The copy lives in
`src/landing/i18n.ts` and is the owner's, byte for byte, asymmetries
included; the page is assembled from the same `design/` components the
documentation uses, and the parity test below is what says so.

**The landing wears its own chrome, by name.** `src/routes/layout.tsx` is
the documentation's frame — brand, search, footer. The landing addresses ask
for a different one the way Qwik Router provides for: the route file is
`index@landing.tsx` and the layout it names is `layout-landing.tsx`, which
stops the chain. So `/`, `/ru/`, `/en/` and `/404.html` sit in the same route
tree as `/doc/…` and wear a header carrying the way in to the manual, the two
source mirrors and the language switch, with no condition inside either
frame asking which half of the site it is on.

**`404` and `/en/` are routes, not server configuration.** `404@landing.tsx`
builds to `/404.html`, the file `error_page 404` has always served.
`/en/index.html` answers the legacy English prefix that nginx used to
redirect: a static build cannot return a status code, so the page redirects
itself and tells crawlers not to index it.

**`<html lang>` is decided in `entry.ssr.tsx`.** It is a container attribute
— written before any component runs — so no route can set it, and a Russian
page under an English shell is invisible to everything except a parity test.
The address is what knows the answer, and on this site the address is where
the language lives.

**Configuration comes from the build environment** (`src/config.ts`):
`VITE_SITE_ORIGIN` for the domain, `VITE_UMAMI_WEBSITE_ID` for the
analytics tag, `VITE_INDEXNOW_KEY` for the key file, `VITE_SITE_LASTMOD` for
the sitemap. The last three default to empty on purpose: an id or a key
identifies a live property, belongs to the deployment rather than to the
repository, and an empty one means «publish nothing», never «publish a
guess». No website id, no analytics tag at all; no key, no key file.

## The machine files of the domain

`tools/root-files.mjs` writes `robots.txt`, `llms.txt`, `llms-full.txt`,
`sitemap.xml`, `feed.xml`, the IndexNow key file and `og.png` into the
static output, and copies the fonts to their public `/fonts/*.woff2`
addresses. They are derived from what the build actually produced rather
than kept by hand, because a hand-kept sitemap goes stale the first time a
page moves (`##SITE-ONE-SITE`).

Two details are easy to undo by accident. The crawler names in `robots.txt`
are read from each provider's own documentation on the build day and never
from memory (`##SEO-ROBOTS`, F-38); the list carries the date it was
checked. And the font preload links are rewritten after rendering to point
at the content-hashed asset the stylesheet fetches — a preload of the public
path beside a `@font-face` naming the hashed one is not a preload, it is a
second download of the same face.

`og.png` is drawn, not stored: `tools/og-card.mjs` renders the hero's
constellation on the brand ground, in colours read out of `design/palette.css`,
and encodes the PNG itself. The Astro site referenced the file from four meta
tags and never had it (A0.26).

## The parity test

`node tools/parity.mjs <astro-dist> [site/dist]` compares this build against
the Astro landing's: the set of addresses, and for `/`, `/ru/` and
`/404.html` the language, title, description, `canonical`, every `hreflang`,
every `og:` and `twitter:` tag, the normalised structured data, the analytics
tag and the visible text; `robots.txt` and the key file byte for byte;
`llms.txt` by its disambiguation paragraph and its links; the sitemap by its
addresses.

Every difference is either fixed or a rule in the script with the reason it
exists, and a difference no rule covers fails the run (F-73). A green run is
the gate for pointing the domain at this build. The reference is built in a
scratch copy of the Astro repository — `npm ci && npm run build && node
scripts/build-llms-full.mjs` — never in that repository itself, which this
campaign only reads (R-28).

## The end-to-end run

`pnpm test:e2e` builds nothing: it reads `site/dist` through
`tests/serve.mjs`, forty lines of Node that bind `127.0.0.1`, serve
`<route>/index.html` behind a directory address and 308 an address
missing its slash. **Run a build first** — the suite measures whatever is
on disk, and a stale `dist` is a suite measuring last week.

It exists because the reader's behaviours are the one part of this
package a type checker cannot judge at all. A block «in the middle of the
window», a page that does NOT scroll itself on reload, a theme already
right at the first frame, a table inside a scrolling region: none is a
pure function, and a unit test that never opens a page passes on all of
them while a reader sees none.

One worker and no retries, on purpose. Several tests read and write one
origin's `localStorage` and one of them is about what a reload restores,
so parallel workers would be two readers sharing one memory; and a retry
that turns a failure green is a failure that comes back later, in front
of somebody else.

The browser is downloaded into the user's own Playwright cache by
`pnpm exec playwright install chromium` and never into this tree.
