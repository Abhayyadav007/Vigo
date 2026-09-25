import "../global.css";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ApiClientProvider, AuthProvider, useAuth } from "@vigo/api-client";
import { colors } from "@vigo/ui";
import { Stack } from "expo-router";
import { StatusBar } from "expo-status-bar";
import { useState } from "react";
import { ActivityIndicator, View } from "react-native";
import { apiClient, firebaseAuth } from "../lib/api";
import { DeliveryProvider } from "../lib/location";

export default function RootLayout() {
  const [queryClient] = useState(() => new QueryClient());
  return (
    <ApiClientProvider client={apiClient}>
      <QueryClientProvider client={queryClient}>
        <AuthProvider adapter={firebaseAuth.adapter} client={apiClient} requiredRole="CUSTOMER">
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
  const stack = (
    <Stack screenOptions={{ headerShown: false, headerTintColor: colors.brand, headerBackButtonDisplayMode: "minimal" }}>
      <Stack.Protected guard={signedIn}>
        <Stack.Screen name="(tabs)" />
        <Stack.Screen name="category/[id]" />
        <Stack.Screen name="product/[id]" />
        <Stack.Screen name="order/[id]" />
        <Stack.Screen name="address/new" options={{ presentation: "modal" }} />
      </Stack.Protected>
      <Stack.Protected guard={!signedIn}>
        <Stack.Screen name="login" />
      </Stack.Protected>
    </Stack>
  );
  // Location/serviceability is only resolved once signed in.
  return signedIn ? <DeliveryProvider>{stack}</DeliveryProvider> : stack;
}
