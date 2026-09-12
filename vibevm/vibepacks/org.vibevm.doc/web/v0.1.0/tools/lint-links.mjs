#!/usr/bin/env node
// `node tools/lint-links.mjs [outDir]` — every address the built site
// writes, checked against the files it wrote.
//
// It exists because none of the other gates can see this. The type
// checker proves that a link is a string; the page-count gate proves the
// pages were generated; the parity test compares the landing against the
// site it replaced. Nobody looks at whether `rel=canonical` names a page
// that exists, whether the `hreflang` of a page in Russian is answered by
// the same annotation coming back, or whether the `.md` an agent is
// offered beside a page was actually copied there. A site can be green on
// every other gate and still send every crawler that reads it to a 404.
//
// Four questions, and a red answer to any of them fails the build:
//
//   1. does every internal link name a file the build wrote?
//   2. is every `hreflang` pair mutual — does the page it names name it
//      back (Google discards a one-sided annotation silently)?
//   3. does every `rel=canonical` name a page that exists?
//   4. is every external address one of the handful the site is allowed
//      to name, and is nothing fetched from any of them (R-09)?
//
// Three kinds of miss are NOT failures, and each is named rather than
// waved through. A `/doc/` link into a package this site does not carry
// is a citation leaving the library — the address map is deterministic,
// so a page can cite documentation nobody has published here, and that is
// a fact about the citation graph rather than a broken link. A link
// written INSIDE the island is the pipeline's writing and not the site's:
// `vibe doc build` resolved it from the package's own source and the site
// publishes those bytes unchanged, so a form the address map does not
// carry is a finding about the renderer — counted and printed by shape,
// because the site cannot rewrite it without becoming a second renderer,
// and filing it in the table below would record a live defect as a
// deliberate one. And the EXCEPTIONS table carries the misses that are
// known, filed and deliberate; anything not in it is red.

import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

import { siteConfig } from "../site/src/config.ts";

const PACKAGE_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const SITE_ROOT = join(PACKAGE_ROOT, "site");

/**
 * The hosts a page of this site may name, and why each one is there.
 *
 * It is the list the landing already carried into the port, checked
 * against the Astro build it replaced: two source mirrors, two
 * vocabularies that are identifiers rather than resources, the address
 * of the `llms.txt` specification, and one string inside a framework
 * chunk that nobody requests (X-020). A host that is not here is a
 * finding, whether or not anything is fetched from it.
 */
const ALLOWED_HOSTS = new Map([
  ["github.com", "the canonical source repository, linked by the landing"],
  ["gitverse.ru", "the source mirror, linked by the landing"],
  [
    "schema.org",
    "the JSON-LD @context: a vocabulary identifier, never fetched",
  ],
  ["www.w3.org", "an XML namespace name, never fetched"],
  ["www.sitemaps.org", "the sitemap namespace name, never fetched"],
  ["llmstxt.org", "the llms.txt specification, linked in llms.txt as it was"],
  [
    "qwikdev-build-v2.qwik-8nx.pages.dev",
    "a string inside a framework error message (X-020); nothing requests it",
  ],
  [
    "localhost",
    "the router's fallback base when no URL is passed; not a request",
  ],
]);

/**
 * Misses that are known, filed, and not this build's to fix.
 *
 * The shape of a parity difference and for the same reason: a gate that
 * cannot be quieted is a gate somebody turns off, and one that can be
 * quieted anonymously is a gate that stops meaning anything. Each entry
 * says what it covers and why it is not a defect of this site.
 */
const EXCEPTIONS = [
  {
    id: "L-01",
    reason:
      "The island golden cites `media/diagram.svg` beside the DOCUMENT, and the pipeline publishes a package's media at the root of its tree under a content name — so the address misses at whatever depth the page is served, and the fixture package carries no such file in either place. SVG is not an allowed medium in this wave (D-20-6). Filed as an island-golden finding; the picture is the only broken one on the fixture page.",
    matches: (miss) => miss.target.endsWith("/media/diagram.svg"),
  },
];

/** Every file under a directory, as site-absolute addresses. */
function filesUnder(dir, root = dir, found = new Set()) {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) filesUnder(full, root, found);
    else
      found.add(
        `/${full
          .slice(root.length + 1)
          .split(sep)
          .join("/")}`,
      );
  }
  return found;
}

/** The public address of a built file: a directory index is its directory. */
function addressOf(file) {
  return file.endsWith("/index.html")
    ? file.slice(0, -"index.html".length)
    : file;
}

/** The file an address asks for: a slash-ended address is a directory index. */
function fileFor(address) {
  return address.endsWith("/") ? `${address}index.html` : address;
}

/** Resolve a link written on a page at `from` into a site address. */
function resolveLink(from, link) {
  const cut = link.replace(/[?#].*$/, "");
  if (cut.length === 0) return null;
  if (/^[a-z][a-z0-9+.-]*:/i.test(cut)) return null;
  if (cut.startsWith("//")) return null;
  if (cut.startsWith("/")) return decodeURI(cut);
  const base = from.endsWith("/") ? from : `${dirname(from)}/`;
  return decodeURI(new URL(cut, `https://site.invalid${base}`).pathname);
}

/**
 * The kinds of address that the site FETCHES rather than merely names.
 *
 * The distinction is the whole of R-09. «The public site does not load
 * scripts from foreign CDNs» is about what a browser goes and gets while
 * rendering a page — a script, a stylesheet, a font, a picture, a card
 * for a share. It is not about what a page LINKS to: a documentation
 * page may cite an RFC, a standard or a repository, and a gate that
 * refused those would be asking the prose to pretend the rest of the web
 * does not exist.
 *
 * So a foreign host is a failure here and a counted note anywhere else.
 */
const FETCHING = new Set([
  "src",
  "og:image",
  "twitter:image",
  "og:url",
  "canonical",
  "sitemap loc",
  "robots sitemap",
  "resolver entry",
  "link:stylesheet",
  "link:preload",
  "link:modulepreload",
  "link:prefetch",
  "link:preconnect",
  "link:dns-prefetch",
  "link:icon",
  "link:manifest",
]);

/**
 * Where the rendered page sits inside the document.
 *
 * The island is one `<article class="doc-page">` written by `vibe doc
 * build` and put into the hole the shell left; everything outside it is
 * the site's own markup. Which of the two wrote a link is the difference
 * between a build that must stop and a number somebody has to act on
 * somewhere else, so the region is measured once per page and every link
 * is asked whether it falls inside it.
 *
 * `null` when the page carries no island — a catalogue, a package page,
 * the landing.
 */
function islandRegion(html) {
  const marker = html.indexOf("data-island");
  if (marker === -1) return null;
  const opens = html.indexOf(">", marker);
  if (opens === -1) return null;
  const closes = html.indexOf("</article>", opens);
  if (closes === -1) return null;
  return { from: opens, to: closes + "</article>".length };
}

/** The links one HTML page writes, with what wrote each of them. */
function linksIn(html) {
  const island = islandRegion(html);
  const inside = (at) => island !== null && at >= island.from && at < island.to;
  const found = [];
  for (const match of html.matchAll(/<link\b[^>]*>/g)) {
    const tag = match[0];
    const rel = /\brel="([^"]*)"/.exec(tag);
    const at = /\bhref="([^"]*)"/.exec(tag);
    if (at === null) continue;
    found.push({
      kind: `link:${(rel?.[1] ?? "").toLowerCase()}`,
      target: at[1] ?? "",
      island: inside(match.index),
    });
  }
  for (const match of html.matchAll(/<(a|img|script|source|iframe)\b[^>]*>/g)) {
    const tag = match[0];
    const at = /\b(href|src)="([^"]*)"/.exec(tag);
    if (at === null) continue;
    found.push({
      kind: at[1] ?? "",
      target: at[2] ?? "",
      island: inside(match.index),
    });
  }
  for (const match of html.matchAll(
    /<meta[^>]+(?:property|name)="(og:image|og:url|twitter:image)"[^>]+content="([^"]*)"/g,
  )) {
    found.push({ kind: match[1] ?? "", target: match[2] ?? "", island: false });
  }
  return found;
}

/** The `hreflang` annotations of one page. */
function alternatesIn(html) {
  const found = [];
  for (const match of html.matchAll(/<link\b[^>]*>/g)) {
    const tag = match[0];
    if (!/\brel="alternate"/.test(tag)) continue;
    const lang = /\bhreflang="([^"]*)"/.exec(tag);
    const at = /\bhref="([^"]*)"/.exec(tag);
    if (lang === null || at === null) continue;
    found.push({ hreflang: lang[1] ?? "", target: at[1] ?? "" });
  }
  return found;
}

/** The canonical of one page, or nothing. */
function canonicalIn(html) {
  for (const match of html.matchAll(/<link\b[^>]*>/g)) {
    const tag = match[0];
    if (!/\brel="canonical"/.test(tag)) continue;
    const at = /\bhref="([^"]*)"/.exec(tag);
    if (at !== null) return at[1] ?? "";
  }
  return null;
}

export function lintLinks(outDirName = "dist", write = process.stdout) {
  const outDir = join(SITE_ROOT, outDirName);
  if (!existsSync(outDir)) {
    throw new Error(`lint-links: ${outDirName} does not exist — build first`);
  }
  const config = siteConfig(process.env);
  const origin = config.origin;
  const files = filesUnder(outDir);
  const addresses = new Set([...files].map((file) => addressOf(file)));
  const say = (line) => write.write(`${line}\n`);

  const failures = [];
  const excused = new Map();
  const citations = new Set();
  /** Island links the address map does not carry, by the name they end in. */
  const islandForms = new Map();
  const foreign = new Map();
  const outbound = new Map();
  let checked = 0;

  /** An address the site claims to serve: is there a file behind it? */
  const carried = (address) =>
    files.has(fileFor(address)) || addresses.has(address);

  /** The coordinates the site actually carries pages for. */
  const coordinates = new Set(
    [...addresses]
      .map((address) =>
        /^\/doc\/(?:[a-z]{2,3}\/)?([^/]+\.[^/]+)\/([^/]+)\//.exec(address),
      )
      .filter((match) => match !== null)
      .map((match) => `${match[1]}/${match[2]}`),
  );

  const miss = (page, kind, target, island) => {
    const entry = { page, kind, target };
    const excuse = EXCEPTIONS.find((rule) => rule.matches(entry));
    if (excuse !== undefined) {
      excused.set(excuse.id, (excused.get(excuse.id) ?? 0) + 1);
      return;
    }
    const citation = /^\/doc\/(?:[a-z]{2,3}\/)?([^/]+\.[^/]+)\/([^/]+)\//.exec(
      target,
    );
    if (
      citation !== null &&
      !coordinates.has(`${citation[1]}/${citation[2]}`)
    ) {
      citations.add(target);
      return;
    }
    /* Inside the island the site is not the author. `vibe doc build`
       wrote this link out of the package's own source and the site
       publishes the bytes unchanged (`##PIPE-ONE-CONTENT-PATH`), so a
       form the address map does not carry is a fact about the renderer
       and is counted by the shape it takes — a page's sibling as
       `<document>/index.xml`, say, where the site serves `<document>/`
       and `<document>.xml`. Rewriting it here would make the site a
       second renderer; failing on it would stop a build over
       documentation the site renders correctly. */
    if (island === true) {
      const name =
        target
          .split("/")
          .filter((one) => one.length > 0)
          .pop() ?? target;
      islandForms.set(name, (islandForms.get(name) ?? 0) + 1);
      return;
    }
    failures.push(`${page}: ${kind} -> ${target} is not in the output`);
  };

  /** An address written as an absolute URL: internal, allowed, or a finding. */
  const external = (page, kind, target) => {
    let url;
    try {
      url = new URL(target);
    } catch {
      return null;
    }
    if (`${url.protocol}//${url.host}` === origin) return url.pathname;
    const why = ALLOWED_HOSTS.get(url.hostname);
    if (why === undefined) {
      if (!FETCHING.has(kind)) {
        outbound.set(url.hostname, (outbound.get(url.hostname) ?? 0) + 1);
        return null;
      }
      failures.push(
        `${page}: ${kind} would fetch from ${url.hostname}, which is not allowed`,
      );
      return null;
    }
    foreign.set(url.hostname, (foreign.get(url.hostname) ?? 0) + 1);
    return null;
  };

  const check = (page, kind, target, island = false) => {
    if (target.length === 0) return;
    if (/^(mailto|data|javascript|spec):/i.test(target)) return;
    checked += 1;
    if (/^https?:\/\//i.test(target)) {
      const inside = external(page, kind, target);
      if (inside === null) return;
      if (!carried(decodeURI(inside.replace(/[?#].*$/, "")))) {
        miss(page, kind, decodeURI(inside), island);
      }
      return;
    }
    const address = resolveLink(page, target);
    if (address === null) return;
    if (!carried(address)) miss(page, kind, address, island);
  };

  // 1. Every page: its links, its canonical, its annotations.
  const pages = [...files].filter((file) => file.endsWith(".html"));
  const alternates = new Map();
  const duplicates = new Set();
  for (const file of pages) {
    const page = addressOf(file);
    const html = readFileSync(
      join(outDir, file.slice(1).split("/").join(sep)),
      "utf8",
    );
    for (const link of linksIn(html)) {
      check(page, link.kind, link.target, link.island);
    }
    const canonical = canonicalIn(html);
    if (canonical !== null) check(page, "canonical", canonical);
    const mine = alternatesIn(html);
    if (mine.length > 0) {
      alternates.set(
        page,
        mine.map((one) => ({
          hreflang: one.hreflang,
          address: /^https?:\/\//i.test(one.target)
            ? new URL(one.target).pathname
            : (resolveLink(page, one.target) ?? ""),
        })),
      );
    }
    if (canonical !== null) {
      const at = /^https?:\/\//i.test(canonical)
        ? new URL(canonical).pathname
        : (resolveLink(page, canonical) ?? "");
      if (at !== page) duplicates.add(page);
    }
  }

  // 2. Every annotation, answered from the other side.
  //
  //    Only between pages that are their own canonical. A page that
  //    names another page canonical has said it is not the one to keep —
  //    a numbered address beside its `latest`, the 404 beside the root —
  //    and search engines read its annotations as the canonical's or not
  //    at all. Demanding reciprocity from a duplicate would demand that
  //    the canonical name every duplicate back, which is the opposite of
  //    what a canonical says.
  let pairs = 0;
  for (const [page, mine] of alternates) {
    if (duplicates.has(page)) continue;
    for (const one of mine) {
      if (one.hreflang === "x-default") continue;
      pairs += 1;
      const theirs = alternates.get(one.address);
      if (theirs === undefined) {
        failures.push(
          `${page}: hreflang ${one.hreflang} -> ${one.address} carries no annotations of its own`,
        );
        continue;
      }
      if (!theirs.some((back) => back.address === page)) {
        failures.push(
          `${page}: hreflang ${one.hreflang} -> ${one.address} does not name it back`,
        );
      }
    }
  }

  // 3. The machine files: a sitemap's addresses, and the links in the
  //    indexes an agent reads. They are the site's own writing too, and
  //    a sitemap naming a page nobody wrote is the classic way to spend
  //    a crawler's budget on nothing.
  for (const file of [...files].filter(
    (one) =>
      one.endsWith(".xml") || one.endsWith(".txt") || one.endsWith(".json"),
  )) {
    if (file.endsWith("csp.txt")) continue;
    const text = readFileSync(
      join(outDir, file.slice(1).split("/").join(sep)),
      "utf8",
    );
    for (const match of text.matchAll(/<loc>([^<]+)<\/loc>/g)) {
      check(file, "sitemap loc", match[1] ?? "");
    }
    for (const match of text.matchAll(/\]\((https?:\/\/[^)\s]+)\)/g)) {
      check(file, "index link", match[1] ?? "");
    }
    for (const match of text.matchAll(/^Sitemap:\s*(\S+)$/gm)) {
      check(file, "robots sitemap", match[1] ?? "");
    }
    for (const match of text.matchAll(/"href":\s*"([^"]+)"/g)) {
      check(file, "resolver entry", match[1] ?? "");
    }
  }

  say("documentation links — every address against the files behind it");
  say(
    `    ok        ${pages.length} page(s), ${files.size} file(s) in the output`,
  );
  say(
    `    ok        ${checked} link(s) followed, ${pairs} hreflang pair(s) checked`,
  );
  say(
    `    ok        ${duplicates.size} page(s) name another page canonical and are read as it`,
  );
  for (const [host, count] of [...foreign].sort()) {
    say(`    ok        ${count}x ${host} — ${ALLOWED_HOSTS.get(host)}`);
  }
  if (outbound.size > 0) {
    const total = [...outbound.values()].reduce((sum, one) => sum + one, 0);
    const named = [...outbound]
      .sort((a, b) => b[1] - a[1])
      .slice(0, 5)
      .map(([host, count]) => `${host} (${count})`)
      .join(", ");
    say(
      `    ok        ${total} outbound link(s) to ${outbound.size} host(s) the prose cites, fetched by nothing: ${named}`,
    );
  }
  if (islandForms.size > 0) {
    const total = [...islandForms.values()].reduce((sum, one) => sum + one, 0);
    const named = [...islandForms]
      .sort((a, b) => b[1] - a[1])
      .slice(0, 5)
      .map(([name, count]) => `${name} (${count})`)
      .join(", ");
    say(
      `    note      ${total} link(s) the pipeline wrote inside an island in a form this site does not carry: ${named}`,
    );
  }
  if (citations.size > 0) {
    say(
      `    note      ${citations.size} citation(s) into documentation this site does not carry:`,
    );
    for (const target of [...citations].sort().slice(0, 5)) {
      say(`              ${target}`);
    }
  }
  for (const [id, count] of [...excused].sort()) {
    const rule = EXCEPTIONS.find((one) => one.id === id);
    say(`    ${id}      ${count}x — ${rule?.reason ?? ""}`);
  }
  for (const line of failures) say(`    FAIL      ${line}`);

  say(
    failures.length === 0
      ? `links: green — ${checked} checked, 0 broken.`
      : `links: RED — ${failures.length} broken.`,
  );
  return failures.length === 0 ? 0 : 1;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  process.exit(lintLinks(process.argv[2] ?? "dist"));
}
