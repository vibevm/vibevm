/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-RULE-FOLDED */

/**
 * Paper opens every folded rule, and the screen gets it back.
 *
 * A cited rule is a disclosure the pipeline renders closed, which is what
 * spares a reader 802 quotations on 49 pages. On paper that same fold is
 * a block nobody can open: printing keeps everything a reader could
 * CITE and makes it louder, which is why the block numbers and the link
 * addresses survive there (`##READER-META-AND-PRINT`). So the rules are
 * opened for the print and closed again after it.
 *
 * It is behaviour rather than a print rule in the stylesheet because the
 * state is the element's own. `open` is what every engine agrees means
 * «show the content», and it has to be put back: a reader who cancels the
 * dialog, or prints to a file and keeps reading, must find the page as
 * they left it. So the folds this module opened are remembered, and only
 * those are closed — a rule the reader had opened by hand stays open.
 */

import { all, island } from "./dom.ts";

/** Open every closed rule for the print, and restore them after it. */
export function startPrintedRules(): () => void {
  const region = island();
  if (region === null) return () => undefined;

  /* The folds THIS module opened. Held in the closure and not beside it:
     a module-level value would be one memory shared by every page the
     reader visits, and this one belongs to one print of one island. */
  let opened: HTMLElement[] = [];

  const onBeforePrint = (): void => {
    opened = all("details.rule-fold:not([open])", region);
    for (const fold of opened) fold.setAttribute("open", "");
  };

  const onAfterPrint = (): void => {
    for (const fold of opened) fold.removeAttribute("open");
    opened = [];
  };

  window.addEventListener("beforeprint", onBeforePrint);
  window.addEventListener("afterprint", onAfterPrint);

  return () => {
    window.removeEventListener("beforeprint", onBeforePrint);
    window.removeEventListener("afterprint", onAfterPrint);
  };
}
