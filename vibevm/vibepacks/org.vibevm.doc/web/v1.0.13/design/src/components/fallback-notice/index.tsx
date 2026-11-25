/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-LANGUAGE-SWITCH-KEEPS-PLACE */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

export type FallbackNoticeProps = {
  /** The line in the language the reader ASKED for. */
  readonly asked: string;
  /** The same line in the language they are getting. */
  readonly given: string;
  readonly dismissLabel: string;
};

/**
 * «This page is only in … so far» — bilingual, and shown once.
 *
 * Bilingual because the reader asked for one language and is being
 * handed another, and a notice in only one of the two is guaranteed to
 * be unreadable to somebody it is for. Once per session, because it is
 * an explanation and not a warning: a reader browsing an adaptation in
 * progress meets it on every second page, and the second telling is
 * already noise.
 *
 * It is a strip in the flow of the page and not a modal. Nothing here
 * needs a decision — the page below it is readable either way — and a
 * dialog over readable text is a door held shut in front of it.
 */
export const FallbackNotice = component$<FallbackNoticeProps>((props) => {
  useStyles$(styles);
  return (
    <aside class="fallback-notice" data-fallback-notice role="note" hidden>
      <p class="fallback-notice__line">{props.asked}</p>
      <p class="fallback-notice__line fallback-notice__line--second">
        {props.given}
      </p>
      <button
        class="fallback-notice__dismiss"
        type="button"
        data-fallback-dismiss
      >
        {props.dismissLabel}
      </button>
    </aside>
  );
});
