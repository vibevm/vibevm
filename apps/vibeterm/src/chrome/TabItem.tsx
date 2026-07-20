// spec://vibeterm/design-system (contacts-style list row) + spec://vibeterm/PROP-044 §4/§5/§6
// A terminal row: a colored avatar (the title's initial), the title + a subtext, a hover close ×.
// Right-click opens the full context menu (the reference's eight items + two separators).

import type { JSX } from "solid-js";
import { ContextMenu } from "@kobalte/core/context-menu";

export interface TabSnapshot {
  readonly id: string;
  readonly title: string;
  readonly active: boolean;
}

const AVATAR_COLORS = [
  "var(--vibe-accent)",
  "var(--vibe-foam)",
  "var(--vibe-gold)",
  "var(--vibe-rose)",
];

function avatarColor(id: string): string {
  let h = 0;
  for (let i = 0; i < id.length; i++) h = (h * 31 + id.charCodeAt(i)) >>> 0;
  return AVATAR_COLORS[h % AVATAR_COLORS.length] ?? "var(--vibe-accent)";
}

export function TabItem(props: {
  tab: TabSnapshot;
  onSelect: (id: string) => void;
  onClose: (id: string) => void;
  onSplit: (id: string) => void;
  onTearOff: (id: string) => void;
}): JSX.Element {
  const initial = () => props.tab.title.replace(/^Terminal\s*/, "").slice(0, 1) || "#";
  const onMenu = (fn: () => void) => () => fn();
  return (
    <ContextMenu>
      <ContextMenu.Trigger
        class="tab-item"
        classList={{ active: props.tab.active }}
        onClick={() => props.onSelect(props.tab.id)}
      >
        <span class="tab-avatar" style={{ background: avatarColor(props.tab.id) }}>
          {initial()}
        </span>
        <span class="tab-text">
          <span class="tab-title">{props.tab.title}</span>
          <span class="tab-sub">bash · ready</span>
        </span>
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
          <ContextMenu.Item class="ctx-item" onSelect={onMenu(() => props.onSplit(props.tab.id))}>
            <span class="ctx-glyph" aria-hidden="true">
             ⛶
            </span>
            Open in split view
          </ContextMenu.Item>
          <ContextMenu.Item class="ctx-item" onSelect={onMenu(() => props.onTearOff(props.tab.id))}>
            <span class="ctx-glyph" aria-hidden="true">
              ⤢
            </span>
            Open in new window
          </ContextMenu.Item>
          <ContextMenu.Item class="ctx-item" onSelect={() => {}}>
            <span class="ctx-glyph" aria-hidden="true">
              ⇥
            </span>
            Open in background tab
          </ContextMenu.Item>
          <ContextMenu.Separator class="ctx-sep" />
          <ContextMenu.Item class="ctx-item" onSelect={() => {}}>
            <span class="ctx-glyph" aria-hidden="true">
              ✎
            </span>
            Rename
          </ContextMenu.Item>
          <ContextMenu.Item class="ctx-item" onSelect={onMenu(() => props.onClose(props.tab.id))}>
            <span class="ctx-glyph" aria-hidden="true">
              ×
            </span>
            Close
          </ContextMenu.Item>
          <ContextMenu.Item class="ctx-item" onSelect={() => {}}>
            <span class="ctx-glyph" aria-hidden="true">
              ⊟
            </span>
            Close others
          </ContextMenu.Item>
          <ContextMenu.Item class="ctx-item" onSelect={() => {}}>
            <span class="ctx-glyph" aria-hidden="true">
              ⊠
            </span>
            Close all
          </ContextMenu.Item>
        </ContextMenu.Content>
      </ContextMenu.Portal>
    </ContextMenu>
  );
}
