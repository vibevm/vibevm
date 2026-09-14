/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-SHELL-PARSES-NOTHING */

/**
 * What the column beside the text is made of.
 *
 * Every case here is about a rule that is invisible in a screenshot and
 * expensive to be wrong about: which pages stand first, which folder a
 * page falls into, what that folder is called in a language nobody wrote
 * a title for, and whether a translation's own words reach the headings.
 */

import { strict as assert } from "node:assert";
import { describe, it } from "node:test";

import { contentsOf } from "./contents.ts";
import { librariesOf } from "./library.ts";
import {
  Audience,
  DocumentationStatus,
  PageGenre,
  TranslationStatus,
  type DocManifest,
  type DocPage,
  type Navigation,
} from "../generated/doc-manifest.ts";

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
      "the folders stand in the order their first page does, and a page " +
        "at the root of the tree is in a group with no heading",
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
   * The middle rung of the same ladder. An adaptation that has not named
   * a folder yet shows the SOURCE's words for it, not the directory's:
   * the source said something true about that group, and falling all the
   * way to the folder name would throw it away to show a bare word.
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
   * printed the source's title over every link, so an adaptation that
   * had the page and had named it in its own language was overruled by
   * the manual it adapts. Where the adaptation has NOT reached a page,
   * the source's words are the right ones — that is the text the address
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
