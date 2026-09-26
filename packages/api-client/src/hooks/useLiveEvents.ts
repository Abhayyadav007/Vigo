import type { WsServerMessage } from "@vigo/types";
import { useEffect, useRef, useState } from "react";
import { useAuth } from "../auth-context";
import { useApiBaseUrl } from "../context";
import { openLiveSocket, toWsUrl, type LiveStatus } from "../ws";

/**
 * Keeps a live socket to `path` (e.g. `/v1/ws/picker`) open while signed in,
 * calling `onMessage` for every server message. `null` stays disconnected.
 * Returns the connection status.
 */
export function useLiveEvents(path: string | null, onMessage: (msg: WsServerMessage) => void): LiveStatus {
  const { state, getIdToken } = useAuth();
  const base = useApiBaseUrl();
  const [status, setStatus] = useState<LiveStatus>("connecting");
  const handler = useRef(onMessage);
  useEffect(() => {
    handler.current = onMessage;
  }, [onMessage]);

  const signedIn = state.status === "signedIn";
  useEffect(() => {
    if (!signedIn || !path) return;
    const socket = openLiveSocket({
      url: toWsUrl(base, path),
      getToken: (force) => getIdToken(force),
      onMessage: (msg) => handler.current(msg),
      onStatus: setStatus,
    });
    return () => socket.close();
  }, [base, path, signedIn, getIdToken]);

  return status;
}
