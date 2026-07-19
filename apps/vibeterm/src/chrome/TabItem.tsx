// spec://vibeterm/design-system (contacts-style list row)
// One tab as a list item: a glyph, a title, an active indicator, a hover close affordance.

import type { JSX } from "solid-js";

export interface TabSnapshot {
  readonly id: string;
  readonly title: string;
  readonly active: boolean;
}

export function TabItem(props: {
  tab: TabSnapshot;
  onSelect: (id: string) => void;
  onClose: (id: string) => void;
}): JSX.Element {
  return (
    <div
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
    </div>
  );
}
