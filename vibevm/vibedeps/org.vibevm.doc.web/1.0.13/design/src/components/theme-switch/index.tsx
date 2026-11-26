/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-SETTINGS */

import { component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";

export type ThemeSwitchProps = {
  /** What the group of three is, for a reader who cannot see them. */
  readonly label: string;
  /** What each choice is called; the compact form shows no word. */
  readonly lightLabel: string;
  readonly darkLabel: string;
  readonly systemLabel: string;
  /**
   * The header's form — three marks in a pill — rather than the reading
   * panel's row of words. The words are what a panel with a heading over
   * it can afford; a header cannot, and a mark with a name a screen
   * reader can say is the honest way to spend that room.
   */
  readonly compact: boolean;
};

/**
 * The three states of the theme, as one control.
 *
 * It is a component and not markup in two places because a reader may
 * meet it twice — in the landing's header and in a documentation page's
 * reading panel — and a control that offered two states in one place and
 * three in the other would be two controls wearing one name.
 *
 * Three buttons and never a toggle. `dark` and `light` are the reader's
 * explicit choice and are stamped on the root element; `system` stamps
 * nothing, because the ABSENCE of the attribute is what hands the
 * decision back to the operating system. A two-way switch would quietly
 * remove that third answer and leave no way back to it.
 *
 * It carries no behaviour, like everything else in this system: the
 * buttons say what they are with `data-theme-choice`, and `reader/theme.ts`
 * is what stamps the document, remembers the choice and marks whichever
 * button is current. Which is also why a page may carry two of these and
 * both stay right.
 */
export const ThemeSwitch = component$<ThemeSwitchProps>((props) => {
  useStyles$(styles);
  const choices = [
    { value: "light", label: props.lightLabel },
    { value: "dark", label: props.darkLabel },
    { value: "system", label: props.systemLabel },
  ] as const;

  return (
    <div
      class={
        props.compact ? "theme-switch theme-switch--marks" : "theme-switch"
      }
      role="group"
      aria-label={props.label}
      data-theme-switch
    >
      {choices.map((choice) => (
        <button
          key={choice.value}
          class="theme-switch__button"
          type="button"
          data-theme-choice={choice.value}
          aria-label={choice.label}
          aria-pressed="false"
        >
          {props.compact ? (
            <Mark of={choice.value} />
          ) : (
            <span>{choice.label}</span>
          )}
        </button>
      ))}
    </div>
  );
});

/**
 * The three marks, drawn rather than fetched and drawn rather than
 * typed: an emoji is a font's opinion about a sun, and a picture from
 * anyone else's host is a reader whose theme choice left the domain
 * (R-09, D-20).
 */
const Mark = component$<{ of: "light" | "dark" | "system" }>((props) => {
  if (props.of === "light") {
    return (
      <svg
        width="14"
        height="14"
        viewBox="0 0 14 14"
        fill="none"
        aria-hidden="true"
      >
        <circle
          cx="7"
          cy="7"
          r="2.9"
          stroke="currentColor"
          stroke-width="1.3"
        />
        <path
          d="M7 .9v1.6M7 11.5v1.6M1.3 7h1.6M11.1 7h1.6M2.97 2.97l1.13 1.13M9.9 9.9l1.13 1.13M11.03 2.97 9.9 4.1M4.1 9.9l-1.13 1.13"
          stroke="currentColor"
          stroke-width="1.3"
          stroke-linecap="round"
        />
      </svg>
    );
  }
  if (props.of === "dark") {
    return (
      <svg
        width="14"
        height="14"
        viewBox="0 0 14 14"
        fill="none"
        aria-hidden="true"
      >
        <path
          d="M11.6 8.6A5.1 5.1 0 0 1 5.4 2.4a5.1 5.1 0 1 0 6.2 6.2Z"
          stroke="currentColor"
          stroke-width="1.3"
          stroke-linejoin="round"
        />
      </svg>
    );
  }
  return (
    <svg
      width="14"
      height="14"
      viewBox="0 0 14 14"
      fill="none"
      aria-hidden="true"
    >
      <rect
        x="1.4"
        y="2.4"
        width="11.2"
        height="7.6"
        rx="1.4"
        stroke="currentColor"
        stroke-width="1.3"
      />
      <path
        d="M4.8 12.4h4.4"
        stroke="currentColor"
        stroke-width="1.3"
        stroke-linecap="round"
      />
    </svg>
  );
});
