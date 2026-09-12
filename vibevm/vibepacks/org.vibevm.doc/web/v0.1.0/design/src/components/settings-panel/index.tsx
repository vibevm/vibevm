/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-SETTINGS */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

export type SettingsPanelProps = {
  /** What the gear is, for a reader who cannot see it. */
  readonly label: string;
};

/**
 * The gear in the corner, the panel it opens, and the quick row that
 * appears once a reader is actually reading.
 *
 * All of it is markup with handles and none of it is behaviour. The
 * behaviour lives in the reader — it has to, because the same controls
 * drive a page whose text the framework never rendered — and a panel the
 * behaviour built out of strings would be a panel with its colours
 * outside the contrast audit and its structure outside review.
 *
 * Three states of theme, not two. `dark` and `light` are the reader's
 * explicit choice and are stamped on the root element; `system` stamps
 * nothing, because the ABSENCE of the attribute is what hands the
 * decision back to the media query. A two-way toggle would quietly
 * remove the default.
 *
 * The column control is hidden on a narrow screen rather than disabled:
 * there is no second column width to choose between on a phone, and a
 * control that cannot do anything is worse than no control.
 */
export const SettingsPanel = component$<SettingsPanelProps>((props) => {
  useStyles$(styles);
  return (
    <div class="settings" data-settings>
      <div class="settings__quick" data-settings-quick>
        <button
          class="settings__quick-button"
          type="button"
          data-step="font"
          data-delta="-1"
          aria-label="Smaller text"
        >
          A−
        </button>
        <button
          class="settings__quick-button"
          type="button"
          data-step="font"
          data-delta="1"
          aria-label="Larger text"
        >
          A+
        </button>
        <button
          class="settings__quick-button"
          type="button"
          data-toggle="anchors"
          aria-pressed="true"
          aria-label="Block numbers"
        >
          NN
        </button>
      </div>
      <button
        class="settings__gear"
        type="button"
        data-settings-toggle
        aria-label={props.label}
        aria-expanded="false"
        aria-controls="reader-settings"
      >
        <span aria-hidden="true">⚙</span>
      </button>
      <div
        class="settings__panel"
        id="reader-settings"
        data-settings-panel
        hidden
      >
        <fieldset class="settings__group">
          <legend>Theme</legend>
          <button
            class="settings__button"
            type="button"
            data-theme-choice="light"
          >
            light
          </button>
          <button
            class="settings__button"
            type="button"
            data-theme-choice="dark"
          >
            dark
          </button>
          <button
            class="settings__button"
            type="button"
            data-theme-choice="system"
          >
            system
          </button>
        </fieldset>
        <fieldset class="settings__group">
          <legend>Text</legend>
          <button
            class="settings__button"
            type="button"
            data-step="font"
            data-delta="-1"
          >
            A−
          </button>
          <output class="settings__value" data-value="font">
            100%
          </output>
          <button
            class="settings__button"
            type="button"
            data-step="font"
            data-delta="1"
          >
            A+
          </button>
        </fieldset>
        <fieldset class="settings__group settings__group--width">
          <legend>Column</legend>
          <button
            class="settings__button"
            type="button"
            data-step="width"
            data-delta="-1"
          >
            W−
          </button>
          <output class="settings__value" data-value="width">
            740
          </output>
          <button
            class="settings__button"
            type="button"
            data-step="width"
            data-delta="1"
          >
            W+
          </button>
        </fieldset>
        <fieldset class="settings__group">
          <legend>Block numbers</legend>
          <button
            class="settings__button"
            type="button"
            data-toggle="anchors"
            aria-pressed="true"
          >
            shown
          </button>
        </fieldset>
        <p class="settings__reset">
          <button class="settings__button" type="button" data-reset>
            reset
          </button>
        </p>
      </div>
    </div>
  );
});
