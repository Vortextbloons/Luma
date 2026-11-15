import test from "node:test";
import assert from "node:assert/strict";
import { isTap, stickVector } from "../src/lib/gestures.ts";

test("the stick clamps diagonal movement to a circle", () => {
  const vector = stickVector(1000, 1000, 268);
  assert.ok(Math.abs(Math.hypot(vector.x, vector.y) - 1) < 1e-9);
});

test("the stick maps a normal drag into a relative vector", () => {
  const vector = stickVector(50, 0, 268);
  assert.ok(vector.x > 0.8 && vector.x < 0.9);
  assert.equal(vector.y, 0);
  assert.deepEqual(stickVector(0, 0, 268), { x: 0, y: 0 });
});

test("a tap is short and stationary", () => {
  assert.equal(isTap(5, 349), true);
  assert.equal(isTap(6, 349), false);
  assert.equal(isTap(5, 350), false);
});
