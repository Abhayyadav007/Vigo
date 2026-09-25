import axios, { AxiosError, type AxiosInstance, type InternalAxiosRequestConfig } from "axios";
import type { ErrorBody } from "@vigo/types";

export interface ApiClientOptions {
  /** Backend origin, e.g. `http://localhost:8080`. */
  baseURL: string;
  timeoutMs?: number;
  /**
   * Returns the current Firebase ID token (or null when signed out).
   * `forceRefresh` is true once after a 401, to recover from an expired token.
   */
  getIdToken?: (forceRefresh: boolean) => Promise<string | null>;
}

/** Error thrown for every non-2xx response, carrying the backend's error code. */
export class ApiError extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    message: string,
  ) {
    super(message);
    this.name = "ApiError";
  }
}

function isErrorBody(data: unknown): data is ErrorBody {
  if (typeof data !== "object" || data === null || !("error" in data)) return false;
  const { error } = data;
  return typeof error === "object" && error !== null && "code" in error && "message" in error;
}

export function createApiClient({ baseURL, timeoutMs = 15_000, getIdToken }: ApiClientOptions): AxiosInstance {
  const instance = axios.create({ baseURL, timeout: timeoutMs });
  const retried = new WeakSet<InternalAxiosRequestConfig>();

  if (getIdToken) {
    instance.interceptors.request.use(async (config) => {
      const token = await getIdToken(false);
      if (token) config.headers.set("Authorization", `Bearer ${token}`);
      return config;
    });
  }

  instance.interceptors.response.use(undefined, async (err: unknown) => {
    if (!(err instanceof AxiosError) || !err.response) {
      return Promise.reject(err instanceof Error ? err : new Error(String(err)));
    }
    const status = err.response.status;
    const data: unknown = err.response.data;
    const config = err.config;

    // One forced token refresh per request, then give up.
    if (status === 401 && getIdToken && config && !retried.has(config)) {
      retried.add(config);
      const fresh = await getIdToken(true);
      if (fresh) {
        config.headers.set("Authorization", `Bearer ${fresh}`);
        return instance.request(config);
      }
    }

    if (isErrorBody(data)) {
      return Promise.reject(new ApiError(status, data.error.code, data.error.message));
    }
    return Promise.reject(new ApiError(status, "HTTP_ERROR", err.message));
  });

  return instance;
}
