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
