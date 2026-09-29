/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#SITE-MOUNT */

import { Slot, component$ } from "@qwik.dev/core";

/**
 * The chrome a documentation page carries in BOTH builds.
 *
 * This is the root of the embedded adapter's route tree and a nested
 * layout of the static one, so anything a page needs wherever it is read
 * belongs here — and anything that only makes sense on the public site
 * belongs one level up, in the layout the embedded build never sees.
 *
 * It is thin on purpose, and it stays thin. The table of contents, the
 * version switch, the meta row and the agent surface all belong to one
 * ADDRESS rather than to the tree, and the route that resolves an
 * address is the only place that knows which of them a given address
 * has — the catalogue has no version to switch and no place to return
 * to. So they are rendered there, once, for both readers, and this
 * layout stays what it is: the element a documentation page lives in.
 *
 * A `div` and not an `article`: the catalogue is also under this tree,
 * and a list of packages is not a self-contained composition.
 */
export default component$(() => {
  return (
    <div class="doc-layout">
      <Slot />
    </div>
  );
});
