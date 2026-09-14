/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-DESIGN-FLOOR */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

export type SectionHeadProps = {
  readonly title: string;
  /** Where «see all» leads, already a served path. */
  readonly moreHref: string;
  readonly moreLabel: string;
};

/**
 * A heading with its escape hatch on the right — «Related» and «See
 * all».
 *
 * The pair travels together because a shelf that shows six of forty
 * without saying where the other thirty-four are is a dead end, and a
 * link placed by hand next to each heading is a link somebody will
 * forget.
 */
export const SectionHead = component$<SectionHeadProps>((props) => {
  useStyles$(styles);
  return (
    <div class="section-head">
      <h2 class="section-head__title">{props.title}</h2>
      <a class="section-head__more" href={props.moreHref}>
        {props.moreLabel}
      </a>
    </div>
  );
});
