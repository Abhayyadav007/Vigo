import "../global.css";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ApiClientProvider, AuthProvider, useAuth } from "@vigo/api-client";
import { colors } from "@vigo/ui";
import { Stack } from "expo-router";
import { StatusBar } from "expo-status-bar";
import { useState } from "react";
import { ActivityIndicator, View } from "react-native";
import { apiClient, firebaseAuth } from "../lib/api";

export default function RootLayout() {
  const [queryClient] = useState(() => new QueryClient());
  return (
    <ApiClientProvider client={apiClient}>
      <QueryClientProvider client={queryClient}>
        <AuthProvider adapter={firebaseAuth.adapter} client={apiClient} requiredRole="PICKER">
          <StatusBar style="auto" />
          <RootStack />
        </AuthProvider>
      </QueryClientProvider>
    </ApiClientProvider>
  );
}

function RootStack() {
  const { state } = useAuth();
  if (state.status === "loading") {
    return (
      <View className="flex-1 items-center justify-center bg-background">
        <ActivityIndicator size="large" color={colors.brand} />
      </View>
    );
  }
  const signedIn = state.status === "signedIn";
  return (
    <Stack screenOptions={{ headerShown: false }}>
      <Stack.Protected guard={signedIn}>
        <Stack.Screen name="index" />
      </Stack.Protected>
      <Stack.Protected guard={!signedIn}>
        <Stack.Screen name="login" />
      </Stack.Protected>
    </Stack>
  );
}
