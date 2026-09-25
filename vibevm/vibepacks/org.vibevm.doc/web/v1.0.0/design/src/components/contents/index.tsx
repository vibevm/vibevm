/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-DESIGN-FLOOR */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

/** One page of the manual, as the column lists it. */
export type ContentsItem = {
  readonly label: string;
  /** Already a served path: the caller builds it with `href()`. */
  readonly href: string;
  /** The page the reader is on, marked for the eye and for a screen reader. */
  readonly current: boolean;
};

/** One folder of the page tree, under the name a reader sees. */
export type ContentsSection = {
  /** The folder as the page paths spell it; the key, never the name. */
  readonly id: string;
  /**
   * What the navigation shows over the group. Empty for the pages that
   * live at the root of the tree and are in no folder at all: a heading
   * over them would have to be invented, and the group is drawn without
   * one instead.
   */
  readonly title: string;
  readonly items: ReadonlyArray<ContentsItem>;
};

/**
 * One chapter of the declared learning path
 * (`spec://org.vibevm.core/vibevm/common/PROP-057#NAV-CHAPTERS-READER`).
 *
 * A chapter is not a folder. The path is written by the author out of
 * whichever pages a lesson needs, so a chapter names pages from any
 * number of directories and the `id` is its own word rather than a path
 * segment.
 */
export type ContentsChapter = {
  /** The chapter's identity in the package; the key, never the name. */
  readonly id: string;
  /**
   * The number the reader sees over it, `1`. Empty for an appendix
   * chapter, which is pages a reader looks things up in rather than reads
   * through and therefore carries no place in the count.
   */
  readonly number: string;
  readonly title: string;
  readonly items: ReadonlyArray<ContentsItem>;
};

export type ContentsProps = {
  /** What the column is called, said once and read aloud: «Contents». */
  readonly label: string;
  /**
   * The pages the documentation asked to stand first, outside every
   * group. Empty when it asked for none, which is the usual case.
   */
  readonly pinned: ReadonlyArray<ContentsItem>;
  readonly sections: ReadonlyArray<ContentsSection>;
  /**
   * The declared learning path, in the order a reader walks it. Empty
   * when the documentation declared none — and then the column is the
   * sections alone, with no switch over it, exactly as it was before a
   * path could be declared.
   */
  readonly chapters: ReadonlyArray<ContentsChapter>;
  /** What the switch over the two views is called: «Contents view». */
  readonly viewLabel: string;
  /** The button that shows the path: «In order». */
  readonly pathLabel: string;
  /** The button that shows the folders: «By section». */
  readonly sectionsLabel: string;
};

/**
 * The manual's own pages, as a column beside the text.
 *
 * It replaces the row of tabs that used to stand under the header. A row
 * holds about six entries at a readable size and then starts scrolling
 * sideways; a manual of thirty pages in eleven folders put every page
 * after the sixth behind a gesture, and told a reader nothing about
 * which part of the manual they were in. A column has room for the whole
 * list, and the folder headings are the shape of the manual said out
 * loud.
 *
 * **One element, two shapes, and the narrow one needs no script.** A
 * `<details>` that is closed is the block a phone gets — a line saying
 * «Contents» that opens when it is tapped, by the browser and nothing
 * else. The same element on a wide screen is the column: the summary is
 * taken away and the content is forced open by the stylesheet, which is
 * the one thing about the shape that the page decides rather than the
 * reader. Two elements would be two lists to keep in step, and the one
 * nobody was looking at would go stale.
 *
 * **Two views of one manual, and both are always in the document.** A
 * documentation that declared a learning path is shown as that path by
 * default and as its folders on request (`##NAV-CHAPTERS-READER`), which
 * is a choice the reader makes and keeps. So the server writes both and
 * the stylesheet shows one, by an attribute a script stamps on the root
 * element before the first stylesheet is parsed — the same arrangement
 * the theme has, and for the same reason: a view that appeared and then
 * changed would be the page telling the reader their setting did not
 * take. Nothing here is hydrated, and a documentation with no path
 * renders neither the switch nor the second pane.
 *
 * Links, never buttons: every entry is a place with an address, and a
 * reader who middle-clicks one must get a tab. `aria-current="page"` and
 * not only a class, because the mark is information and a colour is not
 * readable aloud. The switch is the opposite case and is therefore two
 * buttons: choosing a view changes no address, and a reader who copies
 * the address must not hand someone else their own reading habit.
 *
 * The order is the manifest's and this component does not sort. Which
 * pages stand first, what their folders are called and which chapter
 * holds which page are the documentation's own statements
 * (`##NAV-PINNED`, `##NAV-CHAPTERS`), worked out where the manifest is
 * read; a column that re-ordered what it was handed would be a second
 * opinion about the layer law.
 */
export const Contents = component$<ContentsProps>((props) => {
  useStyles$(styles);
  const path = props.chapters.length > 0;
  return (
    <details class="contents" data-contents>
      <summary class="contents__summary">{props.label}</summary>
      <nav
        class={path ? "contents__nav contents__nav--path" : "contents__nav"}
        aria-label={props.label}
      >
        {!path ? null : (
          <div
            class="contents__views"
            role="group"
            aria-label={props.viewLabel}
            data-contents-views
          >
            {/* The marked button is decided by the stylesheet out of the
                root's attribute, so the pressed pill and the pane below
                it cannot disagree at the first frame. `aria-pressed` is
                written here for the default view and corrected by the
                reader for a stored one — it is read aloud rather than
                seen, so a late correction costs nothing. */}
            <button
              class="contents__view"
              type="button"
              data-contents-choice="path"
              aria-pressed="true"
            >
              {props.pathLabel}
            </button>
            <button
              class="contents__view"
              type="button"
              data-contents-choice="sections"
              aria-pressed="false"
            >
              {props.sectionsLabel}
            </button>
          </div>
        )}
        {!path ? null : (
          <div class="contents__pane contents__pane--path" data-contents-path>
            {props.chapters.map((chapter) => (
              <div key={chapter.id} class="contents__section">
                <h2 class="contents__heading">
                  {chapter.number.length === 0 ? null : (
                    <span class="contents__number">{chapter.number}</span>
                  )}
                  {chapter.title}
                </h2>
                <ul class="contents__list">
                  {chapter.items.map((item) => (
                    <Entry key={item.href} item={item} />
                  ))}
                </ul>
              </div>
            ))}
          </div>
        )}
        {/* The folders, unchanged: the pinned pages first and outside
            every group, then one group per folder. It is the whole of
            the column for a documentation with no path, and the second
            view for one that has it. */}
        <div
          class="contents__pane contents__pane--sections"
          data-contents-sections
        >
          {props.pinned.length === 0 ? null : (
            <ul class="contents__list">
              {props.pinned.map((item) => (
                <Entry key={item.href} item={item} />
              ))}
            </ul>
          )}
          {props.sections.map((section) => (
            <div key={section.id} class="contents__section">
              {section.title.length === 0 ? null : (
                <h2 class="contents__heading">{section.title}</h2>
              )}
              <ul class="contents__list">
                {section.items.map((item) => (
                  <Entry key={item.href} item={item} />
                ))}
              </ul>
            </div>
          ))}
        </div>
      </nav>
    </details>
  );
});

/** One entry, written once for the pinned pages and for the groups. */
const Entry = component$<{ item: ContentsItem }>((props) => (
  <li class="contents__item">
    <a
      class={
        props.item.current
          ? "contents__link contents__link--current"
          : "contents__link"
      }
      href={props.item.href}
      {...(props.item.current ? { "aria-current": "page" as const } : {})}
    >
      {props.item.label}
    </a>
  </li>
));
