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
 * It is thin on purpose. The table of contents, the version switch, the
 * language selector and the agent surface are each their own atom, and
 * each will land here once, for both readers, rather than twice.
 */
export default component$(() => {
  return (
    <article class="doc-layout">
      <Slot />
    </article>
  );
});
