import test from "node:test";
import assert from "node:assert/strict";
import { formatClock, TimelineModel } from "../src/lib/timeline.ts";

function setup() {
  const calls = [];
  const answers = [];
  const notices = [];
  const model = new TimelineModel(
    position => {
      calls.push(position);
      return new Promise(resolve => answers.push(resolve));
    },
    () => true,
    () => {},
    message => notices.push(message),
  );
  return { model, calls, answers, notices };
}

const tick = () => new Promise(resolve => setImmediate(resolve));
const answer = (queue, data) => queue.shift()({ response: { ok: true, status: 200 }, data });

test("timecodes are stable for short and long media", () => {
  assert.equal(formatClock(0), "0:00");
  assert.equal(formatClock(9), "0:09");
  assert.equal(formatClock(60), "1:00");
  assert.equal(formatClock(3600), "60:00");
});

test("a drag preserves the chosen seek until the pending refresh settles", async () => {
  const { model, calls, answers } = setup();
  const first = model.refresh();
  model.beginDrag();
  model.preview(3600);
  model.endDrag(3600);
  assert.equal(calls.length, 1);

  answer(answers, { timeline: { position: 10, duration: 7200 } });
  await first;
  await tick();
  assert.deepEqual(calls, [undefined, 3600]);

  answer(answers, { timeline: { position: 3600, duration: 7200 } });
  await tick();
  assert.equal(model.position, 3600);
  assert.equal(model.duration, 7200);
  assert.equal(model.available, true);
});

test("a rejected seek is reported and a later timeline recovers", async () => {
  const { model, answers, notices } = setup();
  model.beginDrag();
  model.endDrag(90);
  answer(answers, { timeline: { error: "The player declined the seek." } });
  await tick();
  assert.equal(model.duration, 0);
  assert.deepEqual(notices, ["The player declined the seek."]);

  const refresh = model.refresh();
  answer(answers, { timeline: { position: 20, duration: 120 } });
  await refresh;
  assert.equal(model.duration, 120);
  assert.equal(model.position, 20);
  assert.equal(model.available, true);
});
