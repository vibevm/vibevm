// spec://vibeterm/PROP-047#mvc, #single-writer
// Pure tab-registry cell: applies a Command to a ModelView, returns the next ModelView + event deltas.
// No pty, no Electron — those side-effects live in main.cjs. This is the render-free logic the tests
// exercise and the AIUI reads.

import type { ModelView, ModelViewChange, ShellWindow, Tab, TabEvent, TabId } from "./modelview";
import { tabId, windowId } from "./modelview";
import type { Command } from "./protocol";

let counter = 0;

export function resetTabCounter(): void {
  counter = 0;
}

export function freshTabId(): TabId {
  counter += 1;
  return tabId(`t${counter}`);
}

export function emptyModelView(windowIdValue: string): ModelView {
  return {
    windows: [{ id: windowId(windowIdValue), tabs: [] }],
    tabs: new Map(),
    panes: new Map(),
    compact: false,
    activeWindow: windowId(windowIdValue),
    activeTab: null,
    activePane: null,
    enabledActions: [],
  };
}

function setActive(tabs: Map<TabId, Tab>, activeId: TabId): void {
  for (const [k, t] of tabs) tabs.set(k, { ...t, active: k === activeId });
}

// Apply a command purely; main.cjs owns the pty/view side-effects that the events imply.
export function apply(view: ModelView, cmd: Command): ModelViewChange {
  switch (cmd.t) {
    case "open": {
      const id = freshTabId();
      const newTab: Tab = { id, title: `Terminal ${counter}`, kind: "terminal", active: true };
      const tabs = new Map(view.tabs);
      tabs.set(id, newTab);
      setActive(tabs, id);
      const win = view.windows[0];
      if (!win) throw new Error("no window in ModelView");
      const windows: ShellWindow[] = [{ ...win, tabs: [...win.tabs, id] }];
      const next: ModelView = { ...view, windows, tabs, activeTab: id };
      const events: TabEvent[] = [
        { type: "opened", tab: newTab },
        { type: "active-changed", tabId: id },
      ];
      return { view: next, events };
    }
    case "select": {
      if (!view.tabs.has(cmd.tabId)) throw new Error(`unknown tab: ${cmd.tabId}`);
      const tabs = new Map(view.tabs);
      setActive(tabs, cmd.tabId);
      const next: ModelView = { ...view, tabs, activeTab: cmd.tabId };
      return { view: next, events: [{ type: "active-changed", tabId: cmd.tabId }] };
    }
    case "close": {
      const win = view.windows[0];
      if (!win) throw new Error("no window in ModelView");
      const tabs = new Map(view.tabs);
      tabs.delete(cmd.tabId);
      const remaining = win.tabs.filter((t) => t !== cmd.tabId);
      const windows: ShellWindow[] = [{ ...win, tabs: remaining }];
      const events: TabEvent[] = [{ type: "closed", tabId: cmd.tabId }];
      let activeTab = view.activeTab;
      if (activeTab === cmd.tabId) {
        activeTab = remaining.length > 0 ? (remaining[remaining.length - 1] as TabId) : null;
        if (activeTab) {
          setActive(tabs, activeTab);
          events.push({ type: "active-changed", tabId: activeTab });
        }
      }
      const next: ModelView = { ...view, windows, tabs, activeTab };
      return { view: next, events };
    }
    case "set-compact":
      return { view: { ...view, compact: cmd.on }, events: [] };
    case "set-theme":
    case "set-locale":
      // Theme/locale are chrome-side projection state in the pre-MVP; the engine no-ops here.
      return { view, events: [] };
  }
}
