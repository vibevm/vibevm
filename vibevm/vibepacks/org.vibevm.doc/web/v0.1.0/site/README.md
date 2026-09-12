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

## The landing

`/` and `/ru/` are the landing the domain has served since it opened, moved
here from Astro one string at a time (D-28). The copy lives in
`src/landing/i18n.ts` and is the owner's, byte for byte, asymmetries
included; the page is assembled from the same `design/` components the
documentation uses.

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
