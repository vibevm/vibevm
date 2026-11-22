/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#READER-SETTINGS */

/**
 * Everything the reader remembers, and the one rule about remembering
 * it: a browser that refuses storage must not take the page down with
 * it.
 *
 * `localStorage` does not merely return `null` when site data is
 * blocked — the property access itself throws, and so does a write when
 * the quota is full or the page is in a partitioned third-party frame.
 * A reader who has cookies off would otherwise get a blank page instead
 * of a page that has forgotten their font size, which is the wrong trade
 * by a wide margin. So every read and every write is wrapped, and a
 * failure means «nothing was stored», which is the same state as never
 * having chosen.
 */

/** Everything the reader stores is under one visible prefix. */
const PREFIX = "vibe-doc:";

export function readLocal(key: string): string | null {
  try {
    return window.localStorage.getItem(PREFIX + key);
  } catch {
    return null;
  }
}

export function writeLocal(key: string, value: string): void {
  try {
    window.localStorage.setItem(PREFIX + key, value);
  } catch {
    /* A reader with storage off keeps the page and loses the memory. */
  }
}

export function removeLocal(key: string): void {
  try {
    window.localStorage.removeItem(PREFIX + key);
  } catch {
    /* As above: forgetting is allowed to fail. */
  }
}

export function readSession(key: string): string | null {
  try {
    return window.sessionStorage.getItem(PREFIX + key);
  } catch {
    return null;
  }
}

export function writeSession(key: string, value: string): void {
  try {
    window.sessionStorage.setItem(PREFIX + key, value);
  } catch {
    /* As above. */
  }
}

/**
 * The reading position is keyed by the page's path, so two pages do not
 * fight over one memory and a language switch does not inherit the
 * other language's place.
 */
export function positionKey(path: string): string {
  return `reading-pos:${path}`;
}
