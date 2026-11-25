/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SEO-MANIFEST-AND-RESOLVER */

/**
 * What the header's search box asks, and the one fetch behind it.
 *
 * The box is a shape with a question mark: it knows when to ask and what
 * to do with the answers, and nothing about the corpus. This is the
 * corpus — the index the build published beside the page manifest,
 * fetched once and kept for the life of the document.
 *
 * Once, and lazily. A reader who never types never fetches it, and a
 * reader who types six letters fetches it on the first of them: the
 * promise is remembered rather than the result, so six keystrokes in the
 * same second are one request rather than six.
 *
 * A failure is an empty answer and not an exception. The index is a
 * convenience over a catalogue that is one click away in the same
 * header, and a reader whose network dropped the file should get a
 * search that finds nothing — not a page with a rejection nobody caught.
 */

import { $ } from "@qwik.dev/core";
import type { SearchHit } from "@vibe-docs/design";

import { matches, readSearchIndex, type SearchIndex } from "../lib/search.ts";
import { SEARCH_INDEX } from "../seo/search.ts";

/** The one request, kept as its promise so concurrent askers share it. */
let asked: Promise<SearchIndex | null> | null = null;

/** The index, fetched at most once per document. */
function corpus(): Promise<SearchIndex | null> {
  asked ??= fetch(SEARCH_INDEX, { headers: { accept: "application/json" } })
    .then((response) => (response.ok ? response.json() : null))
    .then((body: unknown) => readSearchIndex(body))
    .catch(() => null);
  return asked;
}

/**
 * The question the box asks, as the site answers it.
 *
 * A QRL and not a plain function, because it crosses from the route into
 * a component prop: it is loaded when a reader first types and not
 * before, which is the same thing the fetch below is doing one level
 * down.
 */
export const findInDocumentation = $(
  async (query: string): Promise<readonly SearchHit[]> => {
    const index = await corpus();
    if (index === null) return [];
    return matches(index, query).map((entry) => ({
      title: entry.title,
      href: entry.href,
      context: entry.context,
    }));
  },
);
