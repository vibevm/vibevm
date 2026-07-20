// spec://vibeterm/PROP-047#contract-union, #codec, #versioning, #exchange-model
// The chrome<->engine transport contract: a versioned discriminated union carrying NO Electron types.
// Electron IPC via a typed preload bridge is one transport adapter; a future sidecar is another.

import type { PaneId, TabId, WindowId } from "./modelview";

export const PROTOCOL_VERSION = "0.1.0";

// A target window for tear-off: a known id, or the literal "new" (create a fresh window).
export type WindowTarget = WindowId | "new";

export type Command =
  | { readonly t: "open" }
  | { readonly t: "select"; readonly tabId: TabId }
  | { readonly t: "close"; readonly tabId: TabId }
  | { readonly t: "pane.split"; readonly tabId: TabId; readonly dir?: "right" | "down" }
  | { readonly t: "pane.close"; readonly paneId: PaneId }
  | { readonly t: "tab.move-to-window"; readonly tabId: TabId; readonly windowId: WindowTarget }
  | { readonly t: "set-compact"; readonly on: boolean }
  | { readonly t: "set-theme"; readonly theme: string }
  | { readonly t: "set-locale"; readonly locale: string };

export type Event =
  | { readonly t: "ready" }
  | { readonly t: "opened"; readonly tabId: TabId; readonly title: string }
  | { readonly t: "closed"; readonly tabId: TabId }
  | { readonly t: "active-changed"; readonly tabId: TabId }
  | { readonly t: "modelview"; readonly view: unknown };

export interface Frame {
  readonly v: string;
  readonly kind: "command" | "event";
  readonly payload: Command | Event;
}

export class ProtocolVersionError extends Error {
  constructor(readonly got: string, readonly want: string) {
    super(`protocol version mismatch: got ${got}, want ${want}`);
    this.name = "ProtocolVersionError";
  }
}

export function encode(frame: Frame): string {
  return JSON.stringify(frame);
}

export function decode(s: string): Frame {
  const f = JSON.parse(s) as Frame;
  if (typeof f.v !== "string" || (f.kind !== "command" && f.kind !== "event")) {
    throw new Error(`invalid frame: ${s}`);
  }
  return f;
}

export function checkVersion(frame: Frame): void {
  if (frame.v !== PROTOCOL_VERSION) throw new ProtocolVersionError(frame.v, PROTOCOL_VERSION);
}

export function commandFrame(payload: Command): Frame {
  return { v: PROTOCOL_VERSION, kind: "command", payload };
}

export function eventFrame(payload: Event): Frame {
  return { v: PROTOCOL_VERSION, kind: "event", payload };
}
