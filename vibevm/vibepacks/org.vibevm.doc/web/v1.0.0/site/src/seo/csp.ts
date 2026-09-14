/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SEO-SSR */

/**
 * The Content-Security-Policy the built site needs, computed from the
 * built site.
 *
 * The pages carry inline scripts that cannot become files. The theme has
 * to be on the root element before the first stylesheet is fetched or
 * the reader watches their own setting fail; the router's scroll
 * restoration has to be undone before the router's own bootstrap runs;
 * and the framework writes a page's resumability state into the document
 * itself. A policy that answered this with `'unsafe-inline'` would allow
 * every script anyone ever injects into a page, which is the one thing a
 * policy is for.
 *
 * So each one is named by the hash of its bytes, and the hashes are
 * computed from the HTML that was actually written — never typed into a
 * file that would go stale on the next edit (X-035). The build writes the
 * line to `dist/csp.txt` for the deployment atom to serve as a header;
 * nothing here touches a server (R-24).
 *
 * What counts as an inline script is decided the way a browser decides
 * it: an element with no `src`, whose `type` is absent, empty, `module`
 * or a JavaScript media type. A `<script type="qwik/state">` or an
 * `application/ld+json` block is a data block — the browser never
 * executes it and a policy never blocks it — so hashing those would grow
 * the header by a hash a browser never looks for.
 */

import { createHash } from "node:crypto";

/** The `type` values a browser treats as executable script. */
const EXECUTABLE = new Set([
  "",
  "module",
  "text/javascript",
  "application/javascript",
  "application/ecmascript",
  "text/ecmascript",
  "application/x-javascript",
]);

/** One inline script found in a page. */
export type InlineScript = {
  /** The exact text between the tags — what the hash is taken over. */
  readonly body: string;
  /** The element's `type`, lowercased; `""` when it carries none. */
  readonly type: string;
};

/**
 * Read the attributes of one `<script …>` tag, quote-aware.
 *
 * A regular expression up to the first `>` is wrong here and measurably
 * so: Qwik writes a head script's source into an attribute of the same
 * element, and an attribute value that contains a `>` would end the tag
 * early and take half the body with it. The scanner walks the characters
 * and only ends the tag on a `>` that is not inside quotes.
 */
function tagEnd(html: string, from: number): number {
  let quote = "";
  for (let at = from; at < html.length; at += 1) {
    const ch = html[at] ?? "";
    if (quote !== "") {
      if (ch === quote) quote = "";
      continue;
    }
    if (ch === '"' || ch === "'") quote = ch;
    else if (ch === ">") return at;
  }
  return -1;
}

/** Every inline script of one page, in document order. */
export function inlineScripts(html: string): readonly InlineScript[] {
  const found: InlineScript[] = [];
  let at = 0;
  for (;;) {
    const open = html.indexOf("<script", at);
    if (open === -1) return found;
    const close = tagEnd(html, open + "<script".length);
    if (close === -1) return found;
    const attributes = html.slice(open + "<script".length, close);
    const end = html.indexOf("</script>", close);
    if (end === -1) return found;
    const body = html.slice(close + 1, end);
    at = end + "</script>".length;

    if (/\bsrc\s*=/.test(attributes)) continue;
    const type = /\btype\s*=\s*("([^"]*)"|'([^']*)'|([^\s>]+))/.exec(
      attributes,
    );
    const value = (type?.[2] ?? type?.[3] ?? type?.[4] ?? "")
      .trim()
      .toLowerCase();
    if (!EXECUTABLE.has(value)) continue;
    found.push({ body, type: value });
  }
}

/** The CSP source expression naming one script by its bytes. */
export function hashOf(body: string): string {
  const digest = createHash("sha256").update(body, "utf8").digest("base64");
  return `'sha256-${digest}'`;
}

/**
 * The policy line.
 *
 * `default-src 'self'` is the floor and it is the whole point: images,
 * fonts, the analytics beacon and the web-app manifest are all served by
 * this domain, and nothing on this site is fetched from anywhere else
 * (R-09). Three directives narrow it further — a page has no plugins, no
 * forms, and no business being framed — and `base-uri 'none'` keeps an
 * injected `<base>` from moving every relative address on the page.
 *
 * Styles keep `'unsafe-inline'`, and that is a named limit rather than an
 * oversight. The framework inlines each component's stylesheet into the
 * document, and the reading settings write the measure and the font size
 * onto the element as a `style` attribute — which no hash can cover,
 * because a hash names an element's content and an attribute has none.
 */
export function cspPolicy(hashes: readonly string[]): string {
  const script = ["'self'", ...hashes].join(" ");
  return [
    "default-src 'self'",
    `script-src ${script}`,
    "style-src 'self' 'unsafe-inline'",
    "object-src 'none'",
    "base-uri 'none'",
    "form-action 'none'",
    "frame-ancestors 'none'",
  ].join("; ");
}

/** The hashes a policy line names, for the check that reads it back. */
export function hashesIn(policy: string): readonly string[] {
  return [...policy.matchAll(/'sha256-[A-Za-z0-9+/=]+'/g)].map(
    (match) => match[0],
  );
}

/** The variable the serving configuration reads the policy out of. */
const CSP_VARIABLE = "vibe_csp";

/**
 * The length at which nginx stops reading a configuration parameter.
 *
 * Measured rather than looked up, inside the image the site is served
 * from: 4090 bytes parse, 4096 do not, and the refusal is «too long
 * parameter, probably missing terminating `"` character» — the server
 * does not start at all. It is the configuration reader's own buffer,
 * and no directive raises it.
 */
const NGINX_PARAMETER_LIMIT = 4096;

/**
 * The longest policy this serving arrangement can carry, with room for
 * the directive around it.
 *
 * A site of one manual is comfortably inside it and a site of a whole
 * registry is not: 47 hashes over the package's own fixture library
 * (3.4 kB), about 150 over the manual (11 kB), and 1641 over the 48
 * libraries of the live registry (87 kB). The first fits, the second
 * does not, and the third misses by more than twenty times — which is
 * the ceiling X-044 wrote down before anyone had measured where it was.
 */
export const CSP_CONF_LIMIT = 4000;

/**
 * The same policy as a fragment of serving configuration, generated from
 * the line the build just wrote.
 *
 * A header is not a file, so something has to turn one into the other,
 * and the only two candidates are a hand-written configuration and a
 * generator. A hand-written one is wrong on the first rebuild: the
 * hashes are computed from the bytes of the pages, and a page that gains
 * an inline script gains a hash — so the file would go stale silently,
 * and the symptom would be a blocked script in a reader's browser rather
 * than a red build (X-035).
 *
 * It is a `map` over the response's media type rather than a header on
 * the whole server for the reason X-044 measured: the policy is one hash
 * per distinct inline script — 47 over the fixture library, about 150
 * over the manual — and only a document can execute a script. A server
 * that sent the whole list with every chunk, font and image would spend
 * kilobytes per request to protect a file that cannot run anything. An
 * empty value adds no header at all, which is what every other type
 * gets.
 *
 * The policy is double-quoted and contains no double quote of its own:
 * it is built from `'self'`, directive names and base64 hashes, none of
 * which carry one. The generator refuses rather than emits a fragment it
 * cannot quote, because a configuration file that parses differently
 * from how it reads is worse than no file.
 *
 * And there is a ceiling, which a site of one manual never met and a
 * site of a whole registry meets immediately — see [`CSP_CONF_LIMIT`].
 */
export function cspConf(policy: string): string {
  const line = policy.trim();
  if (line.includes('"') || line.includes("\n")) {
    throw new Error(
      "the policy carries a quote or a newline and cannot be written as one " +
        "quoted nginx value",
    );
  }
  const head = [
    "# Generated by the site build from the policy it computed over the",
    "# pages it wrote (`csp.txt`). Do not edit: the next render replaces it.",
  ];
  if (line.length > CSP_CONF_LIMIT) {
    return [
      ...head,
      "#",
      `# NO POLICY IS SERVED. The policy this render computed is ${line.length}`,
      `# bytes — one hash per distinct inline script — and nginx refuses a`,
      `# single parameter of ${NGINX_PARAMETER_LIMIT} bytes or more, so a`,
      "# configuration carrying it would not start at all. Serving it is not",
      "# the obvious repair either: a header of that size would be sent with",
      "# every page, and the pages of this site average a fraction of it.",
      "#",
      "# This is the ceiling X-044 recorded, reached. What replaces the hashes",
      "# — a nonce written by a server that renders, `'strict-dynamic'`, or a",
      "# policy that stops naming scripts — is a decision about the shape of",
      "# the deployment and belongs to its owner, not to a build.",
      `map $sent_http_content_type $${CSP_VARIABLE} {`,
      '    default "";',
      "}",
      "",
    ].join("\n");
  }
  return [
    ...head,
    "#",
    "# Only a document can execute a script, so only a document carries the",
    "# policy; every other media type maps to the empty string, and nginx",
    "# adds no header for an empty value.",
    `map $sent_http_content_type $${CSP_VARIABLE} {`,
    '    default       "";',
    `    ~*^text/html  "${line}";`,
    "}",
    "",
  ].join("\n");
}
