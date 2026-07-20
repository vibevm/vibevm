// spec://vibeterm/design-system#spacing-scale, spec://vibeterm/PROP-047#projection
// The contacts-style terminal list: a vertical rail of TabItem rows over the projected ModelView.

import { For, Show, type JSX } from "solid-js";
import { TabItem } from "./TabItem";
import { view, sendCommand } from "./bridge";
import { t } from "./i18n";

export function TabList(): JSX.Element {
  const tabs = () => {
    const v = view();
    return v ? [...v.tabs.values()] : [];
  };

  return (
    <div class="tab-list">
      <div class="rail-header">
        <span class="rail-title">{t("tab.list")}</span>
        <button class="rail-new" title={t("tab.new")} onClick={() => sendCommand({ t: "open" })}>
          +
        </button>
      </div>
      <div class="tab-items">
        <For each={tabs()}>
          {(tab) => (
            <TabItem
              tab={{ id: tab.id, title: tab.title, active: tab.active }}
              onSelect={(id) => sendCommand({ t: "select", tabId: id })}
              onClose={(id) => sendCommand({ t: "close", tabId: id })}
              onSplit={(id) => sendCommand({ t: "pane.split", tabId: id, dir: "right" })}
              onTearOff={(id) =>
                sendCommand({ t: "tab.move-to-window", tabId: id, windowId: "new" })
              }
            />
          )}
        </For>
        <Show when={tabs().length === 0}>
          <div class="tab-empty">{t("tab.empty")}</div>
        </Show>
      </div>
    </div>
  );
}
