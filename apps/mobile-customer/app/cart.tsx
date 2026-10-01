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
import Ionicons from "@expo/vector-icons/Ionicons";
import { Button, EmptyState } from "@vigo/ui";
import { Stack, router } from "expo-router";
import { useRef, useState, type ComponentProps } from "react";
import { ActivityIndicator, Image, Pressable, ScrollView, Text, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { CartLineStepper } from "../components/CartControls";
import { DeliveryGate } from "../components/DeliveryGate";
import { useStore } from "../lib/location";
import { colors } from "../lib/theme";

export default function CartScreen() {
  return (
    <DeliveryGate>
      <Stack.Screen options={{ headerShown: true, title: "Checkout" }} />
      <Cart />
    </DeliveryGate>
  );
}

function Cart() {
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
          // Replace, so Back from the order goes home rather than to an empty cart.
          router.replace({ pathname: "/order/[id]", params: { id: res.order.id, placed: "1" } });
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
      <ScrollView contentContainerClassName="gap-3 p-4" contentContainerStyle={{ paddingBottom: insets.bottom + 180 }}>
        <View className="gap-3 rounded-lg bg-surface p-4">
          <View className="flex-row items-center gap-3">
            <View className="h-11 w-11 items-center justify-center rounded-md bg-brand-light">
              <Ionicons name="timer-outline" size={22} color={colors.brand} />
            </View>
            <View className="flex-1">
              <Text className="text-base font-bold text-ink">Delivery in {store.etaMinutes} minutes</Text>
              <Text className="text-sm text-muted">
                Shipment of {data.itemCount} {data.itemCount === 1 ? "item" : "items"} from {store.name}
              </Text>
            </View>
          </View>
          <View className="h-px bg-line" />
          {data.items.map((line) => (
            <View key={line.productId} className={`flex-row items-center gap-3 ${line.available ? "" : "opacity-50"}`}>
              <View className="h-16 w-16 items-center justify-center overflow-hidden rounded-md border border-line bg-photo">
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
                  <Text className="text-sm font-bold text-ink">{formatPaise(line.lineTotalPaise)}</Text>
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
          <View className="flex-row items-center gap-2 rounded-lg bg-brand-light p-3">
            <Ionicons name="bicycle" size={18} color={colors.brand} />
            <Text className="flex-1 text-sm font-semibold text-brand-dark">
              Add {formatPaise(toFree)} more for free delivery
            </Text>
          </View>
        ) : null}

        <View className="gap-2 rounded-lg bg-surface p-4">
          <Text className="mb-1 text-base font-bold text-ink">Bill details</Text>
          <Row icon="document-text-outline" label="Items total" value={formatPaise(bill.itemTotalPaise)} />
          <Row
            icon="bicycle-outline"
            label="Delivery charge"
            value={bill.deliveryFeePaise ? formatPaise(bill.deliveryFeePaise) : "FREE"}
          />
          <View className="my-1 h-px bg-line" />
          <Row label="Grand total" value={formatPaise(bill.totalPaise)} bold />
          {savings > 0 ? (
            <View className="mt-1 rounded-md bg-background p-2">
              <Text className="text-center text-sm font-semibold text-success">
                You save {formatPaise(savings)} on this order
              </Text>
            </View>
          ) : null}
        </View>

        <View className="gap-2 rounded-lg bg-surface p-4">
          <View className="flex-row items-center justify-between">
            <Text className="text-base font-bold text-ink">Deliver to</Text>
            <Pressable onPress={() => router.push("/address/new")} hitSlop={8}>
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
                className={`flex-row items-center gap-3 rounded-md border p-3 ${a.id === selected?.id ? "border-brand bg-brand-light" : "border-line"}`}
              >
                <Ionicons
                  name={a.id === selected?.id ? "radio-button-on" : "radio-button-off"}
                  size={18}
                  color={a.id === selected?.id ? colors.brand : colors.muted}
                />
                <View className="flex-1">
                  <Text className="font-semibold text-ink">{a.label}</Text>
                  <Text className="text-sm text-muted" numberOfLines={2}>
                    {[a.line1, a.line2, a.landmark, a.city, a.pincode].filter(Boolean).join(", ")}
                  </Text>
                </View>
              </Pressable>
            ))
          )}
        </View>

        <View className="gap-2 rounded-lg bg-surface p-4">
          <Text className="text-base font-bold text-ink">Payment</Text>
          <View className="flex-row items-center gap-3 rounded-md border border-brand bg-brand-light p-3">
            <Ionicons name="cash-outline" size={20} color={colors.brand} />
            <View className="flex-1">
              <Text className="font-semibold text-ink">Cash / UPI on delivery</Text>
              <Text className="text-sm text-muted">Pay the rider when your order arrives.</Text>
            </View>
          </View>
          {/* TODO(prod): online payment via the Razorpay SDK (backend flow is ready). */}
          <Text className="text-xs text-muted">Pay online with UPI or card — coming soon.</Text>
        </View>

        <View className="gap-1 rounded-lg bg-surface p-4">
          <Text className="text-sm font-bold text-ink">Cancellation policy</Text>
          <Text className="text-xs leading-5 text-muted">
            You can cancel until the store starts packing your order. After that it's on its way to you.
          </Text>
        </View>
      </ScrollView>

      <View
        className="absolute bottom-0 left-0 right-0 gap-2 border-t border-line bg-surface px-4 pt-3"
        style={{ paddingBottom: insets.bottom + 12 }}
      >
        {selected ? (
          <View className="flex-row items-center gap-2">
            <Ionicons name="home-outline" size={16} color={colors.ink} />
            <Text className="flex-1 text-sm text-muted" numberOfLines={1}>
              <Text className="font-bold text-ink">Delivering to {selected.label}</Text>
              {` · ${[selected.line1, selected.city].filter(Boolean).join(", ")}`}
            </Text>
          </View>
        ) : null}
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

type IconName = ComponentProps<typeof Ionicons>["name"];

function Row({ label, value, bold, icon }: { label: string; value: string; bold?: boolean; icon?: IconName }) {
  return (
    <View className="flex-row items-center gap-2">
      {icon ? <Ionicons name={icon} size={16} color={colors.muted} /> : null}
      <Text className={`flex-1 text-sm ${bold ? "font-bold text-ink" : "text-muted"}`}>{label}</Text>
      <Text className={`text-sm ${bold ? "font-bold text-ink" : "text-ink"}`}>{value}</Text>
    </View>
  );
}
