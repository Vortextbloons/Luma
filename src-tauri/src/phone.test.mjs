import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import test from 'node:test';
import assert from 'node:assert/strict';

const source = readFileSync(new URL('./phone.html', import.meta.url), 'utf8');
const transport = source.slice(source.indexOf('  let socket;'), source.indexOf('  async function send(command, button)'));
function setup() {
  const connections = [];
  class WebSocket {
    static OPEN = 1;
    readyState = 1;
    sent = [];
    constructor() { connections.push(this); }
    send(value) { this.sent.push(JSON.parse(value)); }
    close() { this.readyState = 3; this.onclose(); }
  }
  const context = vm.createContext({ WebSocket, paired: true, document: { visibilityState: 'visible' }, location: { protocol: 'http:', host: 'localhost:9877' }, setTimeout, clearTimeout, applyState() {}, setLink() {} });
  vm.runInContext(transport, context);
  vm.runInContext('connectSocket()', context);
  return { context, connections, request: command => vm.runInContext(`socketRequest(${JSON.stringify(command)})`, context) };
}

test('web remote correlates acknowledgments and sends each command once', async () => {
  const { connections, request } = setup();
  const first = request('right'), second = request('activate');
  const connection = connections[0];
  assert.deepEqual(connection.sent.map(value => value.command), ['right', 'activate']);
  connection.onmessage({ data: JSON.stringify({ id: 2, state: {} }) });
  connection.onmessage({ data: JSON.stringify({ id: 1, state: {} }) });
  assert.equal((await first).id, 1);
  assert.equal((await second).id, 2);
});

test('disconnect rejects pending input without replaying it after reconnect', async () => {
  const { context, connections, request } = setup();
  const result = request('activate');
  connections[0].close();
  await assert.rejects(result, /Connection interrupted/);
  vm.runInContext('connectSocket()', context);
  assert.equal(connections.length, 2);
  assert.equal(connections[1].sent.length, 0);
  connections[1].close();
});
