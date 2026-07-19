// spec://vibeterm/PROP-047#tree, #deltas, #per-tab-scope
// The serialisable ModelView: a window -> tab -> pane tree, no rendering types.
// Change is delivered by re-resolution + event-deltas; there is no in-place mutation.

export type TabId = string & { readonly __brand: "TabId" };
export type WindowId = string & { readonly __brand: "WindowId" };
export type PaneId = string & { readonly __brand: "PaneId" };

export function tabId(s: string): TabId {
  return s as TabId;
}
export function windowId(s: string): WindowId {
  return s as WindowId;
}
export function paneId(s: string): PaneId {
  return s as PaneId;
}

export interface Pane {
  readonly id: PaneId;
  readonly tabId: TabId;
  readonly bounds: { readonly x: number; readonly y: number; readonly w: number; readonly h: number };
}

export interface Tab {
  readonly id: TabId;
  readonly title: string;
  readonly kind: "terminal";
  readonly active: boolean;
}

export interface ShellWindow {
  readonly id: WindowId;
  readonly tabs: readonly TabId[];
}

export interface EnabledAction {
  readonly addr: string;
  readonly enabled: boolean;
  readonly reason: string | null;
}

export interface ModelView {
  readonly windows: readonly ShellWindow[];
  readonly tabs: ReadonlyMap<TabId, Tab>;
  readonly panes: ReadonlyMap<PaneId, Pane>;
  readonly compact: boolean;
  readonly activeWindow: WindowId;
  readonly activeTab: TabId | null;
  readonly activePane: PaneId | null;
  readonly enabledActions: readonly EnabledAction[];
}

// spec://vibeterm/PROP-047#deltas — events are the ModelView's deltas.
export type TabEvent =
  | { readonly type: "opened"; readonly tab: Tab }
  | { readonly type: "closed"; readonly tabId: TabId }
  | { readonly type: "active-changed"; readonly tabId: TabId }
  | {
      readonly type: "moved";
      readonly tabId: TabId;
      readonly fromWindow: WindowId;
      readonly toWindow: WindowId;
    };

export interface ModelViewChange {
  readonly view: ModelView;
  readonly events: readonly TabEvent[];
}
