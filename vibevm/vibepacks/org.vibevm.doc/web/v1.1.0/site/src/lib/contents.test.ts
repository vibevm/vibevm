/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-SHELL-PARSES-NOTHING */

/**
 * What the column beside the text is made of.
 *
 * Every case here is about a rule invisible in a screenshot and expensive
 * to be wrong about: which pages stand first, which folder a page falls
 * into, what that folder is called in a language nobody wrote a title for,
 * and whether a translation's own words reach the headings. The second
 * half is the other order the same pages have — the declared learning
 * path — and its rules are of the same kind: a number that must not count
 * an appendix, a chapter name a translation has not given, and which page
 * the one before it is.
 */

import { strict as assert } from "node:assert";
import { describe, it } from "node:test";

import { contentsOf, learningPath, neighboursOf } from "./contents.ts";
import { librariesOf } from "./library.ts";
import {
  Audience,
  DocumentationStatus,
  PageGenre,
  TranslationStatus,
  type DocManifest,
  type DocPage,
  type Navigation,
  type NavigationChapter,
} from "../generated/doc-manifest.ts";

import type { PagerLink } from "@vibe-docs/design";

/** One page of a fixture manifest; only the fields the column reads vary. */
function page(path: string, title: string): DocPage {
  return {
    path,
    title,
    genre: PageGenre.Concept,
    audiences: [Audience.Dev],
    anchors: [],
    summary: "",
    reading_time_min: 1,
  };
}

/** One manifest, at the smallest shape a library is made of. */
function manifest(
  overrides: {
    readonly name?: string;
    readonly lang?: string;
    readonly translates?: string;
    readonly navigation?: Navigation;
  },
  ...pages: DocPage[]
): DocManifest {
  const lang = overrides.lang ?? "en";
  return {
    schema_version: 1,
    package: {
      group: "com.example.docs",
      name: overrides.name ?? "manual",
      version: "0.1.0",
      publisher: "com.example.docs",
      title: "A Manual",
      abstract: "",
      lang,
      status: DocumentationStatus.Primary,
      subjects: [],
      audiences: [Audience.Dev],
      rendered_at: "2026-01-01T00:00:00Z",
      ...(overrides.translates === undefined
        ? {}
        : {
            translation: {
              package: overrides.translates,
              version: "0.1.0",
              status: TranslationStatus.Official,
            },
          }),
    },
    pages,
    ...(overrides.navigation === undefined
      ? {}
      : { navigation: overrides.navigation }),
  };
}

const PAGES = [
  page("start/index.xml", "Newcomer's Path"),
  page("start/what-vibevm-is.xml", "What VibeVM is"),
  page("model/lock-and-store.xml", "Lock and store"),
  page("readme.xml", "Read me"),
];

function libraryOf(...manifests: DocManifest[]) {
  const [only] = librariesOf(manifests);
  assert.ok(only !== undefined, "the manifests describe a library");
  return only;
}

describe("the manual's pages as the column lists them", () => {
  it("groups them by folder, in the order the manifest gave them", () => {
    const contents = contentsOf(libraryOf(manifest({}, ...PAGES)), null, null);

    assert.deepEqual(contents.pinned, []);
    assert.deepEqual(
      contents.sections.map((section) => [
        section.id,
        section.title,
        section.items.map((item) => item.label),
      ]),
      [
        ["start", "Start", ["Newcomer's Path", "What VibeVM is"]],
        ["model", "Model", ["Lock and store"]],
        ["", "", ["Read me"]],
      ],
      "the folders stand in the order their first page does, and a page at the root of the tree is in a group with no heading",
    );
  });

  it("puts the pinned pages first and outside their folders", () => {
    const navigation: Navigation = {
      pinned: ["start/what-vibevm-is", "start/index"],
      sections: [],
    };
    const contents = contentsOf(
      libraryOf(manifest({ navigation }, ...PAGES)),
      null,
      null,
    );

    assert.deepEqual(
      contents.pinned.map((item) => item.label),
      ["What VibeVM is", "Newcomer's Path"],
      "in the order the package named them, which is not the manifest's",
    );
    assert.deepEqual(
      contents.sections.map((section) => section.id),
      ["model", ""],
      "the folder both pinned pages came from is gone with them",
    );
  });

  it("passes over a pin that names no page of this documentation", () => {
    const navigation: Navigation = {
      pinned: ["start/moved-away"],
      sections: [],
    };
    const contents = contentsOf(
      libraryOf(manifest({ navigation }, ...PAGES)),
      null,
      null,
    );
    assert.deepEqual(contents.pinned, []);
    assert.equal(
      contents.sections.flatMap((section) => section.items).length,
      PAGES.length,
      "and the page it does not name stays where the manifest put it",
    );
  });

  it("shows a folder under the title the reader's edition gives it", () => {
    const source = manifest(
      {
        navigation: {
          pinned: [],
          sections: [{ id: "start", title: "Getting started" }],
        },
      },
      ...PAGES,
    );
    const adapted = manifest(
      {
        name: "manual-ru",
        lang: "ru",
        translates: "com.example.docs/manual",
        navigation: {
          pinned: [],
          sections: [{ id: "start", title: "С чего начать" }],
        },
      },
      ...PAGES,
    );
    const library = libraryOf(source, adapted);

    const titleAt = (at: string | null): string | undefined =>
      contentsOf(library, at, null).sections.find(
        (section) => section.id === "start",
      )?.title;

    assert.equal(titleAt(null), "Getting started");
    assert.equal(titleAt("ru"), "С чего начать");
  });

  /**
   * The middle rung of the same ladder. An adaptation that has not named a
   * folder shows the SOURCE's words for it, not the directory's: the
   * source said something true about that group, and falling all the way
   * to the folder name would throw it away to show a bare word.
   */
  it("falls to the source's words before it falls to the folder's name", () => {
    const source = manifest(
      {
        navigation: {
          pinned: [],
          sections: [
            { id: "start", title: "Getting started" },
            { id: "model", title: "The model" },
          ],
        },
      },
      ...PAGES,
    );
    const adapted = manifest(
      {
        name: "manual-ru",
        lang: "ru",
        translates: "com.example.docs/manual",
        navigation: {
          pinned: [],
          sections: [{ id: "start", title: "С чего начать" }],
        },
      },
      ...PAGES,
    );
    const sections = contentsOf(libraryOf(source, adapted), "ru", null);

    assert.deepEqual(
      sections.sections.map((section) => [section.id, section.title]),
      [
        ["start", "С чего начать"],
        ["model", "The model"],
        ["", ""],
      ],
    );
  });

  it("marks the page the reader is on, and only that one", () => {
    const contents = contentsOf(
      libraryOf(manifest({}, ...PAGES)),
      null,
      "model/lock-and-store",
    );
    const marked = contents.sections
      .flatMap((section) => section.items)
      .filter((item) => item.current)
      .map((item) => item.label);
    assert.deepEqual(marked, ["Lock and store"]);
  });

  /**
   * The list is the source's and the words are the reader's edition's.
   *
   * The two are separate answers and were being given as one: the column
   * printed the source's title over every link, so an adaptation that had
   * the page and had named it in its own language was overruled by the
   * manual it adapts. Where the adaptation has NOT reached a page, the
   * source's words are the right ones — that is the text the address
   * serves, and a label must say what opening it will show.
   */
  it("names a page in the words of the edition that will serve it", () => {
    const library = libraryOf(
      manifest({}, ...PAGES),
      manifest(
        {
          name: "manual-ru",
          lang: "ru",
          translates: "com.example.docs/manual",
        },
        page("start/index.xml", "Путь новичка"),
      ),
    );
    const labels = (at: string | null): string[] =>
      contentsOf(library, at, null)
        .sections.flatMap((section) => section.items)
        .map((item) => item.label);

    assert.deepEqual(labels(null), [
      "Newcomer's Path",
      "What VibeVM is",
      "Lock and store",
      "Read me",
    ]);
    assert.deepEqual(
      labels("ru"),
      ["Путь новичка", "What VibeVM is", "Lock and store", "Read me"],
      "the one page the adaptation carries is named in its own words, and " +
        "the three it has not reached keep the source's, which is the text " +
        "those addresses serve",
    );
  });

  it("names a pinned page the same way", () => {
    const pinned = contentsOf(
      libraryOf(
        manifest(
          { navigation: { pinned: ["start/index"], sections: [] } },
          ...PAGES,
        ),
        manifest(
          {
            name: "manual-ru",
            lang: "ru",
            translates: "com.example.docs/manual",
          },
          page("start/index.xml", "Путь новичка"),
        ),
      ),
      "ru",
      null,
    ).pinned;

    assert.deepEqual(
      pinned.map((item) => item.label),
      ["Путь новичка"],
      "standing first is about order and says nothing about the words",
    );
  });

  it("lists the source's pages under the adaptation's addresses", () => {
    const library = libraryOf(
      manifest({}, ...PAGES),
      manifest(
        {
          name: "manual-ru",
          lang: "ru",
          translates: "com.example.docs/manual",
        },
        page("start/index.xml", "Путь новичка"),
      ),
    );
    const contents = contentsOf(library, "ru", null);
    const items = contents.sections.flatMap((section) => section.items);

    assert.equal(
      items.length,
      PAGES.length,
      "an adaptation in progress is not a smaller manual",
    );
    for (const item of items) {
      assert.ok(
        item.href.startsWith("/doc/ru/com.example.docs/manual/"),
        `${item.href} keeps the reader in their language`,
      );
    }
  });
});

/**
 * The learning path over the same four pages (`##NAV-CHAPTERS`). The
 * appendix stands in the MIDDLE of these rows on purpose: it is the only
 * arrangement that tells «an appendix carries no number» apart from «an
 * appendix is last and has no number after it», and a count that skipped
 * the wrong row would look like the second.
 */
function declaring(...chapters: NavigationChapter[]): Navigation {
  return { pinned: [], sections: [], chapters };
}

const CHAPTERS = declaring(
  {
    id: "first-steps",
    title: "Getting started",
    pages: ["start/what-vibevm-is", "start/index"],
  },
  {
    id: "reference",
    title: "Look things up",
    pages: ["readme"],
    appendix: true,
  },
  { id: "model", title: "How it works", pages: ["model/lock-and-store"] },
);

/** The same path, named in Russian — except for the appendix. */
const RUSSIAN_CHAPTERS = declaring(
  { id: "first-steps", title: "Первые шаги", pages: [] },
  { id: "model", title: "Как это устроено", pages: [] },
);

/** One library whose source declares a path. */
function withPath(navigation: Navigation = CHAPTERS) {
  return libraryOf(manifest({ navigation }, ...PAGES));
}

/** The same source with an adaptation beside it, carrying one page. */
function adapted(navigation: Navigation | undefined) {
  return libraryOf(
    manifest({ navigation: CHAPTERS }, ...PAGES),
    manifest(
      {
        name: "manual-ru",
        lang: "ru",
        translates: "com.example.docs/manual",
        ...(navigation === undefined ? {} : { navigation }),
      },
      page("start/index.xml", "Путь новичка"),
    ),
  );
}

describe("the learning path the documentation declared", () => {
  it("numbers the chapters in order and leaves an appendix unnumbered", () => {
    const path = learningPath(withPath(), null);
    assert.deepEqual(
      path?.map((chapter) => [chapter.number, chapter.title]),
      [
        ["1", "Getting started"],
        ["", "Look things up"],
        ["2", "How it works"],
      ],
      "an appendix takes no place in the count: the chapter after it keeps the number it would have had",
    );
    assert.deepEqual(
      path?.map((chapter) => chapter.appendix),
      [false, true, false],
    );
  });

  it("holds the pages the author named, in the order they were named", () => {
    assert.deepEqual(
      learningPath(withPath(), null)?.map((one) =>
        one.pages.map((page) => page.document),
      ),
      [
        ["start/what-vibevm-is", "start/index"],
        ["readme"],
        ["model/lock-and-store"],
      ],
      "which is not the manifest's order, and is not sorted here either",
    );
  });

  it("tells a documentation that declared none from one that declared nothing", () => {
    const silent = libraryOf(manifest({}, ...PAGES));
    assert.equal(learningPath(silent, null), null);
    assert.deepEqual(contentsOf(silent, null, null).chapters, []);
    assert.equal(neighboursOf(silent, null, "start/index"), null);
    // A package that opened the table and named no chapter said something
    // else: it arrives as the empty path it wrote.
    assert.deepEqual(learningPath(withPath(declaring()), null), []);
  });

  it("passes over a chapter's page that names no page of this documentation", () => {
    const library = withPath(
      declaring({
        id: "first-steps",
        title: "Getting started",
        pages: ["start/index", "start/moved-away"],
      }),
    );
    assert.deepEqual(
      learningPath(library, null)?.[0]?.pages.map((one) => one.document),
      ["start/index"],
    );
  });

  it("marks the page the reader is on in this view too, and only that one", () => {
    const marked = contentsOf(withPath(), null, "model/lock-and-store")
      .chapters.flatMap((chapter) => chapter.items)
      .filter((item) => item.current)
      .map((item) => item.label);
    assert.deepEqual(marked, ["Lock and store"]);
  });

  it("names a chapter in the reader's edition's words, falling back to the source's", () => {
    const path = learningPath(adapted(RUSSIAN_CHAPTERS), "ru");
    assert.deepEqual(
      path?.map((chapter) => chapter.title),
      ["Первые шаги", "Look things up", "Как это устроено"],
      "the adaptation named two of the three; the one it did not keeps the source's words, having no directory to fall back on",
    );
  });

  it("takes the source's path whole when the translation names no chapter", () => {
    const path = learningPath(adapted(undefined), "ru");
    assert.deepEqual(
      path?.map((chapter) => [chapter.number, chapter.title]),
      [
        ["1", "Getting started"],
        ["", "Look things up"],
        ["2", "How it works"],
      ],
    );
    const pages = path?.flatMap((chapter) => chapter.pages) ?? [];
    assert.deepEqual(
      pages.map((one) => one.label),
      ["What VibeVM is", "Путь новичка", "Read me", "Lock and store"],
      "and the words are the serving edition's, in this view as in the other",
    );
    // An adaptation in progress is not a shorter path, and every page of it leads into the language being read.
    assert.equal(pages.length, 4);
    for (const one of pages) {
      assert.ok(
        one.href.startsWith("/doc/ru/com.example.docs/manual/"),
        `${one.href} keeps the reader in their language`,
      );
    }
  });
});

describe("where the path leads from one page of it", () => {
  /** The two neighbours as «title, number, chapter»; `null` for none. */
  type Said = readonly [string, string, string] | null;
  const say = (link: PagerLink | undefined): Said =>
    link === undefined
      ? null
      : [link.title, link.chapter?.number ?? "", link.chapter?.title ?? ""];

  function around(document: string): readonly Said[] {
    const found = neighboursOf(withPath(), null, document);
    assert.ok(found !== null, `${document} stands on the path`);
    return [say(found.previous), say(found.next)];
  }

  it("offers only the way on from the first page", () => {
    assert.deepEqual(around("start/what-vibevm-is"), [
      null,
      ["Newcomer's Path", "", ""],
    ]);
  });

  it("offers only the way back from the last", () => {
    assert.deepEqual(around("model/lock-and-store"), [
      ["Read me", "", "Look things up"],
      null,
    ]);
  });

  it("names the chapter only where the path crosses into another one", () => {
    assert.deepEqual(
      around("start/index"),
      [
        ["What VibeVM is", "", ""],
        ["Read me", "", "Look things up"],
      ],
      "the page before shares its chapter and is named alone; the page after opens the appendix, captioned and unnumbered",
    );
    assert.deepEqual(
      around("readme"),
      [
        ["Newcomer's Path", "1", "Getting started"],
        ["Lock and store", "2", "How it works"],
      ],
      "both neighbours of the appendix stand in numbered chapters",
    );
  });

  it("answers nothing for a page the declared path does not reach", () => {
    const library = withPath({
      pinned: [],
      sections: [],
      chapters: [
        { id: "first-steps", title: "Getting started", pages: ["start/index"] },
      ],
    });
    assert.equal(neighboursOf(library, null, "readme"), null);
  });

  it("leads to the addresses and the words of the edition being read", () => {
    const found = neighboursOf(
      adapted(RUSSIAN_CHAPTERS),
      "ru",
      "start/what-vibevm-is",
    );
    assert.equal(
      found?.next?.href,
      "/doc/ru/com.example.docs/manual/0.1.0/start/index/",
    );
    assert.equal(found?.next?.title, "Путь новичка");
  });

  it("captions a crossing in the words of the edition being read", () => {
    const found = neighboursOf(adapted(RUSSIAN_CHAPTERS), "ru", "readme");
    assert.deepEqual(found?.previous?.chapter, {
      number: "1",
      title: "Первые шаги",
    });
    assert.deepEqual(found?.next?.chapter, {
      number: "2",
      title: "Как это устроено",
    });
  });
});
