/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-MOUNT */

import { Slot, component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

/**
 * The strip of controls that belongs to the DOCUMENTATION rather than to
 * the site: which language of it a reader wants offered, and which
 * version they are reading.
 *
 * It stands under the site's own header and above whatever the address
 * resolved to, because that is the order of the two questions — the site
 * is the same on every address and the documentation is not. The site's
 * header keeps what is the site's: the brand, the search over the whole
 * corpus, and the language the furniture speaks.
 *
 * It is a strip in the SITE and not a component of the design system.
 * What hangs in it is decided per address by the route that resolved the
 * address — the catalogue has no version to switch, a page has — and a
 * component that took the address and worked out what to show would be
 * the route's decision moved into the design system, where it could not
 * be tested without one.
 */
export const DocBar = component$(() => {
  useStyles$(styles);
  return (
    <div class="doc-bar">
      <Slot />
    </div>
  );
});
