import { formatPaise, isOrderActive, useCancelOrder, useOrder } from "@vigo/api-client";
import { Button, colors, EmptyState } from "@vigo/ui";
import { Stack, useLocalSearchParams } from "expo-router";
import { ActivityIndicator, Alert, ScrollView, Text, View } from "react-native";
import { STATUS_LABEL, StatusChip } from "../../components/OrderStatus";

export default function OrderScreen() {
  const { id, placed } = useLocalSearchParams<{ id: string; placed?: string }>();
  const order = useOrder(id);
  const cancel = useCancelOrder();

  if (order.isPending) {
    return (
      <View className="flex-1 items-center justify-center bg-background">
        <ActivityIndicator color={colors.brand} />
      </View>
    );
  }
  if (!order.data) return <EmptyState title="Order not found" />;
  const o = order.data;
  const savings = o.bill.mrpTotalPaise - o.bill.itemTotalPaise;

  const confirmCancel = () =>
    Alert.alert("Cancel this order?", "Items go back to the store and you won't be charged.", [
      { text: "Keep order", style: "cancel" },
      {
        text: "Cancel order",
        style: "destructive",
        onPress: () => cancel.mutate({ id: o.id }, { onError: (e) => Alert.alert("Couldn't cancel", e.message) }),
      },
    ]);

  return (
    <View className="flex-1 bg-background">
      <Stack.Screen options={{ headerShown: true, title: o.number }} />
      <ScrollView contentContainerClassName="gap-4 p-4 pb-12">
        {placed && o.status !== "CANCELLED" ? (
          <View className="rounded-lg bg-brand p-4">
            <Text className="text-xl font-bold text-white">Order placed 🎉</Text>
            <Text className="text-white">We're getting it ready.</Text>
          </View>
        ) : null}

        <View className="flex-row items-center justify-between">
          <Text className="text-lg font-bold text-ink">Status</Text>
          <StatusChip status={o.status} />
        </View>

        {o.deliveryOtp ? (
          <View className="items-center gap-1 rounded-lg border border-brand bg-brand-light p-4">
            <Text className="text-sm text-brand-dark">Share this OTP with the rider at delivery</Text>
            <Text testID="delivery-otp" className="text-4xl font-bold tracking-widest text-brand-dark">
              {o.deliveryOtp}
            </Text>
          </View>
        ) : null}
        {o.cancelReason ? <Text className="text-sm text-danger">Cancelled: {o.cancelReason}</Text> : null}

        <View className="gap-2 rounded-lg border border-line p-4">
          {o.events.map((e, i) => (
            <View key={`${e.status}-${i}`} className="flex-row items-center gap-3">
              <View className={`h-2.5 w-2.5 rounded-pill ${e.status === "CANCELLED" ? "bg-danger" : "bg-brand"}`} />
              <Text className="flex-1 text-sm text-ink">{STATUS_LABEL[e.status]}</Text>
              <Text className="text-xs text-muted">
                {new Date(e.at).toLocaleTimeString("en-IN", { hour: "2-digit", minute: "2-digit" })}
              </Text>
            </View>
          ))}
          {/* TODO(phase-7): live map tracking once a rider is assigned. */}
        </View>

        <View className="gap-2 rounded-lg border border-line p-4">
          <Text className="text-base font-bold text-ink">
            {o.itemCount} {o.itemCount === 1 ? "item" : "items"}
          </Text>
          {o.items.map((i) => {
            // After packing, lines may be short: the customer pays for what was found.
            const qty = i.pickedQuantity ?? i.quantity;
            const missing = i.quantity - qty;
            return (
              <View key={i.productId} className="flex-row justify-between gap-3">
                <Text className="flex-1 text-sm text-ink" numberOfLines={2}>
                  {qty} × {i.name} <Text className="text-muted">({i.unitLabel})</Text>
                  {missing > 0 ? <Text className="text-warning">{`  ${missing} unavailable, not charged`}</Text> : null}
                </Text>
                <Text className="text-sm text-ink">{formatPaise(i.unitPricePaise * qty)}</Text>
              </View>
            );
          })}
          <View className="my-1 h-px bg-line" />
          <View className="flex-row justify-between">
            <Text className="text-sm text-muted">Delivery fee</Text>
            <Text className="text-sm text-ink">{o.bill.deliveryFeePaise ? formatPaise(o.bill.deliveryFeePaise) : "FREE"}</Text>
          </View>
          <View className="flex-row justify-between">
            <Text className="font-bold text-ink">Total ({o.paymentMethod === "COD" ? "pay on delivery" : o.paymentStatus.toLowerCase()})</Text>
            <Text className="font-bold text-ink">{formatPaise(o.bill.totalPaise)}</Text>
          </View>
          {savings > 0 ? <Text className="text-sm text-success">You saved {formatPaise(savings)}</Text> : null}
        </View>

        <View className="gap-1 rounded-lg border border-line p-4">
          <Text className="text-base font-bold text-ink">Delivering to {o.address.label}</Text>
          <Text className="text-sm text-muted">
            {[o.address.line1, o.address.line2, o.address.landmark, o.address.city, o.address.pincode]
              .filter(Boolean)
              .join(", ")}
          </Text>
        </View>

        {o.canCancel && isOrderActive(o.status) ? (
          <Button title="Cancel order" variant="secondary" onPress={confirmCancel} loading={cancel.isPending} />
        ) : null}
      </ScrollView>
    </View>
  );
}
