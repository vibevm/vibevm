/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-DESIGN-FLOOR */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

export type TagProps = {
  readonly label: string;
};

/**
 * A keyword or an audience, printed small and flat.
 *
 * A tag is not a badge and the difference is not decoration: a badge
 * carries a computed standing that the reader is meant to act on, a tag
 * carries a word the page declared about itself. Keeping them apart
 * keeps a page from looking as if it had been awarded something.
 */
export const Tag = component$<TagProps>((props) => {
  useStyles$(styles);
  return <span class="tag">{props.label}</span>;
});
