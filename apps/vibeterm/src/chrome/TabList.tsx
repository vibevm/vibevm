// spec://vibeterm/design-system#spacing-scale, spec://vibeterm/PROP-047#projection, PROP-044 §1
// Column 2: a "TERMINALS" header (label + chevron + new-tab) over the vertical list of TabItem
// rows, with a user-profile card pinned to the bottom (avatar + name + status + mic/headphones/
// settings). The list reads the projected ModelView.

import { For, Show, type JSX } from "solid-js";
import { TabItem } from "./TabItem";
import { view, sendCommand } from "./bridge";
import { t } from "./i18n";

export function TabList(): JSX.Element {
  const tabs = () => {
    const v = view();
    return v ? [...v.tabs.values()] : [];
  };
  const compact = () => view()?.compact ?? false;

  return (
    <section class="terminal-list" classList={{ compact: compact() }}>
      <header class="list-header">
        <span class="list-title">
          <span class="list-chevron" aria-hidden="true">
            ▾
          </span>
          {t("tab.list")}
        </span>
        <button class="list-new" title={t("tab.new")} onClick={() => sendCommand({ t: "open" })}>
          +
        </button>
      </header>

      <div class="list-items">
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

      <footer class="list-user">
        <span class="list-user-avatar" />
        <span class="list-user-text">
          <span class="list-user-name">{t("user.name")}</span>
          <span class="list-user-status">{t("user.status")}</span>
        </span>
        <button class="list-user-icon" title="mute" aria-label="mute">
          🎤
        </button>
        <button class="list-user-icon" title="headphones" aria-label="headphones">
          ♪
        </button>
        <button class="list-user-icon" title="settings" aria-label="settings">
          ⚙
        </button>
      </footer>
    </section>
  );
}
