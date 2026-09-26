import { strict as assert } from "node:assert";
import { mock, test } from "node:test";
import type { WsServerMessage } from "@vigo/types";
import { openLiveSocket, toWsUrl, type MinimalWebSocket } from "./ws.ts";

class FakeSocket implements MinimalWebSocket {
  sent: string[] = [];
  closed = false;
  onopen: ((ev: unknown) => void) | null = null;
  onmessage: ((ev: { data: unknown }) => void) | null = null;
  onclose: ((ev: { code: number }) => void) | null = null;
  onerror: ((ev: unknown) => void) | null = null;
  send(data: string) {
    this.sent.push(data);
  }
  close() {
    this.closed = true;
  }
  serverSays(msg: WsServerMessage) {
    this.onmessage?.({ data: JSON.stringify(msg) });
  }
  drop(code = 1006) {
    this.onclose?.({ code });
  }
}

const flush = () => new Promise((r) => setImmediate(r));

function harness(tokens: (force: boolean) => string | null = () => "tok") {
  const sockets: FakeSocket[] = [];
  const messages: WsServerMessage[] = [];
  const statuses: string[] = [];
  const forced: boolean[] = [];
  const handle = openLiveSocket({
    url: "ws://x/v1/ws/picker",
    getToken: (force) => {
      forced.push(force);
      return Promise.resolve(tokens(force));
    },
    onMessage: (m) => messages.push(m),
    onStatus: (s) => statuses.push(s),
    createSocket: () => {
      const s = new FakeSocket();
      sockets.push(s);
      return s;
    },
    random: () => 0.5,
  });
  return { sockets, messages, statuses, forced, handle };
}

void test("authenticates first, then forwards events", async () => {
  const h = harness();
  h.sockets[0]!.onopen?.({});
  await flush();
  assert.deepEqual(JSON.parse(h.sockets[0]!.sent[0]!), { type: "auth", token: "tok" });
  h.sockets[0]!.serverSays({ type: "ready" });
  assert.equal(h.statuses.at(-1), "open");
  assert.deepEqual(h.messages, [], "no resync on the first connect");
});

void test("reconnects with backoff and asks for a resync", async () => {
  mock.timers.enable({ apis: ["setTimeout"] });
  try {
    const h = harness();
    h.sockets[0]!.onopen?.({});
    await flush();
    h.sockets[0]!.serverSays({ type: "ready" });

    h.sockets[0]!.drop();
    assert.equal(h.statuses.at(-1), "reconnecting");
    assert.equal(h.sockets.length, 1);
    mock.timers.tick(374); // first delay is 250..500ms; random 0.5 -> 375ms
    assert.equal(h.sockets.length, 1);
    mock.timers.tick(1);
    assert.equal(h.sockets.length, 2);

    h.sockets[1]!.onopen?.({});
    await flush();
    h.sockets[1]!.serverSays({ type: "ready" });
    assert.deepEqual(h.messages, [{ type: "resync" }]);
  } finally {
    mock.timers.reset();
  }
});

void test("refreshes the token after the server says it expired", async () => {
  mock.timers.enable({ apis: ["setTimeout"] });
  try {
    const h = harness();
    h.sockets[0]!.onopen?.({});
    await flush();
    h.sockets[0]!.drop(4008);
    mock.timers.tick(1_000);
    h.sockets[1]!.onopen?.({});
    await flush();
    assert.deepEqual(h.forced, [false, true]);
  } finally {
    mock.timers.reset();
  }
});

void test("gives up on FORBIDDEN and when closed by the app", async () => {
  mock.timers.enable({ apis: ["setTimeout"] });
  try {
    const h = harness();
    h.sockets[0]!.onopen?.({});
    await flush();
    h.sockets[0]!.serverSays({ type: "error", code: "FORBIDDEN", message: "not allowed" });
    h.sockets[0]!.drop(4003);
    mock.timers.tick(60_000);
    assert.equal(h.sockets.length, 1);
    assert.equal(h.statuses.at(-1), "stopped");

    const h2 = harness();
    h2.handle.close();
    assert.equal(h2.sockets[0]!.closed, true);
    mock.timers.tick(60_000);
    assert.equal(h2.sockets.length, 1);
  } finally {
    mock.timers.reset();
  }
});

void test("toWsUrl", () => {
  assert.equal(toWsUrl("http://10.0.2.2:8080/", "/v1/ws/picker"), "ws://10.0.2.2:8080/v1/ws/picker");
  assert.equal(toWsUrl("https://api.vigo.in", "/v1/ws/picker"), "wss://api.vigo.in/v1/ws/picker");
});
