import { useQueryClient } from "@tanstack/react-query";
import type { MeResponse, Role } from "@vigo/types";
import type { AxiosInstance } from "axios";
import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { ApiError } from "./client";

/** The slice of Firebase Auth the provider needs; implemented in `./firebase/{native,web}`. */
// Function-typed properties (not methods) so they can be passed around unbound,
// e.g. `getIdToken: adapter.getIdToken`.
export interface AuthAdapter {
  /** Subscribes to sign-in/sign-out; returns an unsubscribe function. */
  onAuthStateChanged: (listener: (user: { uid: string } | null) => void) => () => void;
  getIdToken: (forceRefresh?: boolean) => Promise<string | null>;
  signOut: () => Promise<void>;
}

export type AuthState =
  | { status: "loading" }
  | { status: "signedOut"; error?: string }
  | { status: "signedIn"; user: MeResponse };

export interface AuthContextValue {
  state: AuthState;
  signOut: () => Promise<void>;
  /** Re-reads the user from the backend (e.g. after a role change). */
  refresh: () => Promise<void>;
}

const AuthContext = createContext<AuthContextValue | null>(null);

const ROLE_LABEL: Record<Role, string> = {
  CUSTOMER: "customer",
  PICKER: "picker",
  RIDER: "rider",
  ADMIN: "admin",
};

export interface AuthProviderProps {
  adapter: AuthAdapter;
  client: AxiosInstance;
  /** The only role allowed to use this app; other roles are signed out. */
  requiredRole: Role;
  children: ReactNode;
}

/**
 * Unified auth state for every client: listens to Firebase, calls
 * `POST /v1/auth/sync` after sign-in, and enforces the app's role.
 */
export function AuthProvider({ adapter, client, requiredRole, children }: AuthProviderProps) {
  const queryClient = useQueryClient();
  const [state, setState] = useState<AuthState>({ status: "loading" });
  // Error to show once the forced sign-out below comes back through the listener.
  const pendingError = useRef<string | undefined>(undefined);
  // Ignores results from a sync that was overtaken by a newer auth event.
  const generation = useRef(0);

  const sync = useCallback(async () => {
    const gen = ++generation.current;
    setState({ status: "loading" });
    try {
      const { data } = await client.post<MeResponse>("/v1/auth/sync");
      if (gen !== generation.current) return;
      if (data.role !== requiredRole) {
        pendingError.current = `This app is for ${ROLE_LABEL[requiredRole]} accounts. Your account is a ${ROLE_LABEL[data.role]} account.`;
        await adapter.signOut();
        return;
      }
      setState({ status: "signedIn", user: data });
    } catch (err) {
      if (gen !== generation.current) return;
      pendingError.current =
        err instanceof ApiError ? err.message : "Couldn't reach Vigo. Check your connection and try again.";
      await adapter.signOut();
    }
  }, [adapter, client, requiredRole]);

  useEffect(
    () =>
      adapter.onAuthStateChanged((user) => {
        if (user) {
          void sync();
          return;
        }
        generation.current++;
        queryClient.clear();
        const error = pendingError.current;
        pendingError.current = undefined;
        setState(error ? { status: "signedOut", error } : { status: "signedOut" });
      }),
    [adapter, queryClient, sync],
  );

  const value = useMemo<AuthContextValue>(
    () => ({ state, signOut: () => adapter.signOut(), refresh: sync }),
    [adapter, state, sync],
  );
  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error("useAuth must be used inside <AuthProvider>");
  return ctx;
}

/** The signed-in user; only call below a route that requires sign-in. */
export function useCurrentUser(): MeResponse {
  const { state } = useAuth();
  if (state.status !== "signedIn") throw new Error("useCurrentUser called while signed out");
  return state.user;
}
