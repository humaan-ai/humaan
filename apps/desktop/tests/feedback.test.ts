import { test } from "node:test";
import { strict as assert } from "node:assert";
import { get } from "svelte/store";
import { createFeedback } from "../src/lib/feedback";

test("dismissing an export toast preserves its path and unread history", () => {
  const feedback = createFeedback();
  const id = feedback.publish({ title: "Export complete", copyText: "/tmp/export", popup: true });
  feedback.dismiss(id);
  const notice = get(feedback).notices[0]!;
  assert.equal(notice.popup, false);
  assert.equal(notice.copyText, "/tmp/export");
  assert.equal(notice.unread, true);
  feedback.openPanel();
  assert.equal(get(feedback).notices[0]!.unread, false);
});

test("an old footer timer cannot erase a newer result", () => {
  const feedback = createFeedback();
  const old = feedback.publish({ title: "Opened in Rerun" });
  feedback.publish({ title: "Import complete" });
  feedback.clearStatus(old);
  assert.equal(get(feedback).status?.title, "Import complete");
});

test("finishing one activity preserves other operations and their details", () => {
  const feedback = createFeedback();
  const first = feedback.start("Importing");
  const second = feedback.start("Setting up tools");
  feedback.updateActivity(second, "Installing packages");
  feedback.finish(first);
  assert.equal(get(feedback).activities.length, 1);
  assert.equal(get(feedback).activities[0]!.detail, "Installing packages");
  feedback.clearHistory();
  assert.equal(get(feedback).activities.length, 1);
});

test("project reset removes paths, popups, status, and activities", () => {
  const feedback = createFeedback();
  feedback.publish({ title: "Failed", detail: "/private/project", kind: "error", popup: true });
  feedback.start("Exporting");
  feedback.openPanel();
  feedback.reset();
  assert.deepEqual(get(feedback), { notices: [], activities: [], status: null, panelOpen: false });
});

test("visible history receives new events without redundant popups; history is bounded", () => {
  const feedback = createFeedback();
  feedback.openPanel();
  for (let i = 0; i < 60; i++) feedback.publish({ title: `Event ${i}`, popup: true });
  const state = get(feedback);
  assert.equal(state.notices.length, 50);
  assert.equal(state.notices[0]!.title, "Event 59");
  assert.ok(state.notices.every(n => !n.popup && !n.unread));
});
