import type { LatLng } from "@vigo/types";
import { Text, View } from "react-native";
import MapView, { Marker } from "react-native-maps";
import { mapsEnabled } from "../lib/maps";
import { colors } from "../lib/theme";

/** Straight-line distance in km (haversine). */
function km(a: LatLng, b: LatLng) {
  const rad = (d: number) => (d * Math.PI) / 180;
  const h =
    Math.sin(rad(b.lat - a.lat) / 2) ** 2 +
    Math.cos(rad(a.lat)) * Math.cos(rad(b.lat)) * Math.sin(rad(b.lng - a.lng) / 2) ** 2;
  return 12742 * Math.asin(Math.sqrt(h));
}

/** Drop point plus the rider's live position (moves as `riderLocation` events arrive). */
export function TrackingMap({ drop, rider }: { drop: LatLng; rider: LatLng | null }) {
  if (!mapsEnabled) {
    return (
      <View className="rounded-lg bg-surface p-4">
        <Text className="text-sm text-ink">
          {rider ? `Your rider is ${km(drop, rider).toFixed(1)} km away.` : "Waiting for your rider's location…"}
        </Text>
      </View>
    );
  }
  const lat = rider ? (drop.lat + rider.lat) / 2 : drop.lat;
  const lng = rider ? (drop.lng + rider.lng) / 2 : drop.lng;
  const span = rider ? Math.max(Math.abs(drop.lat - rider.lat), Math.abs(drop.lng - rider.lng)) * 2.5 : 0;
  const delta = Math.max(span, 0.01);
  return (
    <MapView
      style={{ height: 240, borderRadius: 12 }}
      initialRegion={{ latitude: lat, longitude: lng, latitudeDelta: delta, longitudeDelta: delta }}
      pitchEnabled={false}
      toolbarEnabled={false}
    >
      <Marker coordinate={{ latitude: drop.lat, longitude: drop.lng }} title="You" pinColor={colors.brand} />
      {rider ? (
        <Marker
          coordinate={{ latitude: rider.lat, longitude: rider.lng }}
          title="Your rider"
          pinColor={colors.accent}
        />
      ) : null}
    </MapView>
  );
}
