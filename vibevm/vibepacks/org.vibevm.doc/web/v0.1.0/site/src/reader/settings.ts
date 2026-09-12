/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-SETTINGS */

/**
 * What a reader may change about the reading, and where it is kept.
 *
 * Four settings and one rule about each: the theme, the text size, the
 * column width, and whether the block numbers are shown. They are kept
 * in `localStorage` on the public site and handed to the host through
 * `postMessage` in an embedded reader, and this module does not know
 * which of the two it is in — it publishes and it is told, and the
 * bridge decides where that goes. A module that reached for storage
 * directly would be a module that cannot run inside an editor.
 *
 * The theme has THREE states and the third one is the default. `dark`
 * and `light` are stamped on the root element; `system` stamps nothing,
 * because the absence of the attribute is what hands the decision to
 * `prefers-color-scheme`. A two-state toggle would silently take the
 * default away, and the reader would never get it back.
 */

import { all } from "./dom.ts";
import { readLocal, removeLocal, writeLocal } from "./storage.ts";

/** The steps the text size moves in, as percentages of the page's own. */
const FONT_STEPS = [80, 90, 100, 110, 120, 135, 150] as const;

/** The column widths, in pixels. Desktop only; a phone has one. */
const WIDTH_STEPS = [740, 900, 1100, 1400] as const;

export type Theme = "dark" | "light" | "system";

export type ReaderSettings = {
  readonly theme: Theme;
  /** The text size as a percentage; one of `FONT_STEPS`. */
  readonly font: number;
  /** The reading measure in pixels; one of `WIDTH_STEPS`. */
  readonly width: number;
  /** Whether the pipeline's block numbers are shown. */
  readonly anchors: boolean;
};

export const DEFAULT_SETTINGS: ReaderSettings = {
  theme: "system",
  font: 100,
  width: 740,
  anchors: true,
};

/** Where the settings live, and who else is told when they change. */
export type SettingsBridge = {
  /** `false` inside a host that keeps them for us. */
  readonly persist: boolean;
  /** Called after every change the reader makes. */
  readonly publish: (settings: ReaderSettings) => void;
};

function isTheme(value: unknown): value is Theme {
  return value === "dark" || value === "light" || value === "system";
}

function nearest(steps: readonly number[], value: number): number {
  let best = steps[0] ?? value;
  for (const step of steps) {
    if (Math.abs(step - value) < Math.abs(best - value)) best = step;
  }
  return best;
}

/**
 * Whether a number is one of the steps — asked, never asserted. A stored
 * value comes from a previous version of this page as often as from this
 * one, so «is it one of ours» is a question with a real answer.
 */
function isStep(steps: readonly number[], value: number): boolean {
  return steps.some((step) => step === value);
}

function stored(): ReaderSettings {
  const theme = readLocal("theme");
  const font = Number.parseInt(readLocal("font") ?? "", 10);
  const width = Number.parseInt(readLocal("width") ?? "", 10);
  const anchors = readLocal("anchors");
  return {
    theme: isTheme(theme) ? theme : DEFAULT_SETTINGS.theme,
    font: isStep(FONT_STEPS, font) ? font : DEFAULT_SETTINGS.font,
    width: isStep(WIDTH_STEPS, width) ? width : DEFAULT_SETTINGS.width,
    anchors: anchors === null ? DEFAULT_SETTINGS.anchors : anchors !== "off",
  };
}

function keep(settings: ReaderSettings): void {
  writeLocal("theme", settings.theme);
  writeLocal("font", String(settings.font));
  writeLocal("width", String(settings.width));
  writeLocal("anchors", settings.anchors ? "on" : "off");
}

function forget(): void {
  for (const key of ["theme", "font", "width", "anchors"]) removeLocal(key);
}

/** Put the settings on the document. Everything visual happens here. */
function apply(settings: ReaderSettings): void {
  const root = document.documentElement;
  if (settings.theme === "system") root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", settings.theme);

  for (const column of all(".prose")) {
    column.style.setProperty("--reader-font", String(settings.font / 100));
    column.style.setProperty("--measure", `${settings.width}px`);
    column.classList.toggle("no-anchors", !settings.anchors);
  }

  for (const button of all("[data-theme-choice]")) {
    const choice = button.dataset["themeChoice"];
    button.classList.toggle("is-current", choice === settings.theme);
  }
  for (const button of all("[data-toggle='anchors']")) {
    button.setAttribute("aria-pressed", settings.anchors ? "true" : "false");
    if (button.classList.contains("settings__button")) {
      button.textContent = settings.anchors ? "shown" : "hidden";
    }
  }
  for (const output of all("[data-value='font']")) {
    output.textContent = `${settings.font}%`;
  }
  for (const output of all("[data-value='width']")) {
    output.textContent = String(settings.width);
  }
}

function stepped(
  steps: readonly number[],
  current: number,
  delta: number,
): number {
  const at = steps.indexOf(nearest(steps, current));
  const next = Math.min(Math.max(at + delta, 0), steps.length - 1);
  return steps[next] ?? current;
}

/** The reader's settings, live: started, changed, told about, stopped. */
export type SettingsHandle = {
  readonly stop: () => void;
  /** Apply a change that came from outside — the host, in embedded mode. */
  readonly receive: (patch: Partial<ReaderSettings>) => void;
  readonly current: () => ReaderSettings;
};

export function startSettings(bridge: SettingsBridge): SettingsHandle {
  let settings: ReaderSettings = bridge.persist ? stored() : DEFAULT_SETTINGS;
  apply(settings);

  const change = (next: ReaderSettings): void => {
    settings = next;
    apply(settings);
    if (bridge.persist) keep(settings);
    bridge.publish(settings);
  };

  const panel = document.querySelector("[data-settings]");
  const toggle = document.querySelector("[data-settings-toggle]");
  const box = document.querySelector("[data-settings-panel]");

  const onClick = (event: Event): void => {
    const target = event.target;
    if (!(target instanceof Element)) return;

    const gear = target.closest("[data-settings-toggle]");
    if (gear !== null && box instanceof HTMLElement) {
      const open = box.hidden;
      box.hidden = !open;
      gear.setAttribute("aria-expanded", open ? "true" : "false");
      return;
    }

    const theme = target.closest("[data-theme-choice]");
    if (theme instanceof HTMLElement) {
      const choice = theme.dataset["themeChoice"];
      if (isTheme(choice)) change({ ...settings, theme: choice });
      return;
    }

    const step = target.closest("[data-step]");
    if (step instanceof HTMLElement) {
      const delta = Number.parseInt(step.dataset["delta"] ?? "0", 10);
      if (step.dataset["step"] === "font") {
        change({
          ...settings,
          font: stepped(FONT_STEPS, settings.font, delta),
        });
      } else if (step.dataset["step"] === "width") {
        change({
          ...settings,
          width: stepped(WIDTH_STEPS, settings.width, delta),
        });
      }
      return;
    }

    if (target.closest("[data-toggle='anchors']") !== null) {
      change({ ...settings, anchors: !settings.anchors });
      return;
    }

    if (target.closest("[data-reset]") !== null) {
      settings = DEFAULT_SETTINGS;
      apply(settings);
      if (bridge.persist) forget();
      bridge.publish(settings);
    }
  };

  const onOutside = (event: Event): void => {
    if (!(box instanceof HTMLElement) || box.hidden) return;
    const target = event.target;
    if (!(target instanceof Element)) return;
    if (panel !== null && panel.contains(target)) return;
    box.hidden = true;
    toggle?.setAttribute("aria-expanded", "false");
  };

  document.addEventListener("click", onClick);
  document.addEventListener("click", onOutside);

  return {
    stop: () => {
      document.removeEventListener("click", onClick);
      document.removeEventListener("click", onOutside);
    },
    receive: (patch) => {
      const next: ReaderSettings = {
        theme: isTheme(patch.theme) ? patch.theme : settings.theme,
        font:
          typeof patch.font === "number"
            ? nearest(FONT_STEPS, patch.font)
            : settings.font,
        width:
          typeof patch.width === "number"
            ? nearest(WIDTH_STEPS, patch.width)
            : settings.width,
        anchors:
          typeof patch.anchors === "boolean" ? patch.anchors : settings.anchors,
      };
      settings = next;
      apply(settings);
      if (bridge.persist) keep(settings);
    },
    current: () => settings,
  };
}
