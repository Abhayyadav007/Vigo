import Constants from "expo-constants";
import { Platform } from "react-native";

/**
 * Whether native maps can render. iOS uses Apple Maps with no key; Android's
 * Google Maps crashes without GOOGLE_MAPS_ANDROID_API_KEY, so those builds
 * fall back to map-free screens.
 */
export const mapsEnabled = Platform.OS !== "android" || Constants.expoConfig?.extra?.androidMapsEnabled === true;
