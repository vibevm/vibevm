/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-META-AND-PRINT */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

export type LightboxProps = {
  readonly label: string;
  readonly closeLabel: string;
};

/**
 * The overlay a picture or a wide table opens into.
 *
 * `position: fixed; inset: 0` and no `backdrop-filter` — the exact
 * combination the reference reader settled on. The inset pair gives
 * precise viewport coverage in every engine with no measuring in script,
 * which `width: 100vw` does not: on some browsers it extends the
 * scrollable area instead. The filter is left out because fixed
 * positioning and `backdrop-filter` together are broken in WebKit, and
 * the blur is worth nothing next to an overlay that does not cover the
 * page.
 *
 * The close button sits BELOW the content rather than over its corner.
 * Over the corner it lands on the picture on a phone, where the picture
 * is the whole width; below it, it is always reachable and never hides
 * what the reader opened.
 *
 * One overlay serves both pictures and tables: they differ in what goes
 * inside, not in how a reader gets out.
 */
export const Lightbox = component$<LightboxProps>((props) => {
  useStyles$(styles);
  return (
    <div
      class="lightbox"
      data-lightbox
      role="dialog"
      aria-modal="true"
      aria-label={props.label}
      hidden
    >
      <div class="lightbox__content">
        <img class="lightbox__image" data-lightbox-image alt="" hidden />
        <div class="lightbox__table" data-lightbox-table hidden />
        <button
          class="lightbox__close"
          type="button"
          data-lightbox-close
          aria-label={props.closeLabel}
        >
          <span aria-hidden="true">×</span>
        </button>
      </div>
    </div>
  );
});
