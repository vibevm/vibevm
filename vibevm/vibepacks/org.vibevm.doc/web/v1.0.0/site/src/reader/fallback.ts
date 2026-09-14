/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-LANGUAGE-SWITCH-KEEPS-PLACE */

/**
 * What happens when a reader asks for a language a page has not reached
 * yet.
 *
 * Never a 404 and never a redirect. The static build materialises the
 * page at the asked-for address with the source's text in it, and the
 * three things that make that honest rather than misleading are already
 * in the document: the source's language on the article, a `canonical`
 * link to the source page, and `noindex`, so a crawler indexes one page
 * rather than two copies of one text.
 *
 * Three things are left for the browser, and each has a reason it cannot
 * be done at build time.
 *
 * The notice is shown once per SESSION, and a session is only a thing a
 * browser has. A reader working through an adaptation in progress meets
 * this page shape repeatedly; the first telling explains, and the second
 * is noise.
 *
 * The internal links are rewritten to the address's language, because
 * the page was rendered from the source's document and its links point
 * where the source's do. A reader who chose Russian and followed one
 * would silently fall back into the source language and never return.
 *
 * The cookie records the choice — the same `lang` cookie the landing
 * uses, so the domain root and the documentation remember one answer.
 */

import { one, targetOf } from "./dom.ts";
import { readSession, writeSession } from "./storage.ts";

/** A year, which is what the vision asks the choice to last. */
const COOKIE_DAYS = 365;

/** One flag per session, per page shape rather than per page. */
const SEEN = "lang-fallback-seen";

/**
 * Remember the language of the address — not of the text.
 *
 * `SameSite=Lax` because the only thing this cookie does is choose a
 * language for a reader arriving at the door, and it has no business
 * travelling with a request some other site made. `path=/` because the
 * landing reads the same name at the domain root.
 */
export function rememberLanguage(segment: string): void {
  const until = new Date(Date.now() + COOKIE_DAYS * 86400000).toUTCString();
  try {
    document.cookie = `lang=${encodeURIComponent(segment)}; expires=${until}; path=/; SameSite=Lax`;
  } catch {
    /* A browser with cookies off keeps the page and forgets the choice. */
  }
}

/** Read the remembered language, or `null` when there is none. */
export function rememberedLanguage(): string | null {
  try {
    const found = /(?:^|;\s*)lang=([^;]*)/.exec(document.cookie);
    return found?.[1] === undefined ? null : decodeURIComponent(found[1]);
  } catch {
    return null;
  }
}

/**
 * Keep the reader inside the language they asked for.
 *
 * Every site-internal link is re-pointed at the address's language: one
 * that already carries a language segment has it replaced, and one that
 * has none gets the segment inserted after the documentation's mount.
 * Links out of the site, and links to the projections beside a page, are
 * left exactly as they are — a `.md` file is not translated by moving
 * its address.
 */
function keepLanguage(segment: string, mount: string): void {
  const links = Array.from(
    document.querySelectorAll<HTMLAnchorElement>(`a[href^="${mount}"]`),
  );
  for (const link of links) {
    const href = link.getAttribute("href");
    if (href === null) continue;
    const rest = href.slice(mount.length);
    const first = rest.split("/")[0] ?? "";
    // A group always has a dot in it and a language tag never does: the
    // same rule the address parser uses, so the two cannot disagree.
    const already = first.length > 0 && !first.includes(".");
    const tail = already ? rest.slice(first.length + 1) : rest;
    link.setAttribute("href", `${mount}${segment}/${tail}`);
  }
}

export type FallbackOptions = {
  /** The language segment of the ADDRESS, or `null` for the source. */
  readonly addressLanguage: string | null;
  /** True when the source's text is served under that address. */
  readonly fallback: boolean;
  /** The documentation's mount, so links can be re-pointed by address. */
  readonly mount: string;
};

export function startFallback(options: FallbackOptions): () => void {
  const segment = options.addressLanguage;
  if (segment !== null) rememberLanguage(segment);

  if (!options.fallback || segment === null) return () => undefined;

  keepLanguage(segment, options.mount);

  const notice = one("[data-fallback-notice]");
  if (notice === null) return () => undefined;

  if (readSession(SEEN) === null) notice.hidden = false;

  const onClick = (event: Event): void => {
    const target = targetOf(event);
    if (target === null) return;
    if (target.closest("[data-fallback-dismiss]") === null) return;
    notice.hidden = true;
    writeSession(SEEN, "1");
  };

  document.addEventListener("click", onClick);
  return () => document.removeEventListener("click", onClick);
}
