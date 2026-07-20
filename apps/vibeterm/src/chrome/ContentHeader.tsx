// spec://vibeterm/PROP-044#regions (the content-area top bar — column 3 header)
// The bar above the active terminal: a hash/terminal glyph + the active tab name on the left,
// a row of utility icons on the right (reference placeholders: search/pin/members/menu, plus two
// WORKING toggles — theme cycle + locale switch).

import { createSignal, type JSX } from "solid-js";
import { view } from "./bridge";
import { t, toggleLocale } from "./i18n";

const THEMES = ["dark-purple", "anthropic"] as const;
type Theme = (typeof THEMES)[number];

const [theme, setTheme] = createSignal<Theme>("dark-purple");
function applyTheme(next: Theme) {
  setTheme(next);
  document.documentElement.setAttribute("data-theme", next);
}
function cycleTheme() {
  const idx = THEMES.indexOf(theme());
  const next = THEMES[(idx + 1) % THEMES.length];
  if (next) applyTheme(next);
}
applyTheme("dark-purple");

export function ContentHeader(): JSX.Element {
  const title = () => {
    const v = view();
    if (!v || !v.activeTab) return t("app.title");
    return v.tabs.get(v.activeTab)?.title ?? t("app.title");
  };
  return (
    <header class="content-header">
      <div class="content-header-left">
        <span class="content-header-glyph" aria-hidden="true">
          #
        </span>
        <span class="content-header-title">{title()}</span>
        <span class="content-header-topic">a VibeTerm terminal</span>
      </div>
      <div class="content-header-right">
        <button class="ch-icon" title="search" aria-label="search" disabled>
          ⌕
        </button>
        <button class="ch-icon" title="pin" aria-label="pin" disabled>
          ☆
        </button>
        <button class="ch-icon" title="members" aria-label="members" disabled>
          ⊕
        </button>
        <span class="ch-sep" />
        <button class="ch-icon working" title={t("theme.toggle")} onClick={cycleTheme}>
          ◐
        </button>
        <button class="ch-icon working" title="locale" onClick={() => toggleLocale()}>
          {t("locale.toggle")}
        </button>
        <button class="ch-icon" title="menu" aria-label="menu" disabled>
          ≡
        </button>
      </div>
    </header>
  );
}
