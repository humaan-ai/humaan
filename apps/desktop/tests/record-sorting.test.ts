import { test } from "node:test";
import { strict as assert } from "node:assert";
import { statusRank } from "../src/lib/record-sorting";

test("statuses use meaningful order with unknown values last", () => {
  assert.ok(statusRank("review_state", "unreviewed")! < statusRank("review_state", "reviewed")!);
  assert.ok(statusRank("assistance", "yes")! < statusRank("assistance", "no")!);
  assert.ok(statusRank("outcome", "success")! < statusRank("outcome", "failure")!);
  assert.equal(statusRank("outcome", "unknown"), null);
});
