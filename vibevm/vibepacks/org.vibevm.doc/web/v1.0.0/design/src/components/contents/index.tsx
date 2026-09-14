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

export type ContentsProps = {
  /** What the column is called, said once and read aloud: «Contents». */
  readonly label: string;
  /**
   * The pages the documentation asked to stand first, outside every
   * group. Empty when it asked for none, which is the usual case.
   */
  readonly pinned: ReadonlyArray<ContentsItem>;
  readonly sections: ReadonlyArray<ContentsSection>;
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
 * Links, never buttons: every entry is a place with an address, and a
 * reader who middle-clicks one must get a tab. `aria-current="page"` and
 * not only a class, because the mark is information and a colour is not
 * readable aloud.
 *
 * The order is the manifest's and this component does not sort. Which
 * pages stand first and what their folders are called is the
 * documentation's own statement (`##NAV-PINNED`), worked out where the
 * manifest is read; a column that re-ordered what it was handed would be
 * a second opinion about the layer law.
 */
export const Contents = component$<ContentsProps>((props) => {
  useStyles$(styles);
  return (
    <details class="contents" data-contents>
      <summary class="contents__summary">{props.label}</summary>
      <nav class="contents__nav" aria-label={props.label}>
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
