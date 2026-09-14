/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-SETTINGS */

import { component$, useStyles$ } from "@qwik.dev/core";
import { ThemeSwitch } from "../theme-switch/index.tsx";
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
 * The theme row is the `ThemeSwitch` component and not three buttons
 * written out here, because the landing's header carries the same
 * control and a reader must not meet two switches that disagree about
 * how many states a theme has. It has three: the absence of the
 * attribute is one of them.
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
          data-quick-toc
          aria-label="Contents"
        >
          ≡
        </button>
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
          <ThemeSwitch
            label="Theme"
            lightLabel="light"
            darkLabel="dark"
            systemLabel="system"
            compact={false}
          />
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
