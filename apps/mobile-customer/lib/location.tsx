import { useServiceability } from "@vigo/api-client";
import type { StoreSummary } from "@vigo/types";
import * as Location from "expo-location";
import { useQuery } from "@tanstack/react-query";
import { createContext, useCallback, useContext, useState, type ReactNode } from "react";

type Coords = { lat: number; lng: number };

type DeliveryState =
  | { status: "locating" }
  | { status: "denied" }
  | { status: "error"; message: string }
  | { status: "unserviceable"; coords: Coords }
  | { status: "ready"; coords: Coords; store: StoreSummary };

const DeliveryContext = createContext<{ state: DeliveryState; retry: () => void } | null>(null);

/** `EXPO_PUBLIC_DEV_LOCATION="12.9784,77.6408"` skips GPS (simulators, demos). */
function devLocation(): Coords | null {
  const raw = process.env.EXPO_PUBLIC_DEV_LOCATION;
  const [lat, lng] = raw ? raw.split(",").map(Number) : [];
  return lat !== undefined && lng !== undefined && Number.isFinite(lat) && Number.isFinite(lng) ? { lat, lng } : null;
}

/**
 * Finds where the customer is and which dark store serves them. Everything
 * catalog-related is scoped to that store.
 */
class LocationDenied extends Error {}

async function deviceLocation(): Promise<Coords> {
  const { status } = await Location.requestForegroundPermissionsAsync();
  if (status !== Location.PermissionStatus.GRANTED) throw new LocationDenied("location permission denied");
  const pos = await Location.getCurrentPositionAsync({ accuracy: Location.Accuracy.Balanced });
  return { lat: pos.coords.latitude, lng: pos.coords.longitude };
}

export function DeliveryProvider({ children }: { children: ReactNode }) {
  const [dev] = useState(devLocation);
  const location = useQuery({
    queryKey: ["device-location"],
    queryFn: deviceLocation,
    enabled: dev === null,
    retry: false,
    staleTime: 5 * 60_000,
  });
  const coords = dev ?? location.data ?? null;
  const serviceability = useServiceability(coords);

  let state: DeliveryState;
  if (location.error instanceof LocationDenied) state = { status: "denied" };
  else if (location.error) state = { status: "error", message: location.error.message };
  else if (!coords || serviceability.isPending) state = { status: "locating" };
  else if (serviceability.isError) state = { status: "error", message: "Couldn't reach Vigo. Check your connection." };
  else if (serviceability.data.store) state = { status: "ready", coords, store: serviceability.data.store };
  else state = { status: "unserviceable", coords };

  const { refetch: refetchLocation } = location;
  const { refetch: refetchServiceability } = serviceability;
  const retry = useCallback(() => {
    if (dev === null) void refetchLocation();
    void refetchServiceability();
  }, [dev, refetchLocation, refetchServiceability]);

  return <DeliveryContext.Provider value={{ state, retry }}>{children}</DeliveryContext.Provider>;
}

export function useDelivery() {
  const ctx = useContext(DeliveryContext);
  if (!ctx) throw new Error("useDelivery must be used inside <DeliveryProvider>");
  return ctx;
}

/** The serving store; only call below `<DeliveryGate>`. */
export function useStore(): StoreSummary {
  const { state } = useDelivery();
  if (state.status !== "ready") throw new Error("useStore called before a store was resolved");
  return state.store;
}
