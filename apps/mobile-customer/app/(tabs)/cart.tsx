import {
  ApiError,
  formatPaise,
  newIdempotencyKey,
  resolveMediaUrl,
  useAddresses,
  useApiBaseUrl,
  useCart,
  useCheckout,
} from "@vigo/api-client";
import type { PaymentMethod } from "@vigo/types";
import { Button, colors, EmptyState } from "@vigo/ui";
import { router } from "expo-router";
import { useRef, useState } from "react";
import { ActivityIndicator, Image, Pressable, ScrollView, Text, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { CartLineStepper } from "../../components/CartControls";
import { useStore } from "../../lib/location";

export default function Cart() {
  const store = useStore();
  const insets = useSafeAreaInsets();
  const base = useApiBaseUrl();
  const cart = useCart(store.id);
  const addresses = useAddresses();
  const checkout = useCheckout();
  const [addressId, setAddressId] = useState<string>();
  const [method] = useState<PaymentMethod>("COD");
  // One key per checkout attempt, so a retried tap can't place two orders.
  const attemptKey = useRef(newIdempotencyKey());

  if (cart.isPending) {
    return (
      <View className="flex-1 items-center justify-center bg-background">
        <ActivityIndicator color={colors.brand} />
      </View>
    );
  }
  const data = cart.data;
  if (!data || data.items.length === 0) {
    return (
      <EmptyState title="Your cart is empty" message={`Add items from ${store.name} to get started.`}>
        <Button title="Start shopping" onPress={() => router.navigate("/")} />
      </EmptyState>
    );
  }

  // Only addresses this store delivers to can be used for this cart.
  const usable = (addresses.data ?? []).filter((a) => a.servingStoreId === store.id);
  const selected = usable.find((a) => a.id === addressId) ?? usable[0];
  const bill = data.bill;
  const savings = bill.mrpTotalPaise - bill.itemTotalPaise;
  const toFree = bill.freeDeliveryAbovePaise - bill.itemTotalPaise;

  const placeOrder = () => {
    if (!selected) return;
    checkout.mutate(
      { body: { storeId: store.id, addressId: selected.id, paymentMethod: method }, idempotencyKey: attemptKey.current },
      {
        onSuccess: (res) => {
          attemptKey.current = newIdempotencyKey();
          router.push({ pathname: "/order/[id]", params: { id: res.order.id, placed: "1" } });
        },
        onError: (e) => {
          // A definite rejection (not a network failure) ends this attempt.
          if (e instanceof ApiError) attemptKey.current = newIdempotencyKey();
        },
      },
    );
  };

  return (
    <View className="flex-1 bg-background">
      <ScrollView contentContainerClassName="gap-4 px-4 pb-40" contentContainerStyle={{ paddingTop: insets.top + 12 }}>
        <Text className="text-2xl font-bold text-ink">Cart</Text>
        <Text className="text-sm text-muted">
          Delivery in {store.etaMinutes} minutes from {store.name}
        </Text>

        <View className="gap-3 rounded-lg border border-line p-3">
          {data.items.map((line) => (
            <View key={line.productId} className={`flex-row items-center gap-3 ${line.available ? "" : "opacity-50"}`}>
              <View className="h-14 w-14 items-center justify-center overflow-hidden rounded-md bg-surface">
                {line.imageUrl ? (
                  <Image source={{ uri: resolveMediaUrl(line.imageUrl, base) }} className="h-full w-full" resizeMode="contain" />
                ) : (
                  <Text>🛒</Text>
                )}
              </View>
              <View className="flex-1 gap-0.5">
                <Text className="text-sm font-medium text-ink" numberOfLines={2}>
                  {line.name}
                </Text>
                <Text className="text-xs text-muted">{line.unitLabel}</Text>
                {line.available ? (
                  <Text className="text-sm font-semibold text-ink">{formatPaise(line.lineTotalPaise)}</Text>
                ) : (
                  <Text className="text-xs font-semibold text-danger">
                    {line.maxQuantity === 0 ? "Unavailable — remove it" : `Only ${line.maxQuantity} left`}
                  </Text>
                )}
              </View>
              <View className="w-28">
                <CartLineStepper productId={line.productId} max={line.maxQuantity} />
              </View>
            </View>
          ))}
        </View>

        {toFree > 0 ? (
          <Text className="rounded-md bg-brand-light p-3 text-sm text-brand-dark">
            Add {formatPaise(toFree)} more for free delivery
          </Text>
        ) : null}

        <View className="gap-2 rounded-lg border border-line p-4">
          <Text className="mb-1 text-base font-bold text-ink">Bill details</Text>
          <Row label="Item total" value={formatPaise(bill.itemTotalPaise)} />
          <Row label="Delivery fee" value={bill.deliveryFeePaise ? formatPaise(bill.deliveryFeePaise) : "FREE"} />
          <View className="my-1 h-px bg-line" />
          <Row label="To pay" value={formatPaise(bill.totalPaise)} bold />
          {savings > 0 ? <Text className="text-sm text-success">You save {formatPaise(savings)} on MRP</Text> : null}
        </View>

        <View className="gap-2 rounded-lg border border-line p-4">
          <View className="flex-row items-center justify-between">
            <Text className="text-base font-bold text-ink">Deliver to</Text>
            <Pressable onPress={() => router.push("/address/new")}>
              <Text className="font-semibold text-brand">+ Add address</Text>
            </Pressable>
          </View>
          {usable.length === 0 ? (
            <Text className="text-sm text-muted">Add an address in {store.name}'s delivery area to continue.</Text>
          ) : (
            usable.map((a) => (
              <Pressable
                key={a.id}
                onPress={() => setAddressId(a.id)}
                className={`rounded-md border p-3 ${a.id === selected?.id ? "border-brand bg-brand-light" : "border-line"}`}
              >
                <Text className="font-semibold text-ink">{a.label}</Text>
                <Text className="text-sm text-muted" numberOfLines={2}>
                  {[a.line1, a.line2, a.landmark, a.city, a.pincode].filter(Boolean).join(", ")}
                </Text>
              </Pressable>
            ))
          )}
        </View>

        <View className="gap-2 rounded-lg border border-line p-4">
          <Text className="text-base font-bold text-ink">Payment</Text>
          <View className="rounded-md border border-brand bg-brand-light p-3">
            <Text className="font-semibold text-ink">Cash / UPI on delivery</Text>
            <Text className="text-sm text-muted">Pay the rider when your order arrives.</Text>
          </View>
          {/* TODO(prod): online payment via the Razorpay SDK (backend flow is ready). */}
          <Text className="text-xs text-muted">Pay online with UPI or card — coming soon.</Text>
        </View>
      </ScrollView>

      <View className="absolute bottom-0 left-0 right-0 gap-2 border-t border-line bg-background p-4">
        {checkout.error ? <Text className="text-sm text-danger">{checkout.error.message}</Text> : null}
        <Button
          testID="place-order"
          title={`Place order · ${formatPaise(bill.totalPaise)}`}
          onPress={placeOrder}
          loading={checkout.isPending}
          disabled={!data.canCheckout || !selected}
        />
      </View>
    </View>
  );
}

function Row({ label, value, bold }: { label: string; value: string; bold?: boolean }) {
  return (
    <View className="flex-row justify-between">
      <Text className={`text-sm ${bold ? "font-bold text-ink" : "text-muted"}`}>{label}</Text>
      <Text className={`text-sm ${bold ? "font-bold text-ink" : "text-ink"}`}>{value}</Text>
    </View>
  );
}
