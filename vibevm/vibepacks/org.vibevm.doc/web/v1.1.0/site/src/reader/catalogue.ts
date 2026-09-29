/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#LOC-SITE */

/**
 * Which language a reader who came to the door gets.
 *
 * The catalogue has no language segment of its own, so somebody has to
 * choose one, and the order is the vision's: what the reader chose
 * before, then what their browser asks for, then the documentation's own
 * language. The last is not a fallback so much as the truthful answer —
 * a manual written in English is in English until somebody adapts it.
 *
 * On a static host this runs in the browser, and that is a real
 * limitation rather than a shortcut. `Accept-Language` is a request
 * header, and a directory of files has nothing that reads one;
 * `navigator.languages` is the same preference list as seen from the
 * other side of the request, so the decision is the same and the place
 * it is taken is one hop later. The cost is a redirect a reader may see
 * flash. Should the site ever be served by something that can read a
 * header, this becomes a 302 and the module goes away.
 *
 * The decision is taken ONCE per session and never against an explicit
 * choice: a reader who has just clicked «English» must land on the
 * English catalogue and stay there, which is why the selector records
 * the click before it navigates.
 */

import { rememberedLanguage } from "./fallback.ts";
import { readSession, writeSession } from "./storage.ts";

/** One decision per session; coming back to the door is not asking again. */
const DECIDED = "catalogue-language";

/** One edition of the catalogue, as the page already knows it. */
export type CatalogueEdition = {
  /** The BCP-47 tag of that edition. */
  readonly tag: string;
  /** Its catalogue address. */
  readonly href: string;
  /** True for the documentation's own language, which has no segment. */
  readonly source: boolean;
};

/** The primary subtag, which is what a preference is matched on. */
function primary(tag: string): string {
  return (tag.split("-")[0] ?? tag).toLowerCase();
}

function find(
  editions: readonly CatalogueEdition[],
  tag: string,
): CatalogueEdition | null {
  const want = primary(tag);
  return editions.find((one) => primary(one.tag) === want) ?? null;
}

export function startCatalogue(
  editions: readonly CatalogueEdition[],
): () => void {
  if (readSession(DECIDED) !== null) return () => undefined;
  writeSession(DECIDED, "1");

  const chosen = rememberedLanguage();
  const remembered = chosen === null ? null : find(editions, chosen);
  if (remembered !== null) {
    if (!remembered.source) window.location.replace(remembered.href);
    return () => undefined;
  }

  for (const preference of navigator.languages) {
    const match = find(editions, preference);
    if (match === null) continue;
    if (!match.source) window.location.replace(match.href);
    return () => undefined;
  }

  // Nothing matched: the documentation's own language is the answer, and
  // the reader is already looking at it.
  return () => undefined;
}
