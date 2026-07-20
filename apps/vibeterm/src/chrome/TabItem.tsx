// spec://vibeterm/design-system (contacts-style list row) + spec://vibeterm/PROP-044 §4/§5
// One tab as a list row with a right-click context menu (Kobalte): Open in split view /
// Open in new window. Click selects; hover shows the close affordance.

import type { JSX } from "solid-js";
import { ContextMenu } from "@kobalte/core/context-menu";

export interface TabSnapshot {
  readonly id: string;
  readonly title: string;
  readonly active: boolean;
}

export function TabItem(props: {
  tab: TabSnapshot;
  onSelect: (id: string) => void;
  onClose: (id: string) => void;
  onSplit: (id: string) => void;
  onTearOff: (id: string) => void;
}): JSX.Element {
  return (
    <ContextMenu>
      <ContextMenu.Trigger
        class="tab-item"
        classList={{ active: props.tab.active }}
        onClick={() => props.onSelect(props.tab.id)}
      >
        <span class="tab-glyph" aria-hidden="true">
          ▸
        </span>
        <span class="tab-title">{props.tab.title}</span>
        <button
          class="tab-close"
          title="close"
          tabIndex={-1}
          onClick={(e) => {
            e.stopPropagation();
            props.onClose(props.tab.id);
          }}
        >
          ×
        </button>
      </ContextMenu.Trigger>
      <ContextMenu.Portal>
        <ContextMenu.Content class="ctx-menu">
          <ContextMenu.Item class="ctx-item" onSelect={() => props.onSplit(props.tab.id)}>
            Open in split view
          </ContextMenu.Item>
          <ContextMenu.Item class="ctx-item" onSelect={() => props.onTearOff(props.tab.id)}>
            Open in new window
          </ContextMenu.Item>
        </ContextMenu.Content>
      </ContextMenu.Portal>
    </ContextMenu>
  );
}
