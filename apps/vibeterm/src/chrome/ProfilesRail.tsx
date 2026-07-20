// spec://vibeterm/PROP-044#regions (the profiles rail — column 1)
// The far-left narrow rail: workspace/quick-access icons on top, colored profile avatars below
// a divider, and a user avatar + settings at the bottom. All inert placeholders for now (the
// reference layout's shape, not its function).

import type { JSX } from "solid-js";

function RailIcon(props: { label: string; children: JSX.Element; active?: boolean }): JSX.Element {
  return (
    <button class="rail-icon" classList={{ active: props.active }} title={props.label}>
      {props.children}
    </button>
  );
}

function Avatar(props: { label: string; color: string; active?: boolean }): JSX.Element {
  return (
    <button
      class="rail-avatar"
      classList={{ active: props.active }}
      title={props.label}
      style={{ background: props.color }}
    >
      {props.label}
    </button>
  );
}

export function ProfilesRail(): JSX.Element {
  return (
    <nav class="profiles-rail" aria-label="profiles">
      <div class="rail-top">
        <RailIcon label="Home" active>
          ◆
        </RailIcon>
        <RailIcon label="Direct">✉</RailIcon>
        <RailIcon label="Browse">⌕</RailIcon>
        <div class="rail-divider" />
        <Avatar label="V" color="var(--vibe-accent)" active />
        <Avatar label="T" color="var(--vibe-foam)" />
        <Avatar label="S" color="var(--vibe-gold)" />
        <button class="rail-add" title="Add profile" aria-label="add profile">
          +
        </button>
      </div>
      <div class="rail-bottom">
        <button class="rail-user" title="settings">
          <span class="rail-user-avatar" />
          <span class="rail-user-gear">⚙</span>
        </button>
      </div>
    </nav>
  );
}
