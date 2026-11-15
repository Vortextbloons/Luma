import test from "node:test";
import assert from "node:assert/strict";
import { PointerController } from "../src/lib/pointer.ts";

function setup() {
  const calls = [];
  const answers = [];
  const events = [];
  const controller = new PointerController({
    owner: "owner",
    post: body => {
      calls.push(body);
      return new Promise(resolve => answers.push(resolve));
    },
    onUnauthorized: () => events.push("unauthorized"),
    onError: message => events.push(message),
    onReleased: () => events.push("released"),
  });
  return { controller, calls, answers, events };
}

const answer = (queue, data, status = 200) => {
  queue.shift()({ response: { status, ok: status >= 200 && status < 300 }, data });
};

test("touchpad updates coalesce and a physical takeover stops the pointer", async () => {
  const { controller, calls, answers, events } = setup();
  controller.start();
  controller.move(0.8, 0);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].action, "start");

  answer(answers, { released: false });
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(calls.length, 2);
  assert.equal(calls[1].action, "move");
  assert.equal(calls[1].x, 0.8);

  answer(answers, { released: true });
  await new Promise(resolve => setImmediate(resolve));
  assert.deepEqual(events, ["released"]);
  assert.equal(calls.length, 2);

  controller.move(0.4, 0);
  assert.equal(calls.length, 2);
});

test("a stop still goes through after a takeover", async () => {
  const { controller, calls, answers } = setup();
  controller.start();
  answer(answers, { released: true });
  await new Promise(resolve => setImmediate(resolve));
  controller.stop();
  assert.equal(calls.at(-1).action, "stop");
});

test("unauthorized pointer input asks for pairing", async () => {
  const { controller, answers, events } = setup();
  controller.start();
  answer(answers, null, 401);
  await new Promise(resolve => setImmediate(resolve));
  assert.deepEqual(events, ["unauthorized"]);
});
