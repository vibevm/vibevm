/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-SETTINGS */

/**
 * What a reader may change about the reading, and where it is kept.
 *
 * Five settings and one rule about each: the theme, the text size, the
 * column width, whether the block numbers are shown, and — where the
 * documentation declared a learning path — which of the two views of the
 * contents the column opens on (`##NAV-CHAPTERS-READER`). They are kept
 * in `localStorage` on the public site and handed to the host through
 * `postMessage` in an embedded reader, and this module does not know
 * which of the two it is in — it publishes and it is told, and the
 * bridge decides where that goes. A module that reached for storage
 * directly would be a module that cannot run inside an editor.
 *
 * Two of the five are applied before this module exists, so what they
 * ARE lives beside them rather than here. The theme is the one setting a
 * page may carry without a reader at all — the landing offers it in the
 * header and offers nothing else — so it lives in `theme.ts`; the
 * contents view is written into the document by the same script that
 * writes the theme, ahead of the first stylesheet, so it lives in
 * `contents-view.ts`. Everything below about either of them is a
 * delegation, and their states, keys and stamping are stated once, there.
 *
 * The panel offers three of the five. The theme is in the header of every
 * page of the site and the contents view is a switch over the list it
 * switches, which is where a reader looks for it; both are still settings
 * of the reading, kept under one prefix, reset by one button, and carried
 * over one bridge to a host that keeps them.
 */

import { SITE } from "../config.ts";
import {
  applyContentsView,
  isContentsView,
  storedContentsView,
  DEFAULT_CONTENTS_VIEW,
  type ContentsView,
} from "./contents-view.ts";
import { all } from "./dom.ts";
import { readLocal, removeLocal, writeLocal } from "./storage.ts";
import { applyTheme, isTheme, storedTheme, type Theme } from "./theme.ts";

/** The steps the text size moves in, as percentages of the page's own. */
const FONT_STEPS = [80, 90, 100, 110, 120, 135, 150] as const;

/** The column widths, in pixels. Desktop only; a phone has one. */
const WIDTH_STEPS = [740, 900, 1100, 1400] as const;

export type { ContentsView, Theme };

export type ReaderSettings = {
  readonly theme: Theme;
  /** The text size as a percentage; one of `FONT_STEPS`. */
  readonly font: number;
  /** The reading measure in pixels; one of `WIDTH_STEPS`. */
  readonly width: number;
  /** Whether the pipeline's block numbers are shown. */
  readonly anchors: boolean;
  /** Which view of the contents the column opens on. */
  readonly contents: ContentsView;
};

/**
 * What a reader who has chosen nothing gets.
 *
 * The theme is the deployment's rather than a constant here (F-48): the
 * same value `theme-init.js` was built with, so the panel's marked
 * button and the page painted before it agree. The contents view is the
 * one the norm names — the declared path — and it is stated where the
 * view is stated, for the same reason. The other three are the reading
 * defaults and belong to the reader rather than to a domain.
 */
export const DEFAULT_SETTINGS: ReaderSettings = {
  theme: SITE.defaultTheme,
  font: 100,
  width: 740,
  anchors: true,
  contents: DEFAULT_CONTENTS_VIEW,
};

/** Where the settings live, and who else is told when they change. */
export type SettingsBridge = {
  /** `false` inside a host that keeps them for us. */
  readonly persist: boolean;
  /** Called after every change the reader makes. */
  readonly publish: (settings: ReaderSettings) => void;
};

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
  const font = Number.parseInt(readLocal("font") ?? "", 10);
  const width = Number.parseInt(readLocal("width") ?? "", 10);
  const anchors = readLocal("anchors");
  return {
    theme: storedTheme(),
    font: isStep(FONT_STEPS, font) ? font : DEFAULT_SETTINGS.font,
    width: isStep(WIDTH_STEPS, width) ? width : DEFAULT_SETTINGS.width,
    anchors: anchors === null ? DEFAULT_SETTINGS.anchors : anchors !== "off",
    contents: storedContentsView(),
  };
}

function keep(settings: ReaderSettings): void {
  writeLocal("theme", settings.theme);
  writeLocal("font", String(settings.font));
  writeLocal("width", String(settings.width));
  writeLocal("anchors", settings.anchors ? "on" : "off");
  writeLocal("contents", settings.contents);
}

function forget(): void {
  for (const key of ["theme", "font", "width", "anchors", "contents"]) {
    removeLocal(key);
  }
}

/** Put the settings on the document. Everything visual happens here. */
function apply(settings: ReaderSettings): void {
  applyTheme(settings.theme);
  applyContentsView(settings.contents);

  for (const column of all(".prose")) {
    column.style.setProperty("--reader-font", String(settings.font / 100));
    column.style.setProperty("--measure", `${settings.width}px`);
    column.classList.toggle("no-anchors", !settings.anchors);
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

    /* The switch over the contents. It is the one setting whose control
       is not in the panel, and it is handled here all the same: the
       choice is kept, published and reset with the other four, and a
       second listener on the document for one attribute would be a
       second place that decides what a reading setting is. */
    const view = target.closest("[data-contents-choice]");
    if (view instanceof HTMLElement) {
      const choice = view.dataset["contentsChoice"];
      if (isContentsView(choice)) change({ ...settings, contents: choice });
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
        contents: isContentsView(patch.contents)
          ? patch.contents
          : settings.contents,
      };
      settings = next;
      apply(settings);
      if (bridge.persist) keep(settings);
    },
    current: () => settings,
  };
}
