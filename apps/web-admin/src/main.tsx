import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ApiClientProvider, createApiClient } from "@vigo/api-client";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./index.css";

const apiClient = createApiClient({ baseURL: import.meta.env.VITE_API_URL ?? "http://localhost:8080" });
const queryClient = new QueryClient();

const root = document.getElementById("root");
if (!root) throw new Error("#root element missing");

createRoot(root).render(
  <StrictMode>
    <ApiClientProvider client={apiClient}>
      <QueryClientProvider client={queryClient}>
        <App />
      </QueryClientProvider>
    </ApiClientProvider>
  </StrictMode>,
);
