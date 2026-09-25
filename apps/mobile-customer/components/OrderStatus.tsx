import type { OrderStatus } from "@vigo/types";
import { Text, View } from "react-native";

export const STATUS_LABEL: Record<OrderStatus, string> = {
  PLACED: "Placed",
  CONFIRMED: "Confirmed",
  PICKING: "Packing",
  PACKED: "Packed",
  RIDER_ASSIGNED: "Rider assigned",
  PICKED_UP: "Picked up",
  OUT_FOR_DELIVERY: "On the way",
  DELIVERED: "Delivered",
  CANCELLED: "Cancelled",
  PARTIALLY_FULFILLED: "Delivered (partial)",
};

const tone = (s: OrderStatus) =>
  s === "CANCELLED" ? "bg-danger" : s === "DELIVERED" || s === "PARTIALLY_FULFILLED" ? "bg-muted" : "bg-brand";

export function StatusChip({ status }: { status: OrderStatus }) {
  return (
    <View className={`rounded-pill px-3 py-1 ${tone(status)}`}>
      <Text className="text-xs font-bold text-white">{STATUS_LABEL[status]}</Text>
    </View>
  );
}
