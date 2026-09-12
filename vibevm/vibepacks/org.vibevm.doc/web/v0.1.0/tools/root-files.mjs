#!/usr/bin/env node
// The machine files at the root of the domain, written by the build that
// writes the pages — because they describe those pages and nothing else
// knows what was built (PROP-057 `##SITE-ONE-SITE`).
//
// `robots.txt`, `llms.txt`, `llms-full.txt`, `sitemap.xml`, `feed.xml`,
// the IndexNow key file, `og.png` and the public copies of the fonts are
// one output of one build. The Astro site kept most of them by hand in
// `public/` and generated two of them with a script that ran after the
// build; a file kept by hand goes stale the first time a page moves, so
// here every one of them is derived from what is actually on disk.
//
// It runs after the page-count gate in `tools/build.mjs` and only for
// the static build: the embedded output is a route template that `vibe`
// fills in on a reader's own machine, and a sitemap of it would name
// addresses that exist nowhere.
//
// Usage as a command, for regenerating without a rebuild:
//   node tools/root-files.mjs [outDir]

import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

import { siteConfig } from "../site/src/config.ts";
import { DOC_SITEMAP } from "../site/src/seo/sitemap.ts";
import { writeOgCard } from "./og-card.mjs";

const PACKAGE_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const SITE_ROOT = join(PACKAGE_ROOT, "site");
const FONTS_DIR = join(PACKAGE_ROOT, "design", "fonts");

/**
 * The crawlers named one by one, with the date and the page each name
 * was read from.
 *
 * `##SEO-ROBOTS` says the list is verified against the providers' own
 * documentation at every build and never written from memory, and F-38
 * says the source and the date go into the report. So they go into the
 * file too: a name here without a source beside it is a name somebody
 * remembered.
 *
 * Every stanza allows everything. The file is a statement of intent, not
 * a gate — `User-agent: *` at the end already allows the whole web — and
 * its purpose is to say to the crawlers that look for their own name
 * that this site wants to be read by them, including for training.
 */
const CRAWLERS_VERIFIED_ON = "2026-09-12";

const SOURCES = [
  ["OpenAI", "https://developers.openai.com/api/docs/bots"],
  [
    "Anthropic",
    "https://support.claude.com/en/articles/8896518-does-anthropic-crawl-data-from-the-web-and-how-can-site-owners-block-the-crawler",
  ],
  [
    "Google",
    "https://developers.google.com/search/docs/crawling-indexing/google-common-crawlers",
  ],
  ["Perplexity", "https://docs.perplexity.ai/guides/bots"],
  ["Common Crawl", "https://commoncrawl.org/ccbot"],
  ["Apple", "https://support.apple.com/en-us/119829"],
  ["Amazon", "https://developer.amazon.com/amazonbot"],
  [
    "Meta",
    "https://developers.facebook.com/docs/sharing/webmasters/web-crawlers/",
  ],
  [
    "Yandex",
    "https://yandex.com/support/webmaster/robot-workings/check-yandex-robots.html",
  ],
];

/**
 * The verification itself, exported so the parity report can quote the
 * date and the pages rather than restate them from memory.
 */
export const CRAWLER_VERIFICATION = {
  date: CRAWLERS_VERIFIED_ON,
  sources: SOURCES,
};

const CRAWLERS = [
  "GPTBot",
  "ChatGPT-User",
  "OAI-SearchBot",
  "ClaudeBot",
  "Claude-User",
  "Claude-SearchBot",
  "PerplexityBot",
  "Perplexity-User",
  "Google-Extended",
  "CCBot",
  "Applebot-Extended",
  "Amazonbot",
  "Amzn-SearchBot",
  "Amzn-User",
  "Meta-ExternalAgent",
  "Meta-ExternalFetcher",
  "YandexBot",
];

/** The faces preloaded by name; the pages ask for them by public path. */
const PUBLIC_FONT_DIR = "fonts";

// ---------------------------------------------------------------------
// Reading what the build produced
// ---------------------------------------------------------------------

/** Every `.html` under a directory, as absolute paths. */
function htmlFiles(dir, found = []) {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) htmlFiles(full, found);
    else if (entry.endsWith(".html")) found.push(full);
  }
  return found;
}

/**
 * The public address of a built page: the path of its directory, with a
 * trailing slash, or the file itself when it is not an `index.html`
 * (`/404.html` is a file and not a place).
 */
function addressOf(outDir, file) {
  const rel = relative(outDir, file).split(sep).join("/");
  return rel.endsWith("index.html")
    ? `/${rel.slice(0, -"index.html".length)}`
    : `/${rel}`;
}

/** The pages of the landing: the two content addresses, in order. */
function landingPages(pages) {
  return pages
    .filter(
      (page) =>
        !page.address.startsWith("/doc/") &&
        !page.address.startsWith("/en/") &&
        !page.address.startsWith("/404"),
    )
    .sort((a, b) => a.address.localeCompare(b.address));
}

/**
 * A page that asks not to be indexed does not stand in a sitemap.
 *
 * The two say opposite things otherwise: a sitemap is «index this» and
 * `noindex` is «do not», and a crawler handed both spends a fetch to be
 * turned away. The pages this is about are the translation fallbacks —
 * the source's text materialised under an adaptation's address so a
 * language never 404s (`##READER-LANGUAGE-SWITCH-KEEPS-PLACE`) — and
 * they carry `canonical` to the source as well, so what a crawler should
 * keep is named rather than merely implied.
 *
 * It reads the built HTML rather than being told, and it runs over every
 * sitemap in the output rather than one: this file writes the domain's
 * sitemap and the documentation writes its own, and the rule belongs to
 * whatever writes a sitemap rather than to any one of them. It is
 * idempotent — a second pass over a pruned file removes nothing.
 */
function pruneSitemaps(outDir) {
  let removed = 0;
  const walk = (dir) => {
    for (const entry of readdirSync(dir)) {
      const full = join(dir, entry);
      if (statSync(full).isDirectory()) {
        walk(full);
        continue;
      }
      if (!entry.endsWith(".xml")) continue;
      const xml = readFileSync(full, "utf8");
      if (!xml.includes("<urlset")) continue;
      let dropped = 0;
      const kept = xml.replace(/[ \t]*<url>[\s\S]*?<\/url>\n?/g, (one) => {
        const loc = /<loc>([^<]*)<\/loc>/.exec(one);
        if (loc === null) return one;
        const path = new URL(loc[1]).pathname;
        const file = join(outDir, path.replace(/^\/+/, ""), "index.html");
        if (!existsSync(file)) return one;
        const html = readFileSync(file, "utf8");
        if (!/<meta[^>]+name="robots"[^>]+content="[^"]*noindex/i.test(html)) {
          return one;
        }
        dropped += 1;
        return "";
      });
      if (dropped > 0) {
        writeFileSync(full, kept, "utf8");
        removed += dropped;
      }
    }
  };
  walk(outDir);
  return removed;
}

/** A page's `<title>`, decoded. */
function titleOf(html) {
  const found = /<title[^>]*>([\s\S]*?)<\/title>/i.exec(html);
  return decodeEntities(found?.[1] ?? "").trim();
}

// ---------------------------------------------------------------------
// HTML to text — the shape `scripts/build-llms-full.mjs` established
// ---------------------------------------------------------------------

const ENTITIES = {
  "&amp;": "&",
  "&lt;": "<",
  "&gt;": ">",
  "&quot;": '"',
  "&#39;": "'",
  "&nbsp;": " ",
  "&mdash;": "—",
  "&ndash;": "–",
  "&hellip;": "…",
};

function decodeEntities(text) {
  return text
    .replace(/&#(\d+);/g, (_, code) => String.fromCodePoint(Number(code)))
    .replace(/&#x([0-9a-f]+);/gi, (_, code) =>
      String.fromCodePoint(Number.parseInt(code, 16)),
    )
    .replace(/&[a-z]+;/gi, (entity) => ENTITIES[entity] ?? entity);
}

/**
 * A built page as plain text, headings kept as Markdown.
 *
 * The order of the replacements is the whole algorithm: comments and the
 * script, style and svg elements go first — including the serialised
 * state Qwik appends, which is a page's worth of JSON that means nothing
 * to a reader — then the headings become `#` lines while their tags are
 * still there to recognise, then everything else that is a tag is
 * dropped and the whitespace is collapsed.
 */
function htmlToText(html) {
  let text = html;
  text = text.replace(/<!--[\s\S]*?-->/g, "");
  text = text.replace(/<(script|style|svg|noscript)\b[\s\S]*?<\/\1>/gi, "");
  text = text.replace(/<h1[^>]*>([\s\S]*?)<\/h1>/gi, "\n\n# $1\n\n");
  text = text.replace(/<h2[^>]*>([\s\S]*?)<\/h2>/gi, "\n\n## $1\n\n");
  text = text.replace(/<h3[^>]*>([\s\S]*?)<\/h3>/gi, "\n\n### $1\n\n");
  text = text.replace(/<\/(p|div|section|li|footer|header|main)>/gi, "\n\n");
  text = text.replace(/<[^>]+>/g, "");
  text = decodeEntities(text)
    .replace(/[ \t]+/g, " ")
    .replace(/\n[ \t]+/g, "\n")
    .replace(/\n{3,}/g, "\n\n");
  return text.trim();
}

// ---------------------------------------------------------------------
// The files
// ---------------------------------------------------------------------

function robotsTxt(config) {
  const lines = [
    "# vibevm.org robots.txt",
    "# Open for indexing, including by AI crawlers.",
    "# Machine-readable indexes: /llms.txt, /llms-full.txt, /sitemap.xml",
    "# ASCII-only on purpose: some crawlers parse robots.txt as Latin-1.",
    `# Agent names verified against the providers' own documentation on ${CRAWLERS_VERIFIED_ON}.`,
    "",
  ];
  for (const agent of CRAWLERS) {
    lines.push(`User-agent: ${agent}`, "Allow: /", "");
  }
  lines.push("User-agent: *", "Allow: /", "");
  lines.push(`Sitemap: ${config.origin}/sitemap.xml`);
  lines.push(`Sitemap: ${config.origin}${DOC_SITEMAP}`);
  const text = `${lines.join("\n")}\n`;
  const offender = [...text].find((ch) => ch.codePointAt(0) > 0x7f);
  if (offender !== undefined) {
    throw new Error(
      `robots.txt must stay ASCII-only and contains ${JSON.stringify(offender)}`,
    );
  }
  return text;
}

function sitemapXml(config, pages) {
  const landing = landingPages(pages);
  const entry = (address, priority) =>
    [
      "  <url>",
      `    <loc>${config.origin}${address}</loc>`,
      `    <lastmod>${config.lastmod}</lastmod>`,
      "    <changefreq>weekly</changefreq>",
      `    <priority>${priority}</priority>`,
      "  </url>",
    ].join("\n");

  const body = [
    /* The root is the strongest address the site has, and the Russian
       tree is the one submitted to Yandex: the two priorities are the
       Astro sitemap's, carried over rather than recomputed. */
    ...landing.map((page) =>
      entry(page.address, page.address === "/" ? "1" : "0.9"),
    ),
    /* The documentation's own addresses are not here: they are in
       `/doc/sitemap.xml`, an index by package and language written from
       the page manifests (`##SEO-SITEMAP`), which is the file that knows
       which of them are `latest` and which are translation fallbacks.
       What stands here is the record of that file, so the root sitemap
       names the whole domain even for a crawler that reads it without
       reading `robots.txt` — where the second `Sitemap:` line is the
       proper declaration. A url set cannot hold a sitemap reference, so
       the record is an ordinary entry; it costs one fetch of a file that
       is the point of the fetch. */
    entry(DOC_SITEMAP, "0.8"),
  ];

  return [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
    ...body,
    "</urlset>",
    "",
  ].join("\n");
}

/**
 * The feed, with the landing's own publication date kept.
 *
 * `lastBuildDate` moves with the build, because that is what it means.
 * `pubDate` does not: the two entries were published once, and a feed
 * that re-dates them on every deploy asks every reader's client to
 * announce them again.
 */
const PUBLISHED_AT = "Thu, 16 Jul 2026 12:00:00 GMT";

function feedXml(config, pages, heads) {
  const landing = landingPages(pages);
  const items = landing.map((page) => {
    const head = heads.get(page.address);
    return [
      "    <item>",
      `      <title>${escapeXml(head.title)}</title>`,
      `      <link>${config.origin}${page.address}</link>`,
      `      <description>${escapeXml(head.description)}</description>`,
      `      <guid>${config.origin}${page.address}</guid>`,
      `      <pubDate>${PUBLISHED_AT}</pubDate>`,
      "    </item>",
    ].join("\n");
  });

  const english = heads.get("/");
  return [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">',
    "  <channel>",
    "    <title>VibeVM</title>",
    `    <link>${config.origin}</link>`,
    `    <description>${escapeXml(english.description)}</description>`,
    "    <language>en</language>",
    `    <lastBuildDate>${new Date().toUTCString()}</lastBuildDate>`,
    `    <atom:link href="${config.origin}/feed.xml" rel="self" type="application/rss+xml"/>`,
    ...items,
    "  </channel>",
    "</rss>",
    "",
  ].join("\n");
}

function escapeXml(text) {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

/**
 * The short index for language models.
 *
 * The disambiguation paragraph is the owner's, word for word: several
 * unrelated projects reuse the name, and this paragraph is the site's
 * only chance to say which one it is to a reader that will never see the
 * page. The sections are the ones the Astro file had, with one line
 * added — the documentation now has an index of its own under `/doc/`,
 * and a root index that did not point at it would send an agent to read
 * the landing twice (`##SITE-ONE-SITE`).
 */
function llmsTxt(config) {
  const origin = config.origin;
  return `${[
    "# VibeVM",
    "",
    "> An ultimate prompt library, package manager, and agentic system for Spec-Driven Development. VibeVM installs specs, flows, and skills as versioned dependencies and assembles them into the declarative context an AI coding agent reads at session boot.",
    "",
    'Disambiguation: several unrelated projects reuse the "vibevm" name. The links below are the authoritative ones for this VibeVM.',
    "",
    "## Project",
    "",
    `- [Official site](${origin}): VibeVM — spec-driven development, packaged.`,
    "- [GitHub (canonical source)](https://github.com/vibevm/vibevm): the CLI and package ecosystem.",
    "- [GitVerse (source mirror)](https://gitverse.ru/vibevm/vibevm): source repository on GitVerse.",
    "",
    "## What it is",
    "",
    "- **Prompt library** — addressable, reusable prompts and specifications, cited by URI rather than paraphrase.",
    "- **Package manager** — install specs, flows, and skills as versioned, lockfile-pinned dependencies resolved through a decentralized registry.",
    "- **Agentic system** — any coding agent boots from a computed, spec-driven context assembled at the start of a session.",
    "",
    "## Install",
    "",
    `- Linux, macOS, and WSL: \`curl -fsSL ${origin}/install.sh | bash\``,
    `- Windows PowerShell: \`irm ${origin}/install.ps1 | iex\``,
    "",
    "The native installer installs `vibe`, the on-premises `vibe-index` service, and the matching VibeVM source tree from the same verified release.",
    "",
    "## Machine-readable",
    "",
    `- [Full text](${origin}/llms-full.txt): every page of this site as plain text.`,
    `- [Documentation index](${origin}/doc/llms.txt): the manual and the packages it documents.`,
    `- [Sitemap](${origin}/sitemap.xml)`,
    `- [RSS feed](${origin}/feed.xml)`,
  ].join("\n")}\n`;
}

/**
 * Every landing page as one plain-text document.
 *
 * The landing and not the documentation: the root `llms-full.txt` is
 * this site's front door in text, and the manual publishes its own under
 * `/doc/` from the page manifest rather than from rendered HTML
 * (`##SITE-ONE-SITE`). The 404 is dropped, as it was; `/en/` is dropped
 * because it is a redirect with nothing to read.
 */
function llmsFullTxt(config, pages) {
  const header = `# VibeVM — vibevm.org (full text)

> An ultimate prompt library, package manager, and agentic system for Spec-Driven Development.
> Declarative context assembled for AI agents from versioned stacks, flows, and skills.
>
> Authoritative links (several unrelated projects reuse the "vibevm" name — these are canonical):
> - Official site: ${config.origin}
> - GitHub: https://github.com/vibevm/vibevm
> - GitVerse: https://gitverse.ru/vibevm/vibevm
>
> Short index: ${config.origin}/llms.txt · Spec: https://llmstxt.org/

---
`;

  let body = "";
  for (const page of landingPages(pages)) {
    const title = titleOf(page.html) || page.address;
    body += `\n\n---\n## URL: ${config.origin}${page.address}\n## Title: ${title}\n---\n\n${htmlToText(page.html)}\n`;
  }
  return `${header}${body}\n`;
}

// ---------------------------------------------------------------------
// Fonts: the public paths, and the preloads that must not double them
// ---------------------------------------------------------------------

/**
 * Copy the faces to `/fonts/<face>.woff2`, the addresses the domain has
 * served since the Astro site, and report where the bundle actually put
 * each one.
 *
 * Two answers for one file, and both are wanted. The stylesheet asks for
 * the content-hashed asset the bundler emitted, which is what makes a
 * year-long immutable cache safe. The public path is the one that
 * external links, the nginx `/fonts/` rule and anything holding an older
 * URL still use. The bytes are copied rather than linked because a
 * symlink in a container image is a deployment question, and 476 kB is
 * cheaper than that conversation.
 */
function publishFonts(outDir) {
  const target = join(outDir, PUBLIC_FONT_DIR);
  mkdirSync(target, { recursive: true });
  const faces = readdirSync(FONTS_DIR).filter((name) =>
    name.endsWith(".woff2"),
  );
  for (const face of faces) {
    copyFileSync(join(FONTS_DIR, face), join(target, face));
  }

  const assets = join(outDir, "assets");
  const bundled = new Map();
  if (existsSync(assets)) {
    for (const name of readdirSync(assets)) {
      if (!name.endsWith(".woff2")) continue;
      const face = faces.find((candidate) => name.endsWith(`-${candidate}`));
      if (face !== undefined) bundled.set(face, `/assets/${name}`);
    }
  }
  return { faces, bundled };
}

/**
 * Point each `<link rel="preload">` at the file the stylesheet will ask
 * for.
 *
 * A preload of `/fonts/Inter-latin.woff2` next to a `@font-face` naming
 * `/assets/<hash>-Inter-latin.woff2` is not a preload — it is a second
 * download of the same face, and the browser has no way to know. The
 * page declares the public path because that is what the page means; the
 * build rewrites it to the built path because that is what the browser
 * needs. Nothing else in the HTML is touched.
 */
function rewriteFontPreloads(html, bundled) {
  return html.replace(
    /(<link\b[^>]*\brel="preload"[^>]*\bhref=")\/fonts\/([^"]+)(")/g,
    (whole, before, face, after) => {
      const built = bundled.get(face);
      return built === undefined ? whole : `${before}${built}${after}`;
    },
  );
}

// ---------------------------------------------------------------------
// The entry point
// ---------------------------------------------------------------------

/** What a page's head says, for the feed and for the report. */
function headOf(html) {
  const description = /<meta\s+name="description"\s+content="([^"]*)"/i.exec(
    html,
  );
  return {
    title: titleOf(html),
    description: decodeEntities(description?.[1] ?? ""),
  };
}

export function writeRootFiles(outDirName = "dist") {
  const outDir = join(SITE_ROOT, outDirName);
  if (!existsSync(outDir)) {
    throw new Error(`root-files: ${outDirName} does not exist — build first`);
  }
  const config = siteConfig(process.env);

  const pages = htmlFiles(outDir).map((file) => ({
    file,
    address: addressOf(outDir, file),
    html: readFileSync(file, "utf8"),
  }));

  const heads = new Map(pages.map((page) => [page.address, headOf(page.html)]));

  const { faces, bundled } = publishFonts(outDir);
  let rewrittenPages = 0;
  for (const page of pages) {
    const rewritten = rewriteFontPreloads(page.html, bundled);
    if (rewritten !== page.html) {
      writeFileSync(page.file, rewritten, "utf8");
      page.html = rewritten;
      rewrittenPages += 1;
    }
  }

  const written = [];
  const write = (name, contents) => {
    writeFileSync(join(outDir, name), contents, "utf8");
    written.push(name);
  };

  write("robots.txt", robotsTxt(config));
  write("llms.txt", llmsTxt(config));
  write("llms-full.txt", llmsFullTxt(config, pages));
  write("sitemap.xml", sitemapXml(config, pages));
  write("feed.xml", feedXml(config, pages, heads));

  /* The key file IS the key: its name is the value and so are its
     contents, which is how IndexNow verifies that whoever submits URLs
     for this domain controls it (`##SEO-INDEXNOW`). A key comes from the
     deployment's environment and has no default, so a build that was not
     given one writes no file rather than a file that would fail
     verification. */
  if (config.indexNowKey.length > 0) {
    write(`${config.indexNowKey}.txt`, config.indexNowKey);
  }

  written.push(writeOgCard(join(outDir, "og.png"), PACKAGE_ROOT));

  return {
    written,
    faces: faces.length,
    rewritten: rewrittenPages,
    crawlers: CRAWLERS.length,
    sources: SOURCES.length,
    unindexed: pruneSitemaps(outDir),
  };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const result = writeRootFiles(process.argv[2] ?? "dist");
  process.stdout.write(
    `root-files: wrote ${result.written.join(", ")}; ${result.faces} font file(s) published, ${result.preloaded} preload(s) pointed at the bundle\n`,
  );
}
