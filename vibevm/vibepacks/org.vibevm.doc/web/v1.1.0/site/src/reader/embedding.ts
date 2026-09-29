/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#LOCAL-EMBEDDING-CONTRACT */

/**
 * The contract with a host that has the page inside a webview.
 *
 * Four messages travel, and the shape of each is the whole agreement.
 * Inward: `{ theme }` when the editor's own theme changes, `{ settings }`
 * when the host keeps the reading settings for us, and `{ open }` with a
 * `spec://` address the host wants shown. Outward: `{ settings }` when
 * the reader changes one, `{ openFile }` when they click a projection
 * that the editor should open instead of the webview, and `{ prompt }`
 * when they hand a prompt block to their agent.
 *
 * Everything arriving is DATA and is treated as such. A message is read
 * only for the three shapes above, every field is checked before it is
 * used, and an address is resolved by the site's own address map rather
 * than navigated to as given — a host that sent `javascript:` or a path
 * outside the documentation would get nothing. Nothing here is an
 * instruction to the page beyond opening a page of this documentation.
 *
 * In an ordinary browser tab none of this runs. The detection is one
 * question — is there a frame above us — and a page that is its own top
 * keeps its settings in storage, as it should.
 */

import { hrefOfSpecUri } from "../lib/href.ts";
import { all, targetOf } from "./dom.ts";
import type { ReaderSettings } from "./settings.ts";

/** Whether this page is being read inside a host application. */
export function isEmbedded(): boolean {
  try {
    return window.parent !== window;
  } catch {
    /* A cross-origin parent throws on comparison in some engines. */
    return true;
  }
}

function post(message: Record<string, unknown>): void {
  try {
    // The host is the reader's own editor and its origin is not known to
    // the build; nothing in these messages is a secret — a path, a theme
    // name, a font size — so the wildcard costs nothing it could leak.
    window.parent.postMessage(message, "*");
  } catch {
    /* No parent, or one that refuses messages: the page still works. */
  }
}

function record(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? Object.fromEntries(Object.entries(value))
    : null;
}

function settingsPatch(value: unknown): Partial<ReaderSettings> | null {
  const data = record(value);
  if (data === null) return null;
  const patch: {
    theme?: ReaderSettings["theme"];
    font?: number;
    width?: number;
    anchors?: boolean;
    contents?: ReaderSettings["contents"];
  } = {};
  const theme = data["theme"];
  if (theme === "dark" || theme === "light" || theme === "system") {
    patch.theme = theme;
  }
  const font = data["font"];
  if (typeof font === "number" && Number.isFinite(font)) patch.font = font;
  const width = data["width"];
  if (typeof width === "number" && Number.isFinite(width)) patch.width = width;
  const anchors = data["anchors"];
  if (typeof anchors === "boolean") patch.anchors = anchors;
  const contents = data["contents"];
  if (contents === "path" || contents === "sections") {
    patch.contents = contents;
  }
  return patch;
}

/** What the page hands to the host, and what the host hands back. */
export type EmbeddingOptions = {
  /** Apply settings the host sent. */
  readonly onSettings: (patch: Partial<ReaderSettings>) => void;
  /** The projections this page offers, which the editor opens as files. */
  readonly files: readonly string[];
};

export function startEmbedding(options: EmbeddingOptions): () => void {
  if (!isEmbedded()) return () => undefined;

  const onMessage = (event: MessageEvent): void => {
    const data = record(event.data);
    if (data === null) return;

    const theme = data["theme"];
    if (typeof theme === "string") {
      const patch = settingsPatch({ theme });
      if (patch !== null) options.onSettings(patch);
    }

    const settings = data["settings"];
    if (settings !== undefined) {
      const patch = settingsPatch(settings);
      if (patch !== null) options.onSettings(patch);
    }

    const open = data["open"];
    if (typeof open === "string") {
      const where = hrefOfSpecUri(open);
      if (where !== null) window.location.assign(where);
    }
  };

  /**
   * A click on a projection is a request to open a FILE. The webview
   * would happily navigate to the `.md` and show it as text; the editor
   * can open it properly, and that is the whole reason the host is there.
   */
  const onClick = (event: Event): void => {
    const target = targetOf(event);
    if (target === null) return;

    const hand = target.closest("[data-hand-to-agent]");
    if (hand instanceof HTMLElement) {
      event.preventDefault();
      const block = hand.closest(".prompt");
      const text = block?.querySelector(".prompt-text")?.textContent ?? "";
      if (text.length > 0) post({ prompt: text });
      return;
    }

    const link = target.closest("a[href]");
    if (!(link instanceof HTMLAnchorElement)) return;
    const href = link.getAttribute("href");
    if (href === null) return;
    if (!options.files.includes(href)) return;
    event.preventDefault();
    post({ openFile: href });
  };

  /**
   * The prompt blocks get their «hand to agent» button here and nowhere
   * else: on the public site there is no agent to hand anything to, and
   * a button that does nothing is a promise the page cannot keep.
   */
  const added: HTMLElement[] = [];
  for (const prompt of all(".prompt")) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "prompt__hand";
    button.textContent = "hand to agent";
    button.setAttribute("data-hand-to-agent", "");
    prompt.appendChild(button);
    added.push(button);
  }

  window.addEventListener("message", onMessage);
  document.addEventListener("click", onClick);

  return () => {
    window.removeEventListener("message", onMessage);
    document.removeEventListener("click", onClick);
    for (const button of added) button.remove();
  };
}

/** Tell the host the reader changed a setting. */
export function publishSettings(settings: ReaderSettings): void {
  post({ settings });
}
