/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-META-AND-PRINT */

/**
 * The rules the page cites, collected from the quotations themselves.
 *
 * Each one is already in the text with its `spec://` address on it, so
 * this list is a second view of the same facts and never a second
 * source: nothing is fetched, nothing is parsed, and a page that quotes
 * no rule gets no block at all rather than an empty heading.
 *
 * It is its own module rather than a function inside the contents,
 * because it is now its own block at the end of the page: the contents
 * answers «where am I in this page» and this answers «what does this
 * page stand on», and the two stopped sharing an element when the
 * addresses grew taller than the headings they hung under.
 */

import { all, island, one } from "./dom.ts";

/** Fill the block and reveal it, or leave it hidden if nothing is cited. */
export function startCitedRules(): () => void {
  const region = island();
  const block = one("[data-page-rules]");
  const list = one("[data-page-rules-list]");
  if (region === null || block === null || list === null) {
    return () => undefined;
  }

  list.replaceChildren();
  const seen = new Set<string>();
  for (const quote of all("a.rule[data-uri]", region)) {
    const uri = quote.getAttribute("data-uri");
    if (uri === null || seen.has(uri)) continue;
    seen.add(uri);
    const item = document.createElement("li");
    const link = document.createElement("a");
    link.href = quote.getAttribute("href") ?? "#";
    link.textContent = uri;
    item.appendChild(link);
    list.appendChild(item);
  }

  /* How many, in the summary, because the summary is all a reader sees
     of a folded block. The parentheses are written here and not in the
     markup: an empty pair standing in a page that cites nothing would be
     a promise with no list behind it. */
  const count = one("[data-page-rules-count]");
  if (count !== null) count.textContent = `(${seen.size})`;

  block.hidden = seen.size === 0;

  return () => undefined;
}
