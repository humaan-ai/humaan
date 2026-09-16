import { test } from "node:test";
import { strict as assert } from "node:assert";
import { eligibilityReason, preparationBlockers } from "../src/lib/dataset-preparation";
const ready = { ready: true, custom: false, installing: false, phase: "Ready", error: null };
test("excluded episodes and unchecked confirmation produce actionable requirements", () => {
  const blockers = preparationBlockers(ready, 2, 0, 0, false);
  assert.equal(blockers.length, 3);
  assert.ok(blockers[0]?.includes("Exclude to Train"));
  assert.ok(blockers[1]?.includes("Validation"));
});
test("complete prerequisites allow preparation", () => {
  assert.deepEqual(preparationBlockers(ready, 2, 1, 1, true), []);
});
test("insufficient eligibility explains immutable reviews", () => {
  const blockers = preparationBlockers(ready, 1, 1, 0, true);
  assert.equal(blockers.length, 1);
  assert.ok(blockers[0]?.includes("existing sets keep their saved reviews"));
});
test("tools states have distinct remedies", () => {
  assert.ok(preparationBlockers({ ...ready, ready: false }, 2, 1, 1, true)[0]?.includes("Set up"));
  assert.ok(preparationBlockers({ ...ready, installing: true }, 2, 1, 1, true)[0]?.includes("Wait"));
  assert.ok(preparationBlockers(null, 2, 1, 1, true)[0]?.includes("Refresh"));
});
test("unknown assistance does not silently pass eligibility", () => {
  assert.ok(eligibilityReason({ review_state: "reviewed", outcome: "success", assistance: "unknown" })?.includes("assistance is unknown"));
  assert.equal(eligibilityReason({ review_state: "reviewed", outcome: "success", assistance: "no" }), null);
});
