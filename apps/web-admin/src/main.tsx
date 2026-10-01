import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ApiClientProvider, AuthProvider } from "@vigo/api-client";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./index.css";
import { apiClient, firebaseAuth } from "./lib/api";
import { BACK_OFFICE_ROLES } from "./lib/roles";

const queryClient = new QueryClient();

const root = document.getElementById("root");
if (!root) throw new Error("#root element missing");

createRoot(root).render(
  <StrictMode>
    <ApiClientProvider client={apiClient}>
      <QueryClientProvider client={queryClient}>
        <AuthProvider adapter={firebaseAuth.adapter} client={apiClient} requiredRole={BACK_OFFICE_ROLES}>
          <App />
        </AuthProvider>
      </QueryClientProvider>
    </ApiClientProvider>
  </StrictMode>,
);
