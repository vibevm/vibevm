// spec://vibeterm/PROP-047#mvc, #projection
// The chrome is a one-way projection of the engine's ModelView. The bridge subscribes to engine
// events and surfaces a Solid signal; commands go out through the typed preload bridge.

import { createSignal } from "solid-js";
import type { ModelView } from "@vibeterm/engine";

const [view, setView] = createSignal<ModelView | null>(null);
const [ready, setReady] = createSignal(false);

// The terminal-view native surfaces are laid out by main OVER the content region; the chrome
// does not own them, only the rail. The ModelView tells the rail which tabs exist and which is active.
window.vibeterm.onEvent((ev) => {
  if (ev.t === "ready") setReady(true);
  if (ev.t === "modelview") setView(ev.view as ModelView);
});

// Initial snapshot (the AIUI `state()` peer-read).
void window.vibeterm
  .state()
  .then((v) => setView(v as ModelView))
  .catch(() => {
    /* engine not ready yet — events will follow */
  });

export { view, ready };

export type Command =
  | { readonly t: "open" }
  | { readonly t: "select"; readonly tabId: string }
  | { readonly t: "close"; readonly tabId: string }
  | { readonly t: "pane.split"; readonly tabId: string; readonly dir?: "right" | "down" }
  | { readonly t: "pane.close"; readonly paneId: string }
  | { readonly t: "tab.move-to-window"; readonly tabId: string; readonly windowId: string | "new" }
  | { readonly t: "set-compact"; readonly on: boolean }
  | { readonly t: "set-theme"; readonly theme: string }
  | { readonly t: "set-locale"; readonly locale: string };

export function sendCommand(cmd: Command): Promise<{ ok: boolean; error?: string }> {
  return window.vibeterm.command(cmd as { t: string });
}
