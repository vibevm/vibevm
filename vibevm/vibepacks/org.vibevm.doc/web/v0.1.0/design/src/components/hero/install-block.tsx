/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#STACK-DESIGN-FLOOR */

import { component$, useStyles$ } from "@qwik.dev/core";
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
};

/**
 * The install panel inside the hero: one line per platform, and the
 * command that follows.
 *
 * The prompt (`$`, `PS>`) is marked `user-select: none` in the
 * stylesheet rather than dropped, because a reader who selects the line
 * wants the command and not the shell's punctuation — and a reader who
 * is looking rather than copying wants to know which shell it is.
 *
 * Each line scrolls inside itself. A command is one unbreakable string:
 * wrapping it would invite someone to copy half.
 */
export const InstallBlock = component$<InstallBlockProps>((props) => {
  useStyles$(styles);
  return (
    <section class="install" aria-labelledby={props.headingId}>
      <div class="install__heading">
        <h2 id={props.headingId}>{props.title}</h2>
        <p>{props.lead}</p>
      </div>

      {props.commands.map((entry) => (
        <div key={entry.label} class="install__command">
          <div class="install__label">{entry.label}</div>
          <div class="install__cmd">
            <span class="install__prompt">{entry.prompt}</span>
            <code>{entry.command}</code>
          </div>
        </div>
      ))}

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
