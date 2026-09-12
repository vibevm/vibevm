/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-ONE-SITE */

import { Slot, component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

export type FooterProps = {
  /** The copyright line, already composed by the caller in its language. */
  readonly copyright: string;
};

/**
 * The catalogue footer, shared by the landing and the documentation —
 * one footer for one site, which is the whole reason both live in one
 * application.
 *
 * It sinks below the page's ground rather than floating on it: the
 * footer is the end of the page, and the eye should be able to tell that
 * without reading anything.
 */
export const Footer = component$<FooterProps>((props) => {
  useStyles$(styles);
  return (
    <footer class="footer">
      <div class="footer__inner">
        <div class="footer__columns">
          <Slot />
        </div>
        <p class="footer__copyright">{props.copyright}</p>
      </div>
    </footer>
  );
});
