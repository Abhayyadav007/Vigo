import { createApiClient } from "@vigo/api-client";
import { createWebFirebaseAuth } from "@vigo/api-client/web";

const env = import.meta.env;

export const firebaseAuth = createWebFirebaseAuth(
  {
    apiKey: env.VITE_FIREBASE_API_KEY,
    authDomain: env.VITE_FIREBASE_AUTH_DOMAIN ?? `${env.VITE_FIREBASE_PROJECT_ID}.firebaseapp.com`,
    projectId: env.VITE_FIREBASE_PROJECT_ID,
    ...(env.VITE_FIREBASE_APP_ID ? { appId: env.VITE_FIREBASE_APP_ID } : {}),
  },
  env.VITE_FIREBASE_AUTH_EMULATOR_HOST ? { emulatorHost: env.VITE_FIREBASE_AUTH_EMULATOR_HOST } : {},
);

export const apiClient = createApiClient({
  baseURL: env.VITE_API_URL ?? "http://localhost:8080",
  getIdToken: firebaseAuth.adapter.getIdToken,
});
