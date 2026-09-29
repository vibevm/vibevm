/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-NUMBERING */

/**
 * The platform switch: which `when` blocks of the island a reader sees.
 *
 * Every conditional block is in the page and every one of them is
 * numbered before any is hidden — that is the pipeline's rule, not a
 * convenience, and it is what makes `p12` name the same block on Windows
 * and on Linux and in every adaptation (`##PIPE-NUMBERING`). The switch
 * may therefore HIDE a block and may never remove or renumber one, so
 * this module sets `hidden` and touches nothing else. Gaps in the
 * visible numbering are the accepted price of that rule.
 *
 * Only the `os:` family is the switch's business. A block conditioned on
 * an agent is not a platform variant, and hiding it here would make the
 * switch quietly the arbiter of a second axis it has no control for.
 */

import { all, island, one } from "./dom.ts";
import { readLocal, writeLocal } from "./storage.ts";

const KEY = "platform";
const FAMILY = "os:";

function pills(): HTMLElement[] {
  return all("[data-when-switch] button[value]");
}

/** Which platforms the page actually offers, in the order it offers them. */
function offered(): string[] {
  const out: string[] = [];
  for (const pill of pills()) {
    const value = pill.getAttribute("value");
    if (value !== null && value.length > 0) out.push(value);
  }
  return out;
}

function markPills(chosen: string): void {
  for (const pill of pills()) {
    const current = pill.getAttribute("value") === chosen;
    pill.setAttribute("aria-pressed", current ? "true" : "false");
    pill.classList.toggle("tab-pills__pill--current", current);
  }
}

function markBlocks(chosen: string): void {
  const region = island();
  if (region === null) return;
  for (const block of all("[data-when]", region)) {
    const when = block.dataset["when"] ?? "";
    if (!when.startsWith(FAMILY)) continue;
    block.hidden = when !== FAMILY + chosen;
  }
}

function apply(chosen: string): void {
  markPills(chosen);
  markBlocks(chosen);
}

/**
 * Wire the switch and restore the reader's platform.
 *
 * The choice is remembered but never put in the address: a reader who
 * copies the page's URL must not hand someone else their own operating
 * system, which is why the pills are buttons and the page's address
 * says nothing about them.
 */
export function startPlatformSwitch(): () => void {
  const group = one("[data-when-switch]");
  if (group === null) return () => undefined;

  const available = offered();
  if (available.length === 0) return () => undefined;

  const stored = readLocal(KEY);
  const first = available[0] ?? "";
  const chosen = stored !== null && available.includes(stored) ? stored : first;
  apply(chosen);

  const onClick = (event: Event): void => {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const pill = target.closest("button[value]");
    if (pill === null || !group.contains(pill)) return;
    const value = pill.getAttribute("value");
    if (value === null) return;
    writeLocal(KEY, value);
    apply(value);
  };

  group.addEventListener("click", onClick);
  return () => group.removeEventListener("click", onClick);
}
