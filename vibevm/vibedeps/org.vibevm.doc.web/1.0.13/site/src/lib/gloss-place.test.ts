import assert from "node:assert/strict";
import test from "node:test";

import { GAP, MARGIN, place, type Box, type Viewport } from "./gloss-place.ts";

const VIEW: Viewport = { width: 1440, height: 900, scrollX: 0, scrollY: 0 };

function box(left: number, top: number, width: number, height: number): Box {
  return { left, top, width, height };
}

test("a card stands under the term when there is room under it", () => {
  const link = box(400, 300, 70, 20);
  const found = place(link, box(0, 0, 320, 120), VIEW);
  assert.equal(found.side, "below");
  assert.equal(found.top, 300 + 20 + GAP);
  assert.equal(found.left, 400);
});

test("a card stands over the term when the window ends first", () => {
  // The term is near the bottom: 820 + 20 + 8 + 120 + 12 is past 900.
  const link = box(400, 820, 70, 20);
  const found = place(link, box(0, 0, 320, 120), VIEW);
  assert.equal(found.side, "above");
  assert.equal(found.top, 820 - 120 - GAP);
});

test("a card never covers the term it explains", () => {
  const card = box(0, 0, 320, 120);
  for (const top of [0, 40, 300, 700, 820, 870]) {
    const link = box(400, top, 70, 20);
    const found = place(link, card, VIEW);
    const bottom = found.top + card.height;
    const covered = found.top < top + link.height && bottom > top;
    assert.equal(covered, false, `a card over the term at ${top}px`);
  }
});

test("a card near the right edge is pulled back inside the window", () => {
  const found = place(box(1300, 300, 70, 20), box(0, 0, 320, 120), VIEW);
  assert.equal(found.left, VIEW.width - MARGIN - 320);
  assert.ok(found.left + 320 + MARGIN <= VIEW.width);
});

test("a card wider than the window starts at the left margin", () => {
  const narrow: Viewport = { ...VIEW, width: 300 };
  const found = place(box(10, 300, 70, 20), box(0, 0, 320, 120), narrow);
  assert.equal(found.left, MARGIN);
});

test("with neither side roomy the card goes below and the page scrolls", () => {
  const short: Viewport = { ...VIEW, height: 200 };
  const found = place(box(400, 90, 70, 20), box(0, 0, 320, 300), short);
  assert.equal(found.side, "below");
  assert.equal(found.top, 90 + 20 + GAP);
});

test("the answer is in document coordinates, so a scrolled page holds", () => {
  const scrolled: Viewport = { ...VIEW, scrollX: 40, scrollY: 1200 };
  const found = place(box(400, 300, 70, 20), box(0, 0, 320, 120), scrolled);
  assert.equal(found.top, 1200 + 300 + 20 + GAP);
  assert.equal(found.left, 40 + 400);
});
