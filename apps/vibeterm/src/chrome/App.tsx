// spec://vibeterm/PROP-044#regions, spec://vibeterm/design-system
// The shell window: four columns left-to-right — a narrow profiles rail, the contacts-style
// terminal list, the content area (header bar over a transparent region the per-tab terminal
// WebContentsViews main lays out over), and a right details sidebar. The content header carries
// the working theme/locale toggles; the rest of the chrome mirrors the reference layout.

import type { JSX } from "solid-js";
import { ProfilesRail } from "./ProfilesRail";
import { TabList } from "./TabList";
import { ContentHeader } from "./ContentHeader";
import { RightSidebar } from "./RightSidebar";
import { view } from "./bridge";
import "./theme.css";

export function App(): JSX.Element {
  // The sidebar drops when the active window is split (the reference's split-view shows the two
  // panes taking the full content width).
  const split = () => {
    const v = view();
    if (!v || v.windows.length === 0) return false;
    const w = v.windows.find((x) => x.id === v.activeWindow) ?? v.windows[0];
    return w ? w.panes.length >= 2 : false;
  };
  return (
    <div class="shell" classList={{ split: split() }}>
      <ProfilesRail />
      <TabList />
      <main class="content">
        <ContentHeader />
        <div id="content" />
      </main>
      <RightSidebar />
    </div>
  );
}
