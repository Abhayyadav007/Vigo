import { useQuery } from "@tanstack/react-query";
import type { HealthResponse } from "@vigo/types";
import { ApiError } from "../client";
import { useApiClient } from "../context";

export const healthKeys = { all: ["healthz"] as const };

/**
 * `GET /healthz`. The backend answers 503 with a HealthResponse body when a
 * dependency is down, so that case is returned as data rather than thrown.
 */
export function useHealth() {
  const client = useApiClient();
  return useQuery({
    queryKey: healthKeys.all,
    queryFn: async (): Promise<HealthResponse> => {
      const res = await client.get<HealthResponse>("/healthz", {
        validateStatus: (s) => s === 200 || s === 503,
      });
      return res.data;
    },
    refetchInterval: 10_000,
    retry: (count, err) => !(err instanceof ApiError) && count < 2,
  });
}
