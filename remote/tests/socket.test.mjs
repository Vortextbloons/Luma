import test from "node:test";
import assert from "node:assert/strict";
import { createCommandSocket } from "../src/lib/socket.ts";

function setup(options = {}) {
  const connections = [];
  class Socket {
    static OPEN = 1;
    readyState = 1;
    sent = [];
    onopen = null;
    onmessage = null;
    onclose = null;
    constructor() {
      connections.push(this);
    }
    send(value) {
      this.sent.push(JSON.parse(value));
    }
    close() {
      this.readyState = 3;
      this.onclose?.();
    }
  }
  const client = createCommandSocket({ create: () => new Socket(), ...options });
  client.connect();
  return { client, connections };
}

test("web remote correlates acknowledgments and sends each command once", async () => {
  const { client, connections } = setup();
  const first = client.request("right");
  const second = client.request("activate");
  const connection = connections[0];
  assert.deepEqual(connection.sent.map(value => value.command), ["right", "activate"]);

  connection.onmessage({ data: JSON.stringify({ id: 2, state: {} }) });
  connection.onmessage({ data: JSON.stringify({ id: 1, state: {} }) });
  assert.equal((await first).id, 1);
  assert.equal((await second).id, 2);
});

test("disconnect rejects pending input without replaying it after reconnect", async () => {
  const { client, connections } = setup();
  const result = client.request("activate");
  connections[0].close();
  await assert.rejects(result, /Connection interrupted/);
  client.connect();
  assert.equal(connections.length, 2);
  assert.equal(connections[1].sent.length, 0);
  connections[1].close();
});

test("a timed-out command closes the socket", async () => {
  const { client, connections } = setup({ timeout: 5 });
  await assert.rejects(client.request("right"), /interrupted|did not respond/);
  assert.equal(connections[0].readyState, 3);
});
