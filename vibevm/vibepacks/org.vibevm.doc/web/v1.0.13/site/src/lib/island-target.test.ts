import assert from "node:assert/strict";
import test from "node:test";

import { islandTarget, type ClickedNode } from "./island-target.ts";

/** Build a chain of stand-in elements, outermost first. */
function chain(
  ...levels: ReadonlyArray<{ classes: string[]; attrs: Record<string, string> }>
): ClickedNode {
  let parent: ClickedNode | null = null;
  for (const level of levels) {
    const node: ClickedNode = {
      parent,
      classes: level.classes,
      attribute: (name: string) => level.attrs[name] ?? null,
    };
    parent = node;
  }
  if (parent === null) throw new Error("a chain needs at least one level");
  return parent;
}

test("a click on a block number is the block's address", () => {
  const clicked = chain(
    { classes: ["doc-page"], attrs: {} },
    { classes: [], attrs: { "data-p": "7" } },
    { classes: ["p-anchor"], attrs: { id: "p07", href: "#p07" } },
  );
  assert.deepEqual(islandTarget(clicked), { kind: "anchor", block: "p07" });
});

test("a click on a quoted rule carries the spec address, not the page link", () => {
  const clicked = chain(
    { classes: ["rule"], attrs: {} },
    {
      classes: ["rule"],
      attrs: {
        href: "/doc/com.example/subject/latest/common/PROP-001/",
        "data-uri": "spec://com.example/subject/common/PROP-001#A-RULE",
      },
    },
    { classes: [], attrs: {} },
  );
  assert.deepEqual(islandTarget(clicked), {
    kind: "rule",
    uri: "spec://com.example/subject/common/PROP-001#A-RULE",
  });
});

test("an ordinary link stays an ordinary link", () => {
  const clicked = chain({ classes: [], attrs: { href: "/somewhere/" } });
  assert.deepEqual(islandTarget(clicked), {
    kind: "link",
    href: "/somewhere/",
  });
});

test("prose is not an intent", () => {
  const clicked = chain(
    { classes: ["doc-page"], attrs: {} },
    { classes: [], attrs: { "data-p": "3" } },
  );
  assert.deepEqual(islandTarget(clicked), { kind: "none" });
  assert.deepEqual(islandTarget(null), { kind: "none" });
});

test("the walk gives up before it leaves the island", () => {
  const deep: { classes: string[]; attrs: Record<string, string> }[] = [
    { classes: [], attrs: { href: "/the-layout-wrapper/" } },
  ];
  for (let i = 0; i < 20; i += 1) deep.push({ classes: [], attrs: {} });
  assert.deepEqual(islandTarget(chain(...deep)), { kind: "none" });
});
