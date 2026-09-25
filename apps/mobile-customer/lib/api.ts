import { createApiClient } from "@vigo/api-client";
import { createNativeFirebaseAuth } from "@vigo/api-client/native";

export const firebaseAuth = createNativeFirebaseAuth({
  emulatorHost: process.env.EXPO_PUBLIC_FIREBASE_AUTH_EMULATOR_HOST || undefined,
});

export const apiClient = createApiClient({
  baseURL: process.env.EXPO_PUBLIC_API_URL ?? "http://localhost:8080",
  getIdToken: firebaseAuth.adapter.getIdToken,
});
