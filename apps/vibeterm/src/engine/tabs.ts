// spec://vibeterm/PROP-047#mvc, #single-writer, spec://vibeterm/PROP-044 §3-§5
// Pure tab/pane/window cell: applies a Command to a ModelView, returns the next ModelView + event
// deltas. No pty, no Electron -- those side-effects live in main.cjs. This is the render-free logic
// the tests exercise and the AIUI reads. Split ceiling = 2 panes per window (PROP-044 D3).

import type {
  ModelView,
  ModelViewChange,
  Pane,
  PaneId,
  ShellWindow,
  Tab,
  TabEvent,
  TabId,
  WindowId,
} from "./modelview";
import { paneId, tabId, windowId } from "./modelview";
import type { Command } from "./protocol";

let tabCounter = 0;
let paneCounter = 0;

export function resetTabCounter(): void {
  tabCounter = 0;
  paneCounter = 0;
}

export function freshTabId(): TabId {
  tabCounter += 1;
  return tabId(`t${tabCounter}`);
}

function freshPaneId(): PaneId {
  paneCounter += 1;
  return paneId(`p${paneCounter}`);
}

export function emptyModelView(windowIdValue: string): ModelView {
  const id = windowId(windowIdValue);
  return {
    windows: [{ id, tabs: [], panes: [] }],
    tabs: new Map(),
    panes: new Map(),
    compact: false,
    activeWindow: id,
    activeTab: null,
    activePane: null,
    enabledActions: [],
  };
}

function setActive(tabs: Map<TabId, Tab>, activeId: TabId): void {
  for (const [k, t] of tabs) tabs.set(k, { ...t, active: k === activeId });
}

// The window a tab currently lives in.
function windowOf(view: ModelView, tab: TabId): ShellWindow | undefined {
  return view.windows.find((w) => w.tabs.includes(tab));
}

function activeWin(view: ModelView): ShellWindow {
  const w = view.windows.find((x) => x.id === view.activeWindow) ?? view.windows[0];
  if (!w) throw new Error("no window in ModelView");
  return w;
}

// Collapse a single surviving pane back to slot "full" (used after a split half is closed).
function collapseToFull(panes: Map<PaneId, Pane>, paneId: PaneId): void {
  const only = panes.get(paneId);
  if (only && only.slot !== "full") panes.set(paneId, { ...only, slot: "full" });
}

export function apply(view: ModelView, cmd: Command): ModelViewChange {
  switch (cmd.t) {
    case "open": {
      const id = freshTabId();
      const newTab: Tab = { id, title: `Terminal ${tabCounter}`, kind: "terminal", active: true };
      const tabs = new Map(view.tabs);
      tabs.set(id, newTab);
      setActive(tabs, id);
      const win = activeWin(view);
      // The new tab becomes the visible one: replace the window's panes with a single full pane
      // showing it. (A split is added separately by `pane.split`.)
      const pane: Pane = { id: freshPaneId(), tabId: id, windowId: win.id, slot: "full" };
      const panes = new Map(view.panes);
      for (const pid of win.panes) panes.delete(pid);
      panes.set(pane.id, pane);
      const windows = view.windows.map((w) =>
        w.id === win.id ? { ...w, tabs: [...w.tabs, id], panes: [pane.id] } : w,
      );
      const next: ModelView = { ...view, windows, tabs, panes, activeTab: id, activePane: pane.id };
      return { view: next, events: [{ type: "opened", tab: newTab }, { type: "active-changed", tabId: id }] };
    }

    case "select": {
      if (!view.tabs.has(cmd.tabId)) throw new Error(`unknown tab: ${cmd.tabId}`);
      const tabs = new Map(view.tabs);
      setActive(tabs, cmd.tabId);
      const win = activeWin(view);
      const panes = new Map(view.panes);
      // Make the selected tab the one the active pane shows.
      let activePane = view.activePane;
      const ap = activePane ? panes.get(activePane) : undefined;
      if (ap) {
        panes.set(ap.id, { ...ap, tabId: cmd.tabId });
      } else if (win.panes.length > 0) {
        const firstId = win.panes[0] as PaneId;
        const first = panes.get(firstId);
        if (first) {
          panes.set(firstId, { ...first, tabId: cmd.tabId });
          activePane = firstId;
        }
      }
      const next: ModelView = { ...view, tabs, panes, activeTab: cmd.tabId, activePane: activePane ?? null };
      return { view: next, events: [{ type: "active-changed", tabId: cmd.tabId }] };
    }

    case "close": {
      const win = windowOf(view, cmd.tabId);
      if (!win) return { view, events: [] };
      const tabs = new Map(view.tabs);
      tabs.delete(cmd.tabId);
      const panes = new Map(view.panes);
      const survivingPanes = win.panes.filter((pid) => {
        const p = panes.get(pid);
        if (p && p.tabId === cmd.tabId) {
          panes.delete(pid);
          return false;
        }
        return true;
      });
      if (survivingPanes.length === 1) collapseToFull(panes, survivingPanes[0] as PaneId);
      const remainingTabs = win.tabs.filter((t) => t !== cmd.tabId);
      const events: TabEvent[] = [{ type: "closed", tabId: cmd.tabId }];
      let windows: ShellWindow[];
      if (remainingTabs.length === 0 && view.windows.length > 1) {
        windows = view.windows.filter((w) => w.id !== win.id);
      } else {
        windows = view.windows.map((w) =>
          w.id === win.id ? { ...w, tabs: remainingTabs, panes: survivingPanes } : w,
        );
      }
      let activeTab = view.activeTab;
      let activePane = view.activePane;
      if (activeTab === cmd.tabId) {
        activeTab = remainingTabs.length > 0 ? (remainingTabs[remainingTabs.length - 1] as TabId) : null;
        if (activeTab) {
          setActive(tabs, activeTab);
          events.push({ type: "active-changed", tabId: activeTab });
        }
      }
      if (activePane && !panes.has(activePane)) {
        activePane = survivingPanes.length > 0 ? (survivingPanes[survivingPanes.length - 1] as PaneId) : null;
      }
      return { view: { ...view, windows, tabs, panes, activeTab, activePane }, events };
    }

    case "pane.split": {
      if (!view.tabs.has(cmd.tabId)) throw new Error(`unknown tab: ${cmd.tabId}`);
      const win = activeWin(view);
      if (win.panes.length !== 1) return { view, events: [] }; // split ceiling 2 (D3)
      const panes = new Map(view.panes);
      const firstId = win.panes[0] as PaneId;
      const first = panes.get(firstId);
      if (!first) throw new Error("dangling pane");
      panes.set(firstId, { ...first, slot: "left" });
      const second: Pane = { id: freshPaneId(), tabId: cmd.tabId, windowId: win.id, slot: "right" };
      panes.set(second.id, second);
      const windows = view.windows.map((w) =>
        w.id === win.id ? { ...w, panes: [firstId, second.id] } : w,
      );
      return {
        view: { ...view, windows, panes, activePane: second.id },
        events: [{ type: "pane-split", paneId: second.id, tabId: cmd.tabId }],
      };
    }

    case "pane.close": {
      const win = activeWin(view);
      if (win.panes.length < 2) return { view, events: [] };
      const panes = new Map(view.panes);
      panes.delete(cmd.paneId);
      const surviving = win.panes.filter((p) => p !== cmd.paneId);
      if (surviving.length === 1) collapseToFull(panes, surviving[0] as PaneId);
      const windows = view.windows.map((w) => (w.id === win.id ? { ...w, panes: surviving } : w));
      const activePane = surviving.length > 0 ? (surviving[surviving.length - 1] as PaneId) : null;
      const activeTab = activePane ? (panes.get(activePane)?.tabId ?? view.activeTab) : view.activeTab;
      return { view: { ...view, windows, panes, activePane, activeTab }, events: [{ type: "pane-closed", paneId: cmd.paneId }] };
    }

    case "tab.move-to-window": {
      const src = windowOf(view, cmd.tabId);
      if (!src) return { view, events: [] };
      let targetId: WindowId;
      let windows: readonly ShellWindow[];
      if (cmd.windowId === "new") {
        targetId = windowId(`w${view.windows.length + 1}`);
        windows = [...view.windows, { id: targetId, tabs: [], panes: [] }];
      } else {
        targetId = cmd.windowId;
        windows = view.windows;
      }
      const target = windows.find((w) => w.id === targetId);
      if (!target) throw new Error(`unknown window: ${targetId}`);
      const tabs = new Map(view.tabs);
      const panes = new Map(view.panes);
      // detach the tab (and any pane showing it) from the source window
      const droppedSrcPanes = src.panes.filter((pid) => panes.get(pid)?.tabId === cmd.tabId);
      for (const pid of droppedSrcPanes) panes.delete(pid);
      const srcTabs = src.tabs.filter((t) => t !== cmd.tabId);
      const srcPanes = src.panes.filter((p) => !droppedSrcPanes.includes(p));
      if (srcPanes.length === 1) collapseToFull(panes, srcPanes[0] as PaneId);
      // attach to the target: a fresh full pane if the target was empty, else just the tab
      let targetPanes = target.panes;
      if (target.panes.length === 0) {
        const pane: Pane = { id: freshPaneId(), tabId: cmd.tabId, windowId: targetId, slot: "full" };
        panes.set(pane.id, pane);
        targetPanes = [pane.id];
      }
      const targetTabs = [...target.tabs, cmd.tabId];
      let rebuilt: ShellWindow[];
      if (srcTabs.length === 0 && windows.length > 1) {
        // the source window is now empty -> drop it
        rebuilt = windows
          .filter((w) => w.id !== src.id)
          .map((w) => (w.id === targetId ? { ...w, tabs: targetTabs, panes: targetPanes } : w));
      } else {
        rebuilt = windows.map((w) => {
          if (w.id === src.id) return { ...w, tabs: srcTabs, panes: srcPanes };
          if (w.id === targetId) return { ...w, tabs: targetTabs, panes: targetPanes };
          return w;
        });
      }
      const activePane = targetPanes.length > 0 ? (targetPanes[targetPanes.length - 1] as PaneId) : null;
      const activeTab = activePane ? (panes.get(activePane)?.tabId ?? cmd.tabId) : cmd.tabId;
      setActive(tabs, activeTab);
      return {
        view: {
          ...view,
          windows: rebuilt,
          tabs,
          panes,
          activeWindow: targetId,
          activeTab,
          activePane,
        },
        events: [{ type: "moved", tabId: cmd.tabId, fromWindow: src.id, toWindow: targetId }],
      };
    }

    case "set-compact":
      return { view: { ...view, compact: cmd.on }, events: [] };
    case "set-theme":
    case "set-locale":
      // Theme/locale are chrome-side projection state; the engine no-ops here.
      return { view, events: [] };
  }
}
