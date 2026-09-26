import type { LatLng } from "@vigo/types";
import { colors } from "@vigo/ui";
import MapView, { Marker } from "react-native-maps";

/** Drop point plus the rider's live position (moves as `riderLocation` events arrive). */
export function TrackingMap({ drop, rider }: { drop: LatLng; rider: LatLng | null }) {
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
        <Marker coordinate={{ latitude: rider.lat, longitude: rider.lng }} title="Your rider" pinColor={colors.accent} />
      ) : null}
    </MapView>
  );
}
