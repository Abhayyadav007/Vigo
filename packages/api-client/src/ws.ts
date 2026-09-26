import type { WsServerMessage } from "@vigo/types";

export type LiveStatus = "connecting" | "open" | "reconnecting" | "stopped";

/** The subset of the WebSocket API we use (browser, React Native and Node all provide it). */
export interface MinimalWebSocket {
  send(data: string): void;
  close(code?: number, reason?: string): void;
  onopen: ((ev: unknown) => void) | null;
  onmessage: ((ev: { data: unknown }) => void) | null;
  onclose: ((ev: { code: number }) => void) | null;
  onerror: ((ev: unknown) => void) | null;
}

export interface LiveSocketOptions {
  /** e.g. `ws://localhost:8080/v1/ws/picker` */
  url: string;
  /** Current Firebase ID token; `forceRefresh` after the server said it expired. */
  getToken: (forceRefresh: boolean) => Promise<string | null>;
  onMessage: (msg: WsServerMessage) => void;
  onStatus?: (status: LiveStatus) => void;
  /** Injected for tests; defaults to the global WebSocket. */
  createSocket?: (url: string) => MinimalWebSocket;
  maxBackoffMs?: number;
  random?: () => number;
}

const CLOSE_FORBIDDEN = 4003;
const CLOSE_TOKEN_EXPIRED = 4008;

/**
 * A self-healing authenticated socket: sends `{type: "auth"}` first,
 * reconnects with jittered exponential backoff, refreshes the token when it
 * expires, and stops on FORBIDDEN (retrying can't fix a permission problem).
 * After every reconnect it emits `{type: "resync"}` because events may have
 * been missed while disconnected.
 */
export function openLiveSocket(opts: LiveSocketOptions): { close: () => void } {
  const create = opts.createSocket ?? ((url: string) => new WebSocket(url) as unknown as MinimalWebSocket);
  const maxBackoff = opts.maxBackoffMs ?? 30_000;
  const random = opts.random ?? Math.random;
  let socket: MinimalWebSocket | null = null;
  let attempt = 0;
  let stopped = false;
  let forceRefresh = false;
  let everReady = false;
  let timer: ReturnType<typeof setTimeout> | null = null;

  const status = (s: LiveStatus) => opts.onStatus?.(s);

  const connect = () => {
    if (stopped) return;
    status(attempt === 0 ? "connecting" : "reconnecting");
    const ws = create(opts.url);
    socket = ws;
    ws.onopen = () => {
      void opts.getToken(forceRefresh).then((token) => {
        forceRefresh = false;
        if (socket !== ws) return;
        if (!token) {
          stop();
          return;
        }
        ws.send(JSON.stringify({ type: "auth", token }));
      });
    };
    ws.onmessage = (ev) => {
      if (typeof ev.data !== "string") return;
      let msg: WsServerMessage;
      try {
        msg = JSON.parse(ev.data) as WsServerMessage;
      } catch {
        return;
      }
      if (msg.type === "ready") {
        attempt = 0;
        status("open");
        // Missed events while we were away: tell the app to refetch.
        if (everReady) opts.onMessage({ type: "resync" });
        everReady = true;
        return;
      }
      if (msg.type === "error" && msg.code === "FORBIDDEN") stopped = true;
      opts.onMessage(msg);
    };
    ws.onerror = () => {
      /* onclose follows */
    };
    ws.onclose = (ev) => {
      if (socket !== ws) return;
      socket = null;
      if (ev.code === CLOSE_FORBIDDEN) stopped = true;
      if (stopped) {
        status("stopped");
        return;
      }
      if (ev.code === CLOSE_TOKEN_EXPIRED || ev.code === 4001) forceRefresh = true;
      const base = Math.min(maxBackoff, 500 * 2 ** attempt);
      attempt += 1;
      status("reconnecting");
      timer = setTimeout(connect, base / 2 + random() * (base / 2));
    };
  };

  const stop = () => {
    stopped = true;
    if (timer) clearTimeout(timer);
    const ws = socket;
    socket = null;
    ws?.close(1000, "bye");
    status("stopped");
  };

  connect();
  return { close: stop };
}

/** `http://host:8080` -> `ws://host:8080` (and https -> wss). */
export function toWsUrl(apiBaseUrl: string, path: string): string {
  return `${apiBaseUrl.replace(/\/$/, "").replace(/^http/, "ws")}${path}`;
}
