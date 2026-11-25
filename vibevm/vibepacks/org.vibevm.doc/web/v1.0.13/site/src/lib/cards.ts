/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#NAV-CHAPTERS-READER */

/**
 * The pages of one documentation as CARDS, in either of the two orders it
 * has.
 *
 * It is a file of its own beside `contents.ts` for the reason that file
 * gives about itself, one step further on. `view.ts` answers what one
 * ADDRESS means; `contents.ts` answers what one documentation looks like
 * from the inside as a LIST of places; and this answers the third
 * question, which is what each of those places is ABOUT — the shelf on a
 * documentation's own front page, where a reader is choosing which page to
 * open rather than finding their way back to one.
 *
 * The distinction is not academic and the site got it wrong once in each
 * direction. The navigation lists the source's pages, in the source's
 * words, in every language: an adaptation in progress is not a smaller
 * manual, and a reader must be told which pages exist before being told
 * which of them have been adapted. A card is the other question, so it is
 * the page as the chosen edition HAS it, with the source's text standing
 * in where the adaptation has not reached — which is exactly what the
 * reader will find on opening it.
 *
 * And the summary on a card is the page's own leading fact, never the
 * documentation's abstract. They answer different questions and are about
 * different things; the shelf showed the second under the name of the
 * first, once per page, so every page of a manual claimed to cover the
 * whole of it.
 *
 * Both orders are shelved from here, out of one function per card, because
 * the order is the ONLY difference between them: the layer law's list is
 * the manifest's own, the learning path's is the author's
 * (`##NAV-CHAPTERS-READER`), and a card that differed between the two
 * would be the same page described twice.
 */

import { learningPath } from "./contents.ts";
import { docHref } from "./href.ts";
import {
  addressOf,
  documentOf,
  resolvePage,
  sourceEdition,
  type Library,
} from "./library.ts";

/** One page of a documentation, as the «Pages» shelf shows it. */
export type PageCard = {
  readonly title: string;
  readonly href: string;
  /**
   * The page's own leading fact, from the manifest. Absent when the page
   * declares none — in which case the card shows no disclosure at all,
   * rather than the documentation's abstract under a page's name.
   */
  readonly summary?: string;
};

/** One page as a card, wherever the list it stands in came from. */
function pageCard(
  library: Library,
  at: string | null,
  document: string,
): PageCard {
  const resolved = resolvePage(library, at, document);
  const summary = resolved === null ? "" : resolved.page.summary.trim();
  return {
    title: resolved?.page.title ?? document,
    href: docHref(addressOf(library, at, document)),
    ...(summary.length === 0 ? {} : { summary }),
  };
}

/** Every page of a documentation, in the order the manifest gives them. */
export function pageCards(library: Library, at: string | null): PageCard[] {
  return sourceEdition(library).pages.map((declared) =>
    pageCard(library, at, documentOf(declared.path)),
  );
}

/** One chapter of the learning path, as the package page shelves it. */
export type PathShelf = {
  readonly id: string;
  /** The number over it, `1`; empty for an appendix chapter. */
  readonly number: string;
  readonly title: string;
  readonly pages: readonly PageCard[];
};

/**
 * The same pages grouped into the chapters of the declared learning path,
 * or `null` when the documentation declared none.
 *
 * `null` and not an empty list, because the shelf turns on the difference:
 * a documentation with no path keeps the manifest's order and says so in
 * its caption, and one that declared a path is shelved by chapter.
 */
export function pathCards(
  library: Library,
  at: string | null,
): readonly PathShelf[] | null {
  const path = learningPath(library, at);
  if (path === null) return null;
  return path.map((chapter) => ({
    id: chapter.id,
    number: chapter.number,
    title: chapter.title,
    pages: chapter.pages.map((page) => pageCard(library, at, page.document)),
  }));
}
