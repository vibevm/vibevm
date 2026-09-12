/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SEO-STRUCTURED-DATA */

/**
 * The JSON-LD a documentation page carries.
 *
 * `##SEO-STRUCTURED-DATA` names the type and the fields: a `TechArticle`
 * with `headline`, `abstract`, `author`, `inLanguage`, `keywords`,
 * `datePublished` and `image`, plus a `BreadcrumbList` where a page has
 * a place in a hierarchy — and every page here does: the documentation
 * of the site, then the package, then the page.
 *
 * Every value is read off the manifest and none is composed. The
 * headline is the page's title, the abstract its own leading summary —
 * «taken, never composed», the same rule the meta description follows,
 * because a second description of a page that nobody proofreads is worse
 * than none. The author is the publisher: a documentation package is
 * published by a group, and the group is the only authorship the wire
 * carries.
 *
 * Two dates exist in a manifest and only one of them belongs here.
 * `datePublished` is the render date, which is what the page itself
 * shows a reader; there is no `dateModified`, no revision and no «out of
 * date since», because a version is a contract and the site shows
 * nothing behind it (D-27).
 */

/** What a page needs for its structured data, as plain values. */
export type ArticleFacts = {
  readonly absolute: string;
  readonly title: string;
  readonly summary: string;
  readonly language: string;
  readonly publisher: string;
  readonly renderedAt: string;
  readonly audiences: readonly string[];
  readonly genre?: string;
  readonly image: string;
  readonly packageTitle: string;
  readonly packageAbsolute: string;
  readonly doorAbsolute: string;
};

/** What a package's own page needs. */
export type CollectionFacts = {
  readonly absolute: string;
  readonly title: string;
  readonly abstract: string;
  readonly language: string;
  readonly publisher: string;
  readonly image: string;
  readonly doorAbsolute: string;
};

/**
 * The keywords of a page.
 *
 * The audiences and the genre, and nothing invented. A documentation
 * page is marked up for whom it is written and what kind of page it is,
 * and those two vocabularies are closed and reviewed (PROP-043,
 * PROP-045); a keyword list scraped out of the prose would be a guess
 * dressed as metadata.
 */
function keywords(facts: ArticleFacts): string {
  const words = [...facts.audiences];
  if (facts.genre !== undefined) words.push(facts.genre);
  return words.join(", ");
}

function breadcrumbs(
  items: readonly { name: string; item: string }[],
): unknown {
  return {
    "@type": "BreadcrumbList",
    itemListElement: items.map((entry, index) => ({
      "@type": "ListItem",
      position: index + 1,
      name: entry.name,
      item: entry.item,
    })),
  };
}

/** The graph one documentation page publishes about itself. */
export function articleData(facts: ArticleFacts): string {
  return JSON.stringify({
    "@context": "https://schema.org",
    "@graph": [
      {
        "@type": "TechArticle",
        "@id": facts.absolute,
        url: facts.absolute,
        headline: facts.title,
        abstract: facts.summary,
        description: facts.summary,
        inLanguage: facts.language,
        keywords: keywords(facts),
        datePublished: facts.renderedAt,
        image: facts.image,
        author: { "@type": "Organization", name: facts.publisher },
        publisher: { "@type": "Organization", name: facts.publisher },
        isPartOf: {
          "@type": "TechArticle",
          "@id": facts.packageAbsolute,
          name: facts.packageTitle,
        },
      },
      breadcrumbs([
        { name: "Documentation", item: facts.doorAbsolute },
        { name: facts.packageTitle, item: facts.packageAbsolute },
        { name: facts.title, item: facts.absolute },
      ]),
    ],
  });
}

/** The graph a package's own page publishes. */
export function collectionData(facts: CollectionFacts): string {
  return JSON.stringify({
    "@context": "https://schema.org",
    "@graph": [
      {
        "@type": "CollectionPage",
        "@id": facts.absolute,
        url: facts.absolute,
        name: facts.title,
        abstract: facts.abstract,
        description: facts.abstract,
        inLanguage: facts.language,
        image: facts.image,
        author: { "@type": "Organization", name: facts.publisher },
        publisher: { "@type": "Organization", name: facts.publisher },
      },
      breadcrumbs([
        { name: "Documentation", item: facts.doorAbsolute },
        { name: facts.title, item: facts.absolute },
      ]),
    ],
  });
}
