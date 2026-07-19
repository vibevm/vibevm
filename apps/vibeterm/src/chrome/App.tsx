// spec://vibeterm/PROP-044#regions, spec://vibeterm/design-system
// The shell window: a left rail (the contacts-style terminal list + controls) and a right content
// region that is intentionally transparent (the per-tab terminal WebContentsViews are native surfaces
// main lays out over it). Live theme + locale switching from the start.

import { createSignal, type JSX } from "solid-js";
import { TabList } from "./TabList";
import { t, toggleLocale } from "./i18n";
import "./theme.css";

const THEMES = ["dark-purple", "anthropic"] as const;
type Theme = (typeof THEMES)[number];

export function App(): JSX.Element {
  const [theme, setTheme] = createSignal<Theme>("dark-purple");

  const applyTheme = (next: Theme) => {
    setTheme(next);
    document.documentElement.setAttribute("data-theme", next);
  };

  const cycleTheme = () => {
    const idx = THEMES.indexOf(theme());
    const next = THEMES[(idx + 1) % THEMES.length];
    if (next) applyTheme(next);
  };

  return (
    <div class="shell">
      <aside class="rail">
        <header class="rail-app">
          <span class="app-glyph" aria-hidden="true">
            ◆
          </span>
          <span class="app-name">{t("app.title")}</span>
        </header>
        <TabList />
        <footer class="rail-footer">
          <button class="rail-btn" onClick={cycleTheme} title={t("theme.toggle")}>
            ◐
          </button>
          <button class="rail-btn" onClick={() => toggleLocale()} title="locale">
            {t("locale.toggle")}
          </button>
        </footer>
      </aside>
      <main id="content" />
    </div>
  );
}
