/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#NAV-CHAPTERS-READER */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

/**
 * The chapter a neighbour stands in, printed over its title when it is
 * not the chapter the reader is in.
 *
 * It is two values and not one composed string because one of them is
 * the documentation's words and the other is the interface's: the title
 * belongs to the package and is shown in the package's language, while
 * the word «Chapter» is furniture and moves with the reader's own choice
 * of interface language. A sentence assembled here would be one string
 * the site's language table could not move without moving the title with
 * it.
 */
export type PagerChapter = {
  /**
   * The chapter's number, `3`. Absent for an appendix chapter, which
   * carries no number anywhere (`##NAV-CHAPTERS-READER`) and is named by
   * its title alone.
   */
  readonly number?: string;
  readonly title: string;
};

/** One neighbour of this page on the learning path. */
export type PagerLink = {
  /** Already a served path: the caller builds it with `href()`. */
  readonly href: string;
  /** The page's own title, in the words of the edition being read. */
  readonly title: string;
  /** Absent when the neighbour stands in the reader's own chapter. */
  readonly chapter?: PagerChapter;
};

export type PagerProps = {
  /** What the pair is called, read aloud: «The learning path». */
  readonly label: string;
  /** The word a chapter caption opens with: «Chapter». */
  readonly chapterWord: string;
  /** What the way back is called: «Previous». */
  readonly previousLabel: string;
  /** And the way on: «Next». */
  readonly nextLabel: string;
  /** Absent on the first page of the path. */
  readonly previous?: PagerLink;
  /** Absent on the last. */
  readonly next?: PagerLink;
};

/**
 * Where the path leads from here: the page before this one and the page
 * after it (`##NAV-CHAPTERS-READER`).
 *
 * It is the last thing in the reading column, after everything the page
 * itself carries, because that is where a reader who has finished
 * reading arrives — and a reader who has finished a page of a textbook is
 * asking one question. The first page of the path offers only the way
 * on and the last only the way back; neither draws an empty card in the
 * other's place, because a card that led nowhere would be a promise the
 * path cannot keep.
 *
 * The chapter caption appears only when the neighbour is in ANOTHER
 * chapter, which is the moment it says something: inside a chapter every
 * page shares it, and printing it on each would be a heading repeated
 * once per page.
 *
 * Two links and no buttons, for the reason every other list of places
 * here is links: the neighbour is a page with an address, and a reader
 * who middle-clicks it must get a tab. `rel` says the same thing to a
 * machine that the arrow says to a reader.
 */
export const Pager = component$<PagerProps>((props) => {
  useStyles$(styles);
  return (
    <nav class="pager" aria-label={props.label} data-pager>
      {props.previous === undefined ? null : (
        <a
          class="pager__link pager__link--previous"
          href={props.previous.href}
          rel="prev"
          data-pager-previous
        >
          <span class="pager__way">
            <span class="pager__arrow" aria-hidden="true">
              ←
            </span>
            <span class="pager__word">{props.previousLabel}</span>
          </span>
          <Caption word={props.chapterWord} chapter={props.previous.chapter} />
          <span class="pager__title">{props.previous.title}</span>
        </a>
      )}
      {props.next === undefined ? null : (
        <a
          class="pager__link pager__link--next"
          href={props.next.href}
          rel="next"
          data-pager-next
        >
          <span class="pager__way">
            <span class="pager__word">{props.nextLabel}</span>
            <span class="pager__arrow" aria-hidden="true">
              →
            </span>
          </span>
          <Caption word={props.chapterWord} chapter={props.next.chapter} />
          <span class="pager__title">{props.next.title}</span>
        </a>
      )}
    </nav>
  );
});

/**
 * The chapter over a neighbour's title, written once for both ways.
 *
 * The word stands in an element of its own so that it is a text node of
 * its own: the site's language table moves whole strings, and «Chapter»
 * has to be able to move without the number and the package's title
 * moving with it.
 */
const Caption = component$<{
  word: string;
  /** Declared as «may be absent» rather than optional, because that is
      what the caller has: a neighbour's chapter is read off an optional
      property, and `exactOptionalPropertyTypes` keeps the two apart. */
  chapter: PagerChapter | undefined;
}>((props) => {
  const chapter = props.chapter;
  if (chapter === undefined) return null;
  return (
    <span class="pager__chapter">
      {chapter.number === undefined ? null : (
        <>
          <span class="pager__chapter-word">{props.word}</span>{" "}
          <span class="pager__chapter-number">{chapter.number}</span>
          {" · "}
        </>
      )}
      {chapter.title}
    </span>
  );
});
