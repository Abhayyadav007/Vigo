import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ApiClientProvider, createApiClient } from "@vigo/api-client";
import { Stack } from "expo-router";
import { StatusBar } from "expo-status-bar";
import { useState } from "react";

// TODO(phase-2): wrap in the Firebase AuthProvider and gate routes on role.
export default function RootLayout() {
  const [queryClient] = useState(() => new QueryClient());
  const [apiClient] = useState(() =>
    createApiClient({ baseURL: process.env.EXPO_PUBLIC_API_URL ?? "http://localhost:8080" }),
  );

  return (
    <ApiClientProvider client={apiClient}>
      <QueryClientProvider client={queryClient}>
        <StatusBar style="auto" />
        <Stack screenOptions={{ headerShown: false }} />
      </QueryClientProvider>
    </ApiClientProvider>
  );
}
