/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-CITATIONS-RESOLVED */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

export type RulePanelProps = {
  /** What the panel is, for a reader who cannot see it appear. */
  readonly label: string;
  readonly copyLabel: string;
  readonly openLabel: string;
  readonly closeLabel: string;
};

/**
 * The panel a quoted rule opens beside itself.
 *
 * It is rendered empty, once per page, and filled from the rule that was
 * clicked — because the rule's words are already in the island. The
 * pipeline resolved the citation at build time and put the fact's text
 * there with its `spec://` address on `data-uri`, so showing it costs a
 * read of the DOM and no request at all. That is what makes the same
 * page work in the local reader, offline, and behind a firewall.
 *
 * The markup carries the handles (`data-rule-*`) and no behaviour. A
 * panel the behaviour had to create would be a panel with its styles in
 * a string, unreviewable and invisible to the contrast audit.
 */
export const RulePanel = component$<RulePanelProps>((props) => {
  useStyles$(styles);
  return (
    <aside class="rule-panel" data-rule-panel aria-label={props.label} hidden>
      <p class="rule-panel__uri">
        <code data-rule-uri />
      </p>
      <p class="rule-panel__text" data-rule-text />
      <p class="rule-panel__actions">
        <button class="rule-panel__button" type="button" data-rule-copy>
          {props.copyLabel}
        </button>
        <a class="rule-panel__button" href="#" data-rule-link>
          {props.openLabel}
        </a>
        <button class="rule-panel__button" type="button" data-rule-close>
          {props.closeLabel}
        </button>
      </p>
    </aside>
  );
});
