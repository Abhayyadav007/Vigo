// Background GPS for riders. The task is defined at module load (imported by
// the root layout) so the OS can wake the app with new fixes even when it's
// in the background; each batch is uploaded to the backend.
import { postRiderLocation } from "@vigo/api-client";
import * as Location from "expo-location";
import * as TaskManager from "expo-task-manager";
import { apiClient } from "./api";

const LOCATION_TASK = "vigo-rider-location";

export type TrackingMode = "idle" | "delivery";

TaskManager.defineTask<{ locations: Location.LocationObject[] }>(LOCATION_TASK, async ({ data, error }) => {
  if (error || !data) return;
  const points = data.locations.map((l) => ({
    lat: l.coords.latitude,
    lng: l.coords.longitude,
    ...(l.coords.accuracy !== null ? { accuracyM: l.coords.accuracy } : {}),
    recordedAt: l.timestamp,
  }));
  try {
    await postRiderLocation(apiClient, points);
  } catch {
    // Dropped fixes are fine: the next batch supersedes them.
  }
});

/**
 * Battery-aware settings: while waiting for work, coarse fixes every ~30 s
 * or 50 m; on a delivery, precise fixes every ~5 s or 10 m (the customer
 * watches the map).
 */
const OPTIONS: Record<TrackingMode, Location.LocationTaskOptions> = {
  idle: {
    accuracy: Location.Accuracy.Balanced,
    timeInterval: 30_000,
    distanceInterval: 50,
    deferredUpdatesInterval: 30_000,
  },
  delivery: {
    accuracy: Location.Accuracy.High,
    timeInterval: 5_000,
    distanceInterval: 10,
  },
};

export async function requestPermissions(): Promise<boolean> {
  const fg = await Location.requestForegroundPermissionsAsync();
  if (fg.status !== Location.PermissionStatus.GRANTED) return false;
  const bg = await Location.requestBackgroundPermissionsAsync();
  return bg.status === Location.PermissionStatus.GRANTED;
}

/** Starts (or re-tunes) background updates, and sends one fix right away. */
export async function startTracking(mode: TrackingMode): Promise<void> {
  if (await Location.hasStartedLocationUpdatesAsync(LOCATION_TASK)) {
    await Location.stopLocationUpdatesAsync(LOCATION_TASK);
  }
  await Location.startLocationUpdatesAsync(LOCATION_TASK, {
    ...OPTIONS[mode],
    pausesUpdatesAutomatically: false,
    activityType: Location.ActivityType.OtherNavigation,
    showsBackgroundLocationIndicator: true,
    foregroundService: {
      notificationTitle: "Vigo Rider is online",
      notificationBody: mode === "delivery" ? "Sharing your location with the customer" : "Waiting for deliveries nearby",
      notificationColor: "#0C8346",
    },
  });
  // Dispatch needs a fresh position now, not in 30 s.
  const now = await Location.getCurrentPositionAsync({ accuracy: Location.Accuracy.Balanced });
  await postRiderLocation(apiClient, [
    { lat: now.coords.latitude, lng: now.coords.longitude, recordedAt: now.timestamp },
  ]);
}

export async function stopTracking(): Promise<void> {
  if (await Location.hasStartedLocationUpdatesAsync(LOCATION_TASK)) {
    await Location.stopLocationUpdatesAsync(LOCATION_TASK);
  }
}
