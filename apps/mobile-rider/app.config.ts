import type { ConfigContext, ExpoConfig } from "expo/config";

// Static config lives in app.json; this adds the Firebase native config files,
// which are per-environment and gitignored (see firebase/README.md).
export default ({ config }: ConfigContext): ExpoConfig => ({
  ...(config as ExpoConfig),
  ios: {
    ...config.ios,
    googleServicesFile: process.env.GOOGLE_SERVICE_INFO_PLIST ?? "./firebase/GoogleService-Info.plist",
  },
  android: {
    ...config.android,
    googleServicesFile: process.env.GOOGLE_SERVICES_JSON ?? "./firebase/google-services.json",
  },
});
