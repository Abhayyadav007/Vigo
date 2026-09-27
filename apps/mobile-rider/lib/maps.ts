import { Linking } from "react-native";

/** Opens turn-by-turn directions in Google Maps (app or web). */
export function navigateTo(lat: number, lng: number) {
  void Linking.openURL(`https://www.google.com/maps/dir/?api=1&destination=${lat},${lng}&travelmode=driving`);
}

export function call(phone: string) {
  void Linking.openURL(`tel:${phone}`);
}

export function metres(m: number | null | undefined): string {
  if (m === null || m === undefined) return "—";
  return m < 1000 ? `${Math.round(m)} m` : `${(m / 1000).toFixed(1)} km`;
}
