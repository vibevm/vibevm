#!/usr/bin/env node
// `node tools/parity.mjs <astro-dist> [qwik-dist]` — the gate that says
// whether the landing was moved or merely rebuilt.
//
// A port is easy to judge by eye and easy to get wrong in the half a
// reader never sees. The words can all be there while `canonical` points
// at the wrong host, a `hreflang` is missing, the structured data lost
// its second graph or the Russian page quietly serves English metadata —
// and the page would still look finished. So the two builds are compared
// tag by tag, file by file, address by address, and every difference has
// to be either gone or named.
//
// The naming is the contract (F-73): each expected difference is a rule
// in DIFFERENCES below, with the reason it exists. A difference no rule
// covers is unexplained, and an unexplained difference fails the run. A
// green run is the gate for pointing the domain at the new build (A5.6).
//
// The reference is built from a scratch copy of the Astro repository —
// `npm ci && npm run build && node scripts/build-llms-full.mjs` — never
// from the repository itself, which this campaign only reads (R-28).
//
// No dependencies: the comparisons are string work over built HTML, and
// a parser in the gate would be one more thing to keep pinned.

import { readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

import { isBuilderState } from "./out-dir.mjs";

const PACKAGE_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");

/**
 * The differences that are decisions, each with the decision behind it.
 *
 * `where` says which check may cite the rule; `matches` decides whether
 * a particular observed difference is this one. Nothing here excuses a
 * class of differences wholesale: every rule names a shape, and anything
 * outside those shapes is reported as unexplained.
 */
const DIFFERENCES = [
  {
    id: "D-01",
    where: "addresses",
    reason:
      "`/en/` is new. The Astro deployment answered the legacy English prefix with a 301 in nginx; a static build cannot return a status code, so the address is now a page that redirects itself (noindex, canonical on the root).",
    matches: (address) => address === "/en/",
  },
  {
    id: "D-02",
    where: "addresses",
    reason:
      "`/doc/…` is new: the documentation now lives on the same domain, in the same build, from the same components (D-28, ##SITE-ONE-SITE). The whole point of the move.",
    matches: (address) => address.startsWith("/doc/"),
  },
  {
    id: "D-03",
    where: "addresses",
    reason:
      "`/og.png` is new. Four meta tags on both Astro pages pointed at it and the file existed nowhere in that tree (A0.26): every share of the landing fell back to whatever the platform chose. The build now draws the card.",
    matches: (address) => address === "/og.png",
  },
  {
    id: "D-15",
    where: "addresses",
    reason:
      "`/csp.txt` is new: the policy line, with the `sha256` of every inline script in the output, written from the bytes that were built rather than typed anywhere (X-035). It is an input to the deployment atom, which serves it as a header; nothing on the site fetches it.",
    matches: (address) => address === "/csp.txt",
  },
  {
    id: "D-04",
    where: "addresses",
    reason:
      "The bundler's own output moved: `/_astro/index.<hash>.css` became `/assets/…` and `/build/…`. The stylesheet is inlined into the page and the fonts are content-hashed beside it; the public `/fonts/*.woff2` addresses are still served, and the preload links point at whichever copy the stylesheet actually fetches.",
    matches: (address) =>
      address.startsWith("/assets/") ||
      address.startsWith("/build/") ||
      address.startsWith("/_astro/"),
  },
  {
    id: "D-05",
    where: "addresses",
    reason:
      "`/manifest.json` is the shell's web-app manifest, rewritten without the starter's foreign `$schema` (##STACK-BUILD-HYGIENE).",
    matches: (address) => address === "/manifest.json",
  },
  {
    id: "D-06",
    where: "addresses",
    reason:
      "The IndexNow key file is absent unless the build environment names the key. The key identifies a live property and belongs to the deployment, not to the repository (##SEO-INDEXNOW); a build told nothing writes no file rather than one that would fail the protocol's own verification.",
    matches: (address) => /^\/[0-9a-f]{32}\.txt$/.test(address),
  },
  {
    id: "D-07",
    where: "robots.txt",
    reason:
      "A second `Sitemap:` line, for `/doc/sitemap.xml`. One robots.txt per domain, and the domain now carries two sitemaps (##SITE-ONE-SITE).",
    matches: (line) => /^Sitemap: \S+\/doc\/sitemap\.xml$/.test(line),
  },
  {
    id: "D-08",
    where: "robots.txt",
    reason:
      "Agent names read from the providers' own documentation on the build day and never from memory (F-38, ##SEO-ROBOTS). Dropped: `Claude-Web` and `anthropic-ai`, which Anthropic no longer documents. Added: the documented siblings of names already on the list — `Claude-User`, `Claude-SearchBot`, `Perplexity-User`, `Amzn-SearchBot`, `Amzn-User`, `Meta-ExternalFetcher`. Every stanza still allows everything.",
    matches: (line) =>
      /^User-agent: (Claude-Web|anthropic-ai|Claude-User|Claude-SearchBot|Perplexity-User|Amzn-SearchBot|Amzn-User|Meta-ExternalFetcher)$/.test(
        line,
      ),
  },
  {
    id: "D-09",
    where: "robots.txt",
    reason:
      "One comment line recording the date the names were verified, so the file itself says when it was last checked.",
    matches: (line) => line.startsWith("# Agent names verified against"),
  },
  {
    id: "D-10",
    where: "text",
    reason:
      "The header gains one entry — «Documentation» / «Документация», pointing at `/doc/`. The landing and the manual are one site now, and the way in has to be on the page (D-28).",
    matches: (fragment) =>
      fragment === "Documentation" || fragment === "Документация",
  },
  {
    id: "D-16",
    where: "text",
    reason:
      "The header gains the search box the documentation's header carries: the field's visually hidden label and the `Ctrl K` pill beside it. One site, one way to look for a page (D-28) — and the box is one widget used by both halves rather than two that drift. The theme switch beside it adds no text at all: its three states are drawn marks with names in `aria-label`.",
    matches: (fragment) =>
      fragment === "Search the documentation" ||
      fragment === "Искать в документации" ||
      fragment === "Ctrl K",
  },
  {
    id: "D-11",
    where: "meta",
    reason:
      "`theme-color` is the same colour in lower case. The value is no longer typed into the layout: it is read out of `palette.css`, the one file allowed to write a colour down (R-27), where prettier normalises hex to lower case.",
    matches: (difference) =>
      difference.name === "theme-color" &&
      difference.reference.toLowerCase() === difference.port.toLowerCase(),
  },
  {
    id: "D-12",
    where: "umami",
    reason:
      "The analytics tag is absent unless the build environment names a website id (##SITE-ANALYTICS, D-24). The id identifies a live property; a tag carrying an empty one would load the script and report to nobody. The owner moves the value into the site configuration in A5.1.",
    matches: (observation) => observation === "absent-without-id",
  },
  {
    id: "D-13",
    where: "sitemap",
    reason:
      "`lastmod` is the build date rather than a date frozen in a hand-written file, and the documentation's addresses are listed beside the landing's at a lower priority.",
    matches: (note) => note === "lastmod" || note.startsWith("/doc/"),
  },
  {
    id: "D-14",
    where: "llms.txt",
    reason:
      "One line added, pointing at `/doc/llms.txt`: a root index that did not name the documentation's own index would send an agent to read the landing twice.",
    matches: (line) => line.includes("/doc/llms.txt"),
  },
];

function rulesFor(where) {
  return DIFFERENCES.filter((rule) => rule.where === where);
}

/** Match an observation against the rules for a check; `null` if none. */
function explain(where, observation) {
  return rulesFor(where).find((rule) => rule.matches(observation)) ?? null;
}

// ---------------------------------------------------------------------
// Reading a build
// ---------------------------------------------------------------------

function files(dir, found = []) {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) files(full, found);
    else found.push(full);
  }
  return found;
}

/**
 * Every public address a built tree serves, as the browser asks for it.
 *
 * Public: the builder's own directory is skipped, because it is not an
 * address of the domain and the serving configuration says so with a
 * 404. A deployment's output holds the state and every rendered
 * documentation tree inside it, so a comparison that walked them would
 * report thousands of «only in this build» differences about files
 * nobody can fetch — which is exactly what the first live render did
 * (`isBuilderState`).
 */
function addresses(root) {
  const map = new Map();
  for (const file of files(root)) {
    const rel = relative(root, file).split(sep).join("/");
    const address = rel.endsWith("index.html")
      ? `/${rel.slice(0, -"index.html".length)}`
      : `/${rel}`;
    if (isBuilderState(address)) continue;
    map.set(address, file);
  }
  return map;
}

/**
 * Addresses whose name is a secret.
 *
 * The IndexNow key file is named after the key, and the key is the proof
 * of control over the domain: printing the file name in a report would
 * publish it. The address is compared like any other and shown as a
 * placeholder (R-25).
 */
function safeAddress(address) {
  return /^\/[0-9a-f]{32}\.txt$/.test(address)
    ? "/<indexnow-key>.txt"
    : address;
}

// ---------------------------------------------------------------------
// Reading a page
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

function decode(text) {
  return text
    .replace(/&#(\d+);/g, (_, code) => String.fromCodePoint(Number(code)))
    .replace(/&#x([0-9a-f]+);/gi, (_, code) =>
      String.fromCodePoint(Number.parseInt(code, 16)),
    )
    .replace(/&[a-z]+;/gi, (entity) => ENTITIES[entity] ?? entity);
}

/** The attributes of one tag, as a map, with entities decoded. */
function attributes(tag) {
  const found = new Map();
  for (const match of tag.matchAll(/([a-zA-Z_:][-\w:.]*)="([^"]*)"/g)) {
    found.set(match[1].toLowerCase(), decode(match[2]));
  }
  return found;
}

function tags(html, name) {
  return [...html.matchAll(new RegExp(`<${name}\\b[^>]*>`, "gi"))].map(
    (match) => attributes(match[0]),
  );
}

function head(html) {
  const end = html.indexOf("</head>");
  return end === -1 ? html : html.slice(0, end);
}

function lang(html) {
  return attributes(/<html\b[^>]*>/i.exec(html)?.[0] ?? "").get("lang") ?? "";
}

function title(html) {
  return decode(
    /<title[^>]*>([\s\S]*?)<\/title>/i.exec(html)?.[1] ?? "",
  ).trim();
}

/** Named metadata, keyed the way a reader of the HTML would name it. */
function metaValues(html) {
  const values = new Map();
  for (const tag of tags(head(html), "meta")) {
    const name = tag.get("name") ?? tag.get("property");
    const content = tag.get("content");
    if (name === undefined || content === undefined) continue;
    values.set(name, content);
  }
  return values;
}

/** `rel`-keyed links, each as the list of its `href`s in document order. */
function linkValues(html, rel) {
  return tags(head(html), "link")
    .filter((tag) => tag.get("rel") === rel)
    .map((tag) => tag);
}

/**
 * The structured data, normalised: keys sorted at every depth so two
 * serialisations of one graph compare equal, and the comparison is about
 * the graph rather than about the order a `JSON.stringify` happened to
 * walk it in.
 */
function structuredData(html) {
  const found =
    /<script[^>]*application\/ld\+json[^>]*>([\s\S]*?)<\/script>/i.exec(html);
  if (found === null) return null;
  const sort = (value) => {
    if (Array.isArray(value)) return value.map(sort);
    if (value !== null && typeof value === "object") {
      return Object.fromEntries(
        Object.keys(value)
          .sort()
          .map((key) => [key, sort(value[key])]),
      );
    }
    return value;
  };
  return JSON.stringify(sort(JSON.parse(decode(found[1]))));
}

/**
 * What a reader can see, as the set of text fragments the page shows.
 *
 * A set and not a string, and the reason is worth stating: the two
 * builds put the same words in a different order in exactly one place —
 * the footer, where the shared component prints the copyright after its
 * columns rather than inside them. Comparing the pages as one long
 * string would report that as a difference in every fragment after it,
 * and would then have to be silenced with a rule broad enough to hide a
 * real loss. Comparing fragments answers the two questions that matter
 * and nothing else: is any of the owner's copy missing, and is there any
 * copy here that was not there before.
 */
function visibleText(html) {
  let body = html.slice(html.indexOf("<body"));
  body = body.replace(/<!--[\s\S]*?-->/g, "");
  body = body.replace(/<(script|style|svg|noscript)\b[\s\S]*?<\/\1>/gi, "");
  return new Set(
    body
      .split(/<[^>]+>/)
      .map((fragment) => decode(fragment).replace(/\s+/g, " ").trim())
      .filter((fragment) => fragment.length > 0),
  );
}

/** The analytics tag, without ever reading the id it carries. */
function umami(html) {
  const found = /<script\b[^>]*\bsrc="\/u\/s\.js"[^>]*>/i.exec(head(html));
  if (found === null) return null;
  const tag = attributes(found[0]);
  return {
    hasId: (tag.get("data-website-id") ?? "").length > 0,
    hostUrl: tag.get("data-host-url") ?? "",
    defer: /\bdefer\b/.test(found[0]),
  };
}

// ---------------------------------------------------------------------
// The report
// ---------------------------------------------------------------------

const lines = [];
const explained = [];
let unexplained = 0;

function say(text = "") {
  lines.push(text);
}

function check(name) {
  say(`--- ${name}`);
}

function ok(text) {
  say(`    ok        ${text}`);
}

/** A difference that a rule covers: recorded, counted, not a failure. */
function known(rule, text) {
  say(`    ${rule.id}      ${text}`);
  explained.push(rule.id);
}

function fail(text) {
  say(`    DIFFERENT ${text}`);
  unexplained += 1;
}

/** A difference: explained by a rule of this check, or counted against. */
function difference(where, observation, text) {
  const rule = explain(where, observation);
  if (rule === null) fail(text);
  else known(rule, text);
}

// ---------------------------------------------------------------------
// The checks
// ---------------------------------------------------------------------

/**
 * Address differences, grouped by the rule that covers them.
 *
 * Grouped because a bundler emits a hundred and forty chunks and a
 * report that names each one is a report nobody finishes reading — the
 * fact worth knowing is that all hundred and forty are the bundler's
 * output and none of them is a page. Anything no rule covers is still
 * named one by one, which is the only place a name matters.
 */
function compareAddresses(reference, port) {
  check("addresses");
  const grouped = new Map();
  const consider = (address, side) => {
    const rule = explain("addresses", address);
    if (rule === null) {
      fail(`${safeAddress(address)} — ${side}`);
      return;
    }
    const key = `${rule.id} ${side}`;
    const bucket = grouped.get(key) ?? { rule, side, addresses: [] };
    bucket.addresses.push(safeAddress(address));
    grouped.set(key, bucket);
  };

  for (const address of [...reference.keys()].sort()) {
    if (!port.has(address)) consider(address, "only in the Astro build");
  }
  for (const address of [...port.keys()].sort()) {
    if (!reference.has(address)) consider(address, "only in this build");
  }

  for (const bucket of grouped.values()) {
    const shown = bucket.addresses.slice(0, 3).join(", ");
    const rest =
      bucket.addresses.length > 3
        ? ` and ${bucket.addresses.length - 3} more`
        : "";
    known(
      bucket.rule,
      `${bucket.addresses.length} address(es) ${bucket.side}: ${shown}${rest}`,
    );
  }

  const shared = [...reference.keys()].filter((address) => port.has(address));
  ok(`${shared.length} address(es) served by both`);
}

function compareBytes(name, reference, port, referenceFile, portFile) {
  check(name);
  if (!reference.has(referenceFile)) {
    ok(`the Astro build has no ${name}`);
    return;
  }
  if (!port.has(portFile)) {
    difference("addresses", portFile, `${name} is not in this build`);
    return;
  }
  const a = readFileSync(reference.get(referenceFile));
  const b = readFileSync(port.get(portFile));
  if (a.equals(b)) {
    ok(`byte for byte (${a.length} bytes)`);
    return;
  }
  const meaningful = (buffer) =>
    buffer
      .toString("utf8")
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter((line) => line.length > 0);
  const before = meaningful(a);
  const after = meaningful(b);
  for (const line of before.filter((line) => !after.includes(line))) {
    difference(name, line, `dropped: ${line}`);
  }
  for (const line of after.filter((line) => !before.includes(line))) {
    difference(name, line, `added: ${line}`);
  }
  say(`    note      not byte for byte: ${a.length} bytes became ${b.length}`);
}

function comparePage(address, reference, port) {
  check(`page ${address}`);
  const a = readFileSync(reference, "utf8");
  const b = readFileSync(port, "utf8");

  const same = (what, x, y) => {
    if (x === y) ok(`${what}: ${x.length > 70 ? `${x.slice(0, 67)}…` : x}`);
    else fail(`${what}: «${x}» became «${y}»`);
  };

  same("lang", lang(a), lang(b));
  same("title", title(a), title(b));

  const metaA = metaValues(a);
  const metaB = metaValues(b);
  const named = [...metaA.keys()].filter(
    (name) =>
      name === "description" ||
      name === "theme-color" ||
      name.startsWith("og:") ||
      name.startsWith("twitter:"),
  );
  for (const name of named) {
    const x = metaA.get(name) ?? "";
    const y = metaB.get(name);
    if (y === undefined) fail(`meta ${name}: dropped («${x}»)`);
    else if (x === y) ok(`meta ${name}`);
    else
      difference(
        "meta",
        { name, reference: x, port: y },
        `meta ${name}: «${x}» became «${y}»`,
      );
  }

  const canonicalA = linkValues(a, "canonical")[0]?.get("href") ?? "";
  const canonicalB = linkValues(b, "canonical")[0]?.get("href") ?? "";
  same("canonical", canonicalA, canonicalB);

  const alternates = (html) =>
    linkValues(html, "alternate")
      .filter((tag) => tag.has("hreflang"))
      .map((tag) => `${tag.get("hreflang")} -> ${tag.get("href")}`)
      .sort();
  same("hreflang", alternates(a).join(" | "), alternates(b).join(" | "));

  const preloads = (html) =>
    tags(head(html), "link").filter(
      (tag) => tag.get("rel") === "preload" && tag.get("as") === "font",
    ).length;
  same("font preloads", String(preloads(a)), String(preloads(b)));

  const jsonA = structuredData(a);
  const jsonB = structuredData(b);
  if (jsonA === null && jsonB === null) ok("no structured data on either");
  else if (jsonA === jsonB) ok("structured data (normalised) identical");
  else fail(`structured data: «${jsonA}» became «${jsonB}»`);

  const umamiA = umami(a);
  const umamiB = umami(b);
  if (umamiA !== null && umamiB === null) {
    difference(
      "umami",
      "absent-without-id",
      "the analytics tag is not on this page",
    );
  } else if (umamiA !== null && umamiB !== null) {
    if (umamiB.hasId && umamiB.defer && umamiB.hostUrl === umamiA.hostUrl) {
      ok("analytics tag: deferred, same host, an id is present");
    } else {
      fail(
        `analytics tag: defer=${umamiB.defer}, id present=${umamiB.hasId}, host «${umamiB.hostUrl}» vs «${umamiA.hostUrl}»`,
      );
    }
  } else if (umamiB !== null) {
    fail("an analytics tag appeared on a page that had none");
  }

  const textA = visibleText(a);
  const textB = visibleText(b);
  const lost = [...textA].filter((fragment) => !textB.has(fragment));
  const gained = [...textB].filter((fragment) => !textA.has(fragment));
  for (const fragment of lost) {
    difference("text", fragment, `text lost: «${fragment}»`);
  }
  for (const fragment of gained) {
    difference("text", fragment, `text added: «${fragment}»`);
  }
  ok(`${textA.size} visible fragment(s) compared`);
}

function compareSitemap(reference, port) {
  check("sitemap.xml");
  const locations = (file) =>
    [...readFileSync(file, "utf8").matchAll(/<loc>([^<]+)<\/loc>/g)].map(
      (match) => match[1],
    );
  const a = locations(reference);
  const b = locations(port);
  for (const location of a.filter((location) => !b.includes(location))) {
    fail(`address dropped from the sitemap: ${location}`);
  }
  for (const location of b.filter((location) => !a.includes(location))) {
    const path = new URL(location).pathname;
    difference("sitemap", path, `address added to the sitemap: ${location}`);
  }
  ok(`${a.length} address(es) in the Astro sitemap, ${b.length} in this one`);

  const lastmod = (file) => [
    ...new Set(
      [...readFileSync(file, "utf8").matchAll(/<lastmod>([^<]+)</g)].map(
        (match) => match[1],
      ),
    ),
  ];
  const [before] = lastmod(reference);
  const [after] = lastmod(port);
  if (before !== after) {
    difference(
      "sitemap",
      "lastmod",
      `lastmod: ${before ?? "none"} became ${after ?? "none"}`,
    );
  } else {
    ok("lastmod unchanged");
  }
}

/**
 * The short index for language models.
 *
 * Two things are load-bearing and both are checked literally. The
 * disambiguation paragraph is the site's only chance to tell a model
 * that reuses the name apart from this project, and it has to survive
 * word for word. And every link the Astro index offered has to still be
 * offered: an index is a promise about where things are.
 */
function compareLlms(reference, port) {
  check("llms.txt");
  const a = readFileSync(reference, "utf8");
  const b = readFileSync(port, "utf8");

  const paragraph =
    'Disambiguation: several unrelated projects reuse the "vibevm" name. The links below are the authoritative ones for this VibeVM.';
  if (!a.includes(paragraph)) {
    fail("the Astro index no longer carries the disambiguation paragraph");
  } else if (b.includes(paragraph)) {
    ok("the disambiguation paragraph is there, word for word");
  } else {
    fail("the disambiguation paragraph is not in this build's index");
  }

  const links = (text) =>
    [...text.matchAll(/\((https?:\/\/[^)]+)\)/g)].map((m) => m[1]);
  const before = links(a);
  const after = links(b);
  for (const link of before.filter((link) => !after.includes(link))) {
    fail(`link dropped from llms.txt: ${link}`);
  }
  for (const link of after.filter((link) => !before.includes(link))) {
    difference("llms.txt", link, `link added to llms.txt: ${link}`);
  }
  ok(`${before.length} reference link(s) checked`);
}

// ---------------------------------------------------------------------

function run(referenceRoot, portRoot) {
  const reference = addresses(referenceRoot);
  const port = addresses(portRoot);

  say("landing parity — the Astro build against this one");
  say(
    `  reference : ${relative(PACKAGE_ROOT, referenceRoot) || referenceRoot}`,
  );
  say(`  this build: ${relative(PACKAGE_ROOT, portRoot) || portRoot}`);
  say();

  compareAddresses(reference, port);

  for (const address of ["/", "/ru/", "/404.html"]) {
    const a = reference.get(address);
    const b = port.get(address);
    if (a === undefined || b === undefined) {
      check(`page ${address}`);
      fail("the page is missing from one of the builds");
      continue;
    }
    comparePage(address, a, b);
  }

  compareBytes("robots.txt", reference, port, "/robots.txt", "/robots.txt");

  const keyFile = [...reference.keys()].find((address) =>
    /^\/[0-9a-f]{32}\.txt$/.test(address),
  );
  if (keyFile !== undefined) {
    check("IndexNow key file");
    const portKey = [...port.keys()].find((address) =>
      /^\/[0-9a-f]{32}\.txt$/.test(address),
    );
    if (portKey === undefined) {
      difference(
        "addresses",
        keyFile,
        "the key file is not in this build (no key in the build environment)",
      );
    } else if (
      readFileSync(reference.get(keyFile)).equals(
        readFileSync(port.get(portKey)),
      )
    ) {
      ok("byte for byte");
    } else {
      fail("the key file differs from the one the domain serves");
    }
  }

  if (reference.has("/sitemap.xml") && port.has("/sitemap.xml")) {
    compareSitemap(reference.get("/sitemap.xml"), port.get("/sitemap.xml"));
  }
  if (reference.has("/llms.txt") && port.has("/llms.txt")) {
    compareLlms(reference.get("/llms.txt"), port.get("/llms.txt"));
  }

  say();
  say("--- deliberate differences cited");
  const counted = new Map();
  for (const id of explained) counted.set(id, (counted.get(id) ?? 0) + 1);
  for (const rule of DIFFERENCES) {
    const times = counted.get(rule.id);
    if (times === undefined) continue;
    say(`  ${rule.id} (${times}×) ${rule.reason}`);
  }
  const unused = DIFFERENCES.filter((rule) => !counted.has(rule.id));
  if (unused.length > 0) {
    say();
    say(
      `  rules no difference needed this run: ${unused.map((rule) => rule.id).join(", ")}`,
    );
  }

  say();
  say(
    unexplained === 0
      ? `parity: green — ${explained.length} deliberate difference(s), 0 unexplained.`
      : `parity: RED — ${unexplained} unexplained difference(s), ${explained.length} deliberate.`,
  );
  process.stdout.write(`${lines.join("\n")}\n`);
  return unexplained === 0 ? 0 : 1;
}

const referenceArgument = process.argv[2];
if (referenceArgument === undefined) {
  process.stderr.write(
    [
      "parity: usage — node tools/parity.mjs <astro-dist> [qwik-dist]",
      "",
      "The first argument is the `dist/` of the Astro landing, built in a",
      "scratch copy of that repository (R-28 — it is never built in place):",
      "",
      "  npm ci && npm run build && node scripts/build-llms-full.mjs",
      "",
      "The second defaults to site/dist, this package's static build.",
      "",
    ].join("\n"),
  );
  process.exit(2);
}

process.exit(
  run(
    resolve(referenceArgument),
    resolve(process.argv[3] ?? join(PACKAGE_ROOT, "site", "dist")),
  ),
);
