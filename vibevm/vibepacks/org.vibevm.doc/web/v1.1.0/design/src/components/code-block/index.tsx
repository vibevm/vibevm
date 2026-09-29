/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#PIPE-ONE-CONTENT-PATH */

import { Slot, component$, useStyles$ } from "@qwik.dev/core";
import styles from "./styles.css?inline";
import tableStyles from "../table-scroll/styles.css?inline";

/**
 * The chrome a fence, an example and a generated block wear — and the
 * one component in the system that renders none of what it styles.
 *
 * Every one of those blocks arrives inside the island, finished, from
 * the Rust pipeline; the copy button and the language label on a fence
 * and the expand control on a table are put there by the reader, over
 * markup no component owns. Their styles still have to live somewhere
 * reviewable, in the same token vocabulary as everything else and inside
 * the same contrast audit — a stylesheet assembled in a string by a
 * script would be outside both.
 *
 * So this wraps the region and contributes nothing but a stylesheet. It
 * is the same arrangement the prose component already makes for the
 * island's own vocabulary, for the same reason: what the page shows and
 * what renders it are two different questions here.
 */
export const CodeChrome = component$(() => {
  useStyles$(styles);
  // The scrolling region a wide table is put into is the table-scroll
  // component's, and the reader builds one out of plain elements rather
  // than rendering the component — so its stylesheet is asked for HERE,
  // by name, instead of being copied. One definition, two callers.
  useStyles$(tableStyles);
  return <Slot />;
});
