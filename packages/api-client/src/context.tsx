import { createContext, useContext, type ReactNode } from "react";
import type { AxiosInstance } from "axios";

const ApiClientContext = createContext<AxiosInstance | null>(null);

export function ApiClientProvider({ client, children }: { client: AxiosInstance; children: ReactNode }) {
  return <ApiClientContext.Provider value={client}>{children}</ApiClientContext.Provider>;
}

export function useApiClient(): AxiosInstance {
  const client = useContext(ApiClientContext);
  if (!client) throw new Error("useApiClient must be used inside <ApiClientProvider>");
  return client;
}
