/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-DESIGN-FLOOR */

import {
  component$,
  useSignal,
  useStyles$,
  useVisibleTask$,
} from "@qwik.dev/core";
import styles from "./styles.css?inline";

/** One platform's install line: what to call it, and what to type. */
export type InstallCommand = {
  /** The platform label above the line — mono, small, uppercase. */
  readonly label: string;
  /** The shell's own prompt, shown but not selectable. */
  readonly prompt: string;
  readonly command: string;
};

export type InstallBlockProps = {
  /** The id the block's own heading carries, for `aria-labelledby`. */
  readonly headingId: string;
  readonly title: string;
  readonly lead: string;
  readonly commands: readonly InstallCommand[];
  /** The line under the commands: «then add a spec stack». */
  readonly nextLabel: string;
  /** The command it offers, split so the tool's name can be set apart. */
  readonly nextCommandHead: string;
  readonly nextCommandTail: string;
  /** What the copy button is, for a reader who cannot see the glyph. */
  readonly copyLabel: string;
  /** What the line says back once the command is on the clipboard. */
  readonly copiedLabel: string;
};

/** How long the page says «copied» before going quiet again. */
const COPIED_MS = 1500;

/**
 * The install panel inside the hero: one line per platform, the button
 * that puts a line on the clipboard, and the command that follows.
 *
 * The prompt (`$`, `PS>`) is marked `user-select: none` in the
 * stylesheet rather than dropped, because a reader who selects the line
 * wants the command and not the shell's punctuation — and a reader who
 * is looking rather than copying wants to know which shell it is. The
 * button copies the command alone for the same reason: what is handed
 * over is what is typed.
 *
 * Each line scrolls inside itself, and the button stands outside that
 * scroller. A command is one unbreakable string — wrapping it would
 * invite someone to copy half — and a control that scrolled away with
 * the text it belongs to would be a control a reader has to hunt for.
 *
 * **The button is absent where it cannot work.** An insecure origin has
 * no `navigator.clipboard` at all, so the markup renders it hidden and
 * the first client frame reveals it: a button that silently does nothing
 * is worse than no button, and the server cannot know which of the two
 * this reader will be. The confirmation is a live region BESIDE the
 * button rather than inside it — the button's accessible name is its
 * `aria-label`, which would swallow any word put in its content.
 *
 * The handler is Qwik's own and not an inline script. Every inline
 * script on this site is named by the hash of its bytes in the policy the
 * build writes (`##SEO-SSR`), and one more of them would be one more
 * hash for a button.
 */
export const InstallBlock = component$<InstallBlockProps>((props) => {
  useStyles$(styles);

  const canCopy = useSignal(false);
  /** Which line was just copied, by its label; `""` when none was. */
  const copied = useSignal("");

  /* `document-idle` and not the default: the task answers «may this
     reader copy at all», and the default waits for the panel to be
     scrolled into view — which on a phone is after the reader has
     already reached the line and found no button there. */
  useVisibleTask$(
    () => {
      canCopy.value = "clipboard" in navigator;
    },
    { strategy: "document-idle" },
  );

  return (
    <section class="install" aria-labelledby={props.headingId}>
      <div class="install__heading">
        <h2 id={props.headingId}>{props.title}</h2>
        <p>{props.lead}</p>
      </div>

      {props.commands.map((entry) => {
        const label = entry.label;
        const command = entry.command;
        return (
          <div key={label} class="install__command">
            <div class="install__label">{label}</div>
            <div class="install__cmd">
              <span class="install__line">
                <span class="install__prompt">{entry.prompt}</span>
                <code>{command}</code>
              </span>
              <span class="install__copied" role="status">
                {copied.value === label ? props.copiedLabel : ""}
              </span>
              <button
                class="install__copy"
                type="button"
                hidden={!canCopy.value}
                aria-label={props.copyLabel}
                onClick$={async () => {
                  try {
                    await navigator.clipboard.writeText(command);
                  } catch {
                    /* A refused permission is an answer, not a failure to
                       report: the line stays quiet and the reader selects
                       it by hand, exactly as before there was a button. */
                    return;
                  }
                  copied.value = label;
                  setTimeout(() => {
                    if (copied.value === label) copied.value = "";
                  }, COPIED_MS);
                }}
              >
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 14 14"
                  fill="none"
                  aria-hidden="true"
                >
                  <rect
                    x="4.75"
                    y="4.75"
                    width="7.5"
                    height="7.5"
                    rx="1.6"
                    stroke="currentColor"
                    stroke-width="1.3"
                  />
                  <path
                    d="M9.25 2.75a1.5 1.5 0 0 0-1.5-1.5h-4.5a1.5 1.5 0 0 0-1.5 1.5v4.5a1.5 1.5 0 0 0 1.5 1.5"
                    stroke="currentColor"
                    stroke-width="1.3"
                    stroke-linecap="round"
                  />
                </svg>
              </button>
            </div>
          </div>
        );
      })}

      <div class="install__next">
        <span>{props.nextLabel}</span>
        <code>
          <b class="install__tool">{props.nextCommandHead}</b>
          {props.nextCommandTail}
        </code>
      </div>
    </section>
  );
});
