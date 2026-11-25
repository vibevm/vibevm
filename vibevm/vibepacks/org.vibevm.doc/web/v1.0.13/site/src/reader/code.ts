/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-DERIVED-AT-BUILD */

/**
 * What a reader needs around a block of machine text: the language it is
 * in, a way to take it, and — for the two kinds of block the product
 * itself wrote — a line saying where it came from.
 *
 * The blocks are the pipeline's and are not touched. A toolbar is added
 * beside a fence, a note is added above a generated block, and the text
 * inside both stays byte for byte what the build produced, because the
 * `.md` projection, the `llms` corpus and this page have to agree about
 * it (`##DISC-MACHINE-MIRROR`).
 *
 * The expected output of an example is already under the command — the
 * pipeline emits it — so nothing here moves it. What was missing was the
 * word: `expected output` over a block that otherwise looks exactly like
 * output that just happened.
 */

import { all, copy, island, targetOf } from "./dom.ts";

const TICK_MS = 1500;

/** The label a fence wears, from the class the pipeline put on it. */
function languageOf(code: Element): string | null {
  for (const name of Array.from(code.classList)) {
    if (name.startsWith("language-")) {
      const tag = name.slice("language-".length);
      return tag.length === 0 ? null : tag;
    }
  }
  return null;
}

function note(text: string, ref: string | null): HTMLElement {
  const line = document.createElement("p");
  line.className = "block-note";
  line.textContent = ref === null ? text : `${text} `;
  if (ref !== null) {
    const code = document.createElement("code");
    code.textContent = ref;
    line.appendChild(code);
  }
  return line;
}

export function startCodeChrome(): () => void {
  const region = island();
  if (region === null) return () => undefined;

  const added: HTMLElement[] = [];

  for (const fence of all("pre", region)) {
    const code = fence.querySelector("code");
    if (code === null) continue;

    const toolbar = document.createElement("div");
    toolbar.className = "code-toolbar";

    const language = languageOf(code);
    if (language !== null) {
      const label = document.createElement("span");
      label.className = "code-toolbar__lang";
      label.textContent = language;
      toolbar.appendChild(label);
    }

    const button = document.createElement("button");
    button.type = "button";
    button.className = "code-toolbar__copy";
    button.textContent = "copy";
    button.setAttribute("data-copy-code", "");
    toolbar.appendChild(button);

    fence.appendChild(toolbar);
    added.push(toolbar);
  }

  /** The run and its expected output, told apart in words. */
  for (const output of all(".example .example-output", region)) {
    const line = note("expected output", null);
    output.parentNode?.insertBefore(line, output);
    added.push(line);
  }
  for (const output of all(".example .example-stderr", region)) {
    const line = note("expected error output", null);
    output.parentNode?.insertBefore(line, output);
    added.push(line);
  }

  /** A generated block says what generated it, and from what. */
  for (const derived of all("pre.derived", region)) {
    const line = note(
      `generated from ${derived.dataset["derived"] ?? "the product"}:`,
      derived.dataset["ref"] ?? null,
    );
    derived.parentNode?.insertBefore(line, derived);
    added.push(line);
  }

  const onClick = (event: Event): void => {
    const target = targetOf(event);
    if (target === null) return;
    const button = target.closest("[data-copy-code]");
    if (!(button instanceof HTMLElement)) return;
    const fence = button.closest("pre");
    const code = fence?.querySelector("code");
    const text = code?.textContent ?? "";
    if (text.length === 0) return;
    void copy(text).then((done) => {
      if (!done) return;
      button.classList.add("copied");
      window.setTimeout(() => button.classList.remove("copied"), TICK_MS);
    });
  };

  document.addEventListener("click", onClick);

  return () => {
    document.removeEventListener("click", onClick);
    for (const node of added) node.remove();
  };
}
