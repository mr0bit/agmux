import { test } from "node:test";
import assert from "node:assert/strict";
import {
  clampReviewWidth,
  moveQueuedMessage,
  nextSessionId,
  statuses,
} from "../src/workspace.ts";
import { initialSessions } from "../src/demo.ts";

test("keyboard session navigation wraps at both ends", () => {
  assert.equal(
    nextSessionId(initialSessions, "workspace", -1),
    initialSessions.at(-1)!.id,
  );
  assert.equal(
    nextSessionId(initialSessions, initialSessions.at(-1)!.id, 1),
    "workspace",
  );
  assert.equal(nextSessionId([], "keep-current", 1), "keep-current");
});
test("review divider preserves usable chat width and rejects invalid widths", () => {
  assert.equal(clampReviewWidth(850, 1000), 660);
  assert.equal(clampReviewWidth(100, 1000), 280);
  assert.equal(clampReviewWidth(Number.NaN, 1200), 420);
  assert.equal(clampReviewWidth(2000, 2000), 900);
});
test("queue reordering preserves message identity, attachments and send order", () => {
  const first = { id: "one", text: "First", attachments: ["notes.txt"] };
  const second = { id: "two", text: "Second", attachments: [] };
  const queue = [first, second];
  const reordered = moveQueuedMessage(queue, "two", -1);
  assert.deepEqual(
    reordered.map((item) => item.id),
    ["two", "one"],
  );
  assert.equal(reordered[1], first);
  assert.deepEqual(queue, [first, second]);
  assert.equal(moveQueuedMessage(queue, "one", -1), queue);
  assert.equal(moveQueuedMessage(queue, "unknown", 1), queue);
});
test("all preview connection states have visible labels", () => {
  for (const state of [
    "attention",
    "working",
    "idle",
    "stopped",
    "reconnecting",
  ] as const) {
    assert.ok(statuses[state].label);
    assert.ok(statuses[state].color);
  }
});
