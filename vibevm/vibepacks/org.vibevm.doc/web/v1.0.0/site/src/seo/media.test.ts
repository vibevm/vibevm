/** @scope spec://org.vibevm.core/vibevm/common/PROP-057#CARD-PREVIEW-COMPOSED */

import assert from "node:assert/strict";
import test from "node:test";

import type { DocPackage } from "../generated/doc-manifest.ts";
import { cardPreviewOf, parseMediaMap } from "./media.ts";

/** A card with nothing on it but the members every card has. */
function card(media?: DocPackage["media"]): DocPackage {
  return {
    group: "com.example.docs",
    name: "fixture-manual",
    version: "0.1.0",
    publisher: "com.example.docs",
    title: "Fixture Manual",
    abstract: "What it covers: nothing real.",
    lang: "en",
    status: "community",
    subjects: [],
    audiences: [],
    rendered_at: "2026-09-12T09:00:00Z",
    ...(media === undefined ? {} : { media }),
  };
}

/**
 * The manifest carries the addresses of the card's three images, and the
 * shell shows what it names rather than searching for it. Until X-055
 * the only path was the search — the build walking the copied tree for
 * the one 1200×630 PNG in it — which answers the right way for a package
 * that has a card and cannot tell a package with none from a tree the
 * build was never given.
 */
test("the card of a page is the address its own manifest names", () => {
  const at = cardPreviewOf(
    card({
      icon: "media/aaa.svg",
      banner: "media/bbb.svg",
      preview: "media/ccc.png",
    }),
  );
  assert.equal(
    at,
    "/doc/com.example.docs/fixture-manual/0.1.0/media/ccc.png",
    "beside the package, at the version number and never at `latest`",
  );
});

/**
 * A manifest older than the field is still a manifest (PROP-044 §4.4),
 * and the search remains for it. In a plain Node run no build has handed
 * one over, so the answer is «no card» — which is what makes the page
 * name the site's own picture instead of an address that answers 404.
 */
test("a manifest that names none falls through to what the build found", () => {
  assert.equal(cardPreviewOf(card()), undefined);
});

test("a map the environment did not carry is no map at all", () => {
  assert.deepEqual(parseMediaMap(undefined), {});
  assert.deepEqual(parseMediaMap("not json"), {});
  assert.deepEqual(parseMediaMap('{"a/b@1.0.0":{"preview":"media/x.png"}}'), {
    "a/b@1.0.0": { preview: "media/x.png" },
  });
});
