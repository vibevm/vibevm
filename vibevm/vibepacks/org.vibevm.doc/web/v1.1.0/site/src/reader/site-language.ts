/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

/**
 * The reader's choice of what the FURNITURE speaks, applied to the page
 * they are on.
 *
 * The documentation's language is an address and the site's is a
 * preference, so they are kept the way their kind asks to be kept: the
 * first in the path, the second in `localStorage` beside the theme, read
 * by the same reader on the landing and in the manual. A reader who
 * darkened the site and put its buttons into Russian has said two things
 * about themselves and neither is about which edition of a manual they
 * are reading.
 *
 * The swap happens in the browser because the pages are prerendered. A
 * static build writes each address once, in the language the routes are
 * written in; the reader arrives afterwards. The alternative — a second
 * copy of every documentation address under `/ru/` meaning «the same
 * text with Russian buttons» — would double the address map to say
 * something that is not about the text at all, and would put a
 * preference into `canonical`, `hreflang` and the sitemap, which
 * describe documents.
 *
 * So nothing here touches an address, a `<link>` or `<html lang>`: the
 * language of the DOCUMENT is what that attribute means, and the island
 * is skipped entirely — the text a reader came for is the pipeline's,
 * in the language the pipeline rendered it in.
 *
 * Late chrome is caught rather than re-scanned. The framework re-renders
 * small parts of the header after a keystroke, and a pass that ran again
 * over the whole document on every mutation would fight it; watching for
 * ADDED nodes and translating those alone cannot loop, because nothing
 * this does adds a node.
 */

import {
  chromeIn,
  isSiteLanguage,
  preferredSiteLanguage,
  type SiteLanguage,
} from "../lib/site-language.ts";
import { all } from "./dom.ts";
import { readLocal, writeLocal } from "./storage.ts";

/** Where the choice lives; beside `theme`, under the reader's prefix. */
const KEY = "site-lang";

/** The attributes that carry a name a person reads but does not see. */
const NAMED = ["aria-label", "placeholder", "title"];

/**
 * The language the furniture is in before anyone has chosen: what the
 * browser asks for, which is the same preference list a server would
 * have read out of `Accept-Language`.
 */
export function storedSiteLanguage(): SiteLanguage {
  const stored = readLocal(KEY);
  if (isSiteLanguage(stored)) return stored;
  return preferredSiteLanguage(navigator.languages);
}

/** Remember the choice, for the next page and the next visit. */
export function keepSiteLanguage(language: SiteLanguage): void {
  writeLocal(KEY, language);
}

/**
 * Everything outside the island, walked once.
 *
 * The island is rejected whole: it is the document the reader came for,
 * and a translation table let loose in it would be the site editing a
 * package's text. A script or a style element is rejected for the
 * obvious reason — its text is not read by anybody.
 */
function chrome(within: Node): TreeWalker {
  return document.createTreeWalker(
    within,
    NodeFilter.SHOW_ELEMENT | NodeFilter.SHOW_TEXT,
    {
      acceptNode: (node) => {
        if (!(node instanceof Element)) return NodeFilter.FILTER_ACCEPT;
        if (node.hasAttribute("data-island")) return NodeFilter.FILTER_REJECT;
        const tag = node.tagName;
        if (tag === "SCRIPT" || tag === "STYLE" || tag === "TEMPLATE") {
          return NodeFilter.FILTER_REJECT;
        }
        return NodeFilter.FILTER_ACCEPT;
      },
    },
  );
}

/** Put one region of the chrome into a language. */
function translate(within: Node, language: SiteLanguage): void {
  const walker = chrome(within);
  let node: Node | null = walker.currentNode;
  for (; node !== null; node = walker.nextNode()) {
    if (node instanceof Element) {
      for (const name of NAMED) {
        const value = node.getAttribute(name);
        if (value === null) continue;
        const moved = chromeIn(value.trim(), language);
        if (moved !== null) node.setAttribute(name, moved);
      }
      continue;
    }
    if (!(node instanceof Text)) continue;
    const raw = node.data;
    const text = raw.trim();
    if (text.length === 0) continue;
    const moved = chromeIn(text, language);
    /* Only the words the table knows move, and the whitespace around
       them stays: a label is written in JSX with the newlines of the
       markup around it. */
    if (moved !== null) node.data = raw.replace(text, moved);
  }
}

/**
 * Put a language on the page, and on every switch that offers one.
 *
 * The switches are found by their attribute rather than held as a
 * reference, for the reason the theme's are: a page may carry more than
 * one, and a reader who changes the language in the header must not be
 * left looking at another control still claiming the old one.
 */
export function applySiteLanguage(language: SiteLanguage): void {
  document.documentElement.setAttribute("data-site-lang", language);
  translate(document.body, language);
  for (const button of all("[data-site-lang-choice]")) {
    const current = button.dataset["siteLangChoice"] === language;
    button.classList.toggle("is-current", current);
    button.setAttribute("aria-pressed", current ? "true" : "false");
  }
}

/**
 * The landing's half of the same choice.
 *
 * There the interface language IS the address — `/` is English and
 * `/ru/` is Russian, and both pages are written out in full — so there
 * is nothing to translate and only something to record: a reader who
 * walked in through the Russian front door should find the manual's
 * furniture in Russian too.
 */
export function rememberSiteLanguage(language: SiteLanguage): void {
  keepSiteLanguage(language);
  document.documentElement.setAttribute("data-site-lang", language);
}

export function startSiteLanguage(): () => void {
  let language = storedSiteLanguage();
  applySiteLanguage(language);

  const onClick = (event: Event): void => {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const choice = target.closest("[data-site-lang-choice]");
    if (!(choice instanceof HTMLElement)) return;
    const chosen = choice.dataset["siteLangChoice"];
    if (!isSiteLanguage(chosen) || chosen === language) return;
    language = chosen;
    applySiteLanguage(language);
    keepSiteLanguage(language);
  };

  /* Chrome the framework renders after the first frame — the search
     box's list of answers, a panel that opens — arrives in the language
     the page was built in. It is translated as it appears, which is why
     this watches for added nodes and never re-reads the document. */
  const watch = new MutationObserver((changes) => {
    for (const change of changes) {
      for (const added of change.addedNodes) {
        if (added instanceof Element || added instanceof Text) {
          translate(added, language);
        }
      }
    }
  });
  watch.observe(document.body, { childList: true, subtree: true });

  document.addEventListener("click", onClick);
  return () => {
    watch.disconnect();
    document.removeEventListener("click", onClick);
  };
}
