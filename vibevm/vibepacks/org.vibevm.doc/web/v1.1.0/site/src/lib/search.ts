/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SEO-MANIFEST-AND-RESOLVER */

/**
 * What the header's search box searches, and how it decides what a
 * reader meant.
 *
 * The corpus is the manifests, and nothing else. A documentation's title
 * and abstract, a page's title and its leading fact, and the coordinate
 * they are published under are all already named in the manifest the
 * pipeline wrote (`##PIPE-SHELL-PARSES-NOTHING`) — so the index is a
 * projection of what the site already carries, never a second reading of
 * the rendered pages. A search that read the islands would be a search
 * whose answers drifted from the catalogue's.
 *
 * It is a static host, so there is no query to ask: the build writes the
 * index beside `manifest.json` and the box fetches it once, the first
 * time a reader types. That is also why the matching lives here rather
 * than in the widget — a pure function over values is the half of this
 * that can be judged without a browser, and the acceptance the owner
 * asked for («it finds a page by a word from the title and by a word
 * from the abstract») is a test beside this file.
 *
 * The ranking is deliberately small. Four fields, one rule about each,
 * and every word of the query must land somewhere: a reader who types
 * two words means both. Nothing here stems, folds diacritics or guesses
 * at a language — the corpus is two languages today and a fuzzy matcher
 * that was tuned on one of them would quietly serve the other worse.
 */

/** What a shelf, a page or a documentation is, as the index holds it. */
export type SearchEntry = {
  /** The display title — a page's H1, or the documentation's own name. */
  readonly title: string;
  /** Where it is, already a served path at the `latest` spelling. */
  readonly href: string;
  /** Which documentation and language it belongs to, for the second line. */
  readonly context: string;
  /** The page's leading fact, or the documentation's abstract. */
  readonly summary: string;
  /** `<group>/<name>@<version>`: identity, and a thing readers type. */
  readonly coordinate: string;
  readonly kind: "documentation" | "page";
};

/** The index as `search.json` holds it. */
export type SearchIndex = {
  readonly schema_version: 1;
  readonly entries: readonly SearchEntry[];
};

/** How many answers a dropdown shows before it stops being a list. */
export const SEARCH_LIMIT = 8;

/** The shortest query worth answering; one letter matches everything. */
export const SEARCH_MIN = 2;

/**
 * What each field is worth when a word of the query lands in it.
 *
 * A title is what a reader is usually reaching for, and a word at the
 * START of a title is the strongest signal of all — «lock» should find
 * «Lock and store» before it finds a page that mentions locking in its
 * third sentence. The abstract is worth the least and is still worth
 * something: it is the only field that knows what a page is ABOUT.
 */
const WEIGHT = {
  titleStart: 6,
  title: 4,
  coordinate: 3,
  context: 2,
  summary: 1,
} as const;

/**
 * Whether `token` appears at the start of a word in `haystack`.
 *
 * Written out rather than `\b`, which is defined over ASCII word
 * characters and would answer «yes» in the middle of every Russian word
 * in the corpus. The question is really «is the character before it a
 * letter or a digit», and Unicode property escapes can ask that one.
 */
function atWordStart(haystack: string, token: string): boolean {
  for (let at = haystack.indexOf(token); at !== -1;) {
    if (at === 0) return true;
    const before = haystack[at - 1] ?? "";
    if (!/[\p{L}\p{N}]/u.test(before)) return true;
    at = haystack.indexOf(token, at + 1);
  }
  return false;
}

/** What one entry is worth for one word, and `0` when the word misses. */
function scoreToken(entry: SearchEntry, token: string): number {
  const title = entry.title.toLocaleLowerCase();
  if (atWordStart(title, token)) return WEIGHT.titleStart;
  if (title.includes(token)) return WEIGHT.title;
  if (entry.coordinate.toLocaleLowerCase().includes(token)) {
    return WEIGHT.coordinate;
  }
  if (entry.context.toLocaleLowerCase().includes(token)) return WEIGHT.context;
  if (entry.summary.toLocaleLowerCase().includes(token)) return WEIGHT.summary;
  return 0;
}

/** The words of a query, lower-cased and without the empty ones. */
export function queryWords(query: string): readonly string[] {
  return query
    .toLocaleLowerCase()
    .split(/\s+/)
    .filter((word) => word.length > 0);
}

/**
 * The entries a query names, best first.
 *
 * Every word must land somewhere — two words are a narrowing and not a
 * wish — and the order is the sum of what each of them was worth. Ties
 * keep the index's own order, which is the catalogue's: a source
 * documentation before its adaptations, and a documentation's pages in
 * the order the layer law gives them.
 */
export function matches(
  index: SearchIndex,
  query: string,
  limit: number = SEARCH_LIMIT,
): readonly SearchEntry[] {
  const words = queryWords(query);
  if (words.length === 0 || query.trim().length < SEARCH_MIN) return [];

  const scored: { entry: SearchEntry; score: number }[] = [];
  for (const entry of index.entries) {
    let score = 0;
    for (const word of words) {
      const one = scoreToken(entry, word);
      if (one === 0) {
        score = 0;
        break;
      }
      score += one;
    }
    if (score > 0) scored.push({ entry, score });
  }

  return scored
    .sort((a, b) => b.score - a.score)
    .slice(0, limit)
    .map((one) => one.entry);
}

/**
 * An index read defensively out of whatever the fetch returned.
 *
 * `null` rather than a throw: the box is a convenience over a catalogue
 * that is still one click away, and a reader whose network gave them
 * half a file should get a search that says it found nothing — not a
 * page with an error in the console and a field that has stopped
 * answering.
 */
export function readSearchIndex(value: unknown): SearchIndex | null {
  if (!isRecord(value)) return null;
  if (value["schema_version"] !== 1) return null;
  const raw = value["entries"];
  if (!Array.isArray(raw)) return null;
  const list: readonly unknown[] = raw;

  const entries: SearchEntry[] = [];
  for (const one of list) {
    if (!isRecord(one)) return null;
    const title = text(one, "title");
    const href = text(one, "href");
    /* A row with no name or nowhere to go is not a row a reader could
       have been offered, so the file is not the index it claims to be. */
    if (title.length === 0 || href.length === 0) return null;
    entries.push({
      title,
      href,
      context: text(one, "context"),
      summary: text(one, "summary"),
      coordinate: text(one, "coordinate"),
      kind: one["kind"] === "documentation" ? "documentation" : "page",
    });
  }
  return { schema_version: 1, entries };
}

/** An object, asked rather than asserted — `lib/manifest.ts`'s question. */
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** One string field, or `""` for a field that is not one. */
function text(source: Record<string, unknown>, key: string): string {
  const value = source[key];
  return typeof value === "string" ? value : "";
}
