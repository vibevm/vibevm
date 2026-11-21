import assert from "node:assert/strict";
import { describe, it } from "node:test";

import {
  admitsAuthorship,
  authorshipChoices,
  EVERY_AUTHORSHIP,
} from "./authorship.ts";

/**
 * The mapping of four states onto three groups, which is the whole of
 * this rule and the only place it is decided (`##CARD-AUTHORSHIP`).
 *
 * It is measured here rather than through the browser because it is
 * arithmetic over one value: a test that had to open a shelf to ask what
 * «human-authored» admits would be measuring the shelf.
 */
describe("who wrote the prose, as a shelf is narrowed by it", () => {
  it("admits everything under the entry that narrows nothing", () => {
    for (const declared of ["human", "ai", "mixed", "nonsense", undefined]) {
      assert.equal(admitsAuthorship(EVERY_AUTHORSHIP, declared), true);
    }
  });

  it("puts each named word in its own group", () => {
    assert.equal(admitsAuthorship("human", "human"), true);
    assert.equal(admitsAuthorship("ai", "human"), false);
    assert.equal(admitsAuthorship("ai", "ai"), true);
    assert.equal(admitsAuthorship("human", "ai"), false);
  });

  /** Both hands are in the text, so both readers are told the truth. */
  it("puts a document written by both hands in both groups", () => {
    assert.equal(admitsAuthorship("human", "mixed"), true);
    assert.equal(admitsAuthorship("ai", "mixed"), true);
  });

  /**
   * The rule the field exists for. A documentation that declares nothing
   * is unknown, and a shelf that read silence as «a person wrote it»
   * would make exactly the claim nobody made.
   */
  it("leaves a document that says nothing out of both named groups", () => {
    assert.equal(admitsAuthorship("human", undefined), false);
    assert.equal(admitsAuthorship("ai", undefined), false);
    assert.equal(admitsAuthorship(EVERY_AUTHORSHIP, undefined), true);
  });

  /** A word from a newer pipeline is shelved, not refused, and grouped as silence. */
  it("treats a word this build does not know as a document that said nothing", () => {
    assert.equal(admitsAuthorship("human", "committee"), false);
    assert.equal(admitsAuthorship("ai", "committee"), false);
    assert.equal(admitsAuthorship(EVERY_AUTHORSHIP, "committee"), true);
  });

  it("offers everything first, then the two named groups", () => {
    const choices = authorshipChoices();
    assert.deepEqual(
      choices.map((one) => one.value),
      [EVERY_AUTHORSHIP, "human", "ai"],
    );
    assert.deepEqual(
      choices.map((one) => one.current),
      [true, false, false],
    );
    /* The entry that narrows nothing is the one place the silence rule
       can be read in words, so it says so. */
    assert.match(choices[0]?.note ?? "", /do not say who wrote them/);
  });
});
