import axios, { AxiosError, type AxiosInstance } from "axios";
import type { ErrorBody } from "@vigo/types";

export interface ApiClientOptions {
  /** Backend origin, e.g. `http://localhost:8080`. */
  baseURL: string;
  timeoutMs?: number;
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
  const { error } = data as { error: unknown };
  return typeof error === "object" && error !== null && "code" in error && "message" in error;
}

export function createApiClient({ baseURL, timeoutMs = 15_000 }: ApiClientOptions): AxiosInstance {
  const instance = axios.create({ baseURL, timeout: timeoutMs });

  // TODO(phase-2): request interceptor attaching `Bearer <firebase_id_token>`,
  // and a single forced-refresh retry on 401.

  instance.interceptors.response.use(undefined, (err: unknown) => {
    if (err instanceof AxiosError && err.response) {
      const { status, data } = err.response;
      if (isErrorBody(data)) {
        return Promise.reject(new ApiError(status, data.error.code, data.error.message));
      }
      return Promise.reject(new ApiError(status, "HTTP_ERROR", err.message));
    }
    return Promise.reject(err);
  });

  return instance;
}
