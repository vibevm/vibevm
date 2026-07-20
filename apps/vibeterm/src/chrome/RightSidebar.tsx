// spec://vibeterm/PROP-044#regions (the right sidebar — column 4)
// A search field at the top, then a section of detail items (icon + label + subtext + badge).
// All inert placeholders — the reference layout's shape. (Dropped in split view by the chrome.)

import type { JSX } from "solid-js";
import { t } from "./i18n";

function SidebarItem(props: {
  glyph: string;
  label: string;
  sub: string;
  badge?: string;
}): JSX.Element {
  return (
    <div class="sb-item">
      <span class="sb-item-glyph" aria-hidden="true">
        {props.glyph}
      </span>
      <span class="sb-item-text">
        <span class="sb-item-label">{props.label}</span>
        <span class="sb-item-sub">{props.sub}</span>
      </span>
      {props.badge ? <span class="sb-item-badge">{props.badge}</span> : null}
    </div>
  );
}

export function RightSidebar(): JSX.Element {
  return (
    <aside class="right-sidebar" aria-label="details">
      <div class="sb-search">
        <span class="sb-search-glyph" aria-hidden="true">
          ⌕
        </span>
        <input class="sb-search-input" placeholder={t("sidebar.search")} disabled />
      </div>
      <div class="sb-section">
        <div class="sb-section-header">{t("sidebar.section")}</div>
        <SidebarItem glyph="▸" label="Terminal 1" sub="bash · 80×24" badge="3" />
        <SidebarItem glyph="▸" label="build" sub="cargo watch" badge="12" />
        <SidebarItem glyph="▸" label="logs" sub="tail -f" />
        <SidebarItem glyph="▸" label="repl" sub="node" />
      </div>
    </aside>
  );
}
