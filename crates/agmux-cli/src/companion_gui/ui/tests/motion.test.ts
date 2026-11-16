import { test } from "node:test";
import assert from "node:assert/strict";
import {
  motionDurations,
  motionTiming,
  sessionDirection,
} from "../src/motion-spec.ts";

test("reduced motion removes duration from every interaction category", () => {
  for (const kind of Object.keys(
    motionDurations,
  ) as (keyof typeof motionDurations)[])
    assert.equal(motionTiming(true, kind).duration, 0);
});
test("motion distinguishes immediate feedback from navigation and panels", () => {
  assert.equal(motionTiming(false, "control").duration, 0.12);
  assert.ok(motionTiming(false, "navigation").duration <= 0.24);
  assert.ok(
    motionTiming(false, "panel").duration >= 0.25 &&
      motionTiming(false, "panel").duration <= 0.3,
  );
  for (const kind of Object.keys(
    motionDurations,
  ) as (keyof typeof motionDurations)[]) {
    assert.ok(Number.isFinite(motionTiming(false, kind).duration));
    assert.ok(motionTiming(false, kind).duration > 0);
  }
});
test("directional navigation honors explicit keyboard direction at wraparound", () => {
  const ids = ["first", "middle", "last"];
  assert.equal(sessionDirection(ids, "first", "last"), 1);
  assert.equal(sessionDirection(ids, "last", "first"), -1);
  assert.equal(sessionDirection(ids, "last", "first", 1), 1);
  assert.equal(sessionDirection(ids, "first", "last", -1), -1);
  assert.equal(sessionDirection(ids, "missing", "first"), 1);
});
