import {
  ApiError,
  formatPaise,
  newIdempotencyKey,
  resolveMediaUrl,
  useAddresses,
  useApiBaseUrl,
  useCart,
  useCheckout,
  useCurrentUser,
  useVerifyPayment,
} from "@vigo/api-client";
import type { PaymentMethod, RazorpayCheckout } from "@vigo/types";
import Ionicons from "@expo/vector-icons/Ionicons";
import { Button, EmptyState } from "@vigo/ui";
import { Stack, router } from "expo-router";
import { useRef, useState, type ComponentProps } from "react";
import { ActivityIndicator, Image, Pressable, ScrollView, Text, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { CartLineStepper } from "../components/CartControls";
import { DeliveryGate } from "../components/DeliveryGate";
import { useStore } from "../lib/location";
import { payWithRazorpay } from "../lib/razorpay";
import { colors } from "../lib/theme";

/** An order placed online whose payment hasn't been confirmed yet. */
type AwaitingPayment = {
  orderId: string;
  razorpay: RazorpayCheckout;
  error?: string;
};

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
  const verify = useVerifyPayment();
  const { phone } = useCurrentUser();
  const [addressId, setAddressId] = useState<string>();
  const [chosenMethod, setMethod] = useState<PaymentMethod>();
  const [awaiting, setAwaiting] = useState<AwaitingPayment>();
  const [paying, setPaying] = useState(false);
  // One key per checkout attempt, so a retried tap can't place two orders.
  const attemptKey = useRef(newIdempotencyKey());

  const showOrder = (id: string) =>
    // Replace, so Back from the order goes home rather than to an empty cart.
    router.replace({ pathname: "/order/[id]", params: { id, placed: "1" } });

  /** Opens Razorpay for a placed order and confirms it with the signed result. */
  const pay = async (orderId: string, razorpay: RazorpayCheckout) => {
    setPaying(true);
    const outcome = await payWithRazorpay(razorpay, phone);
    if (!outcome.ok) {
      setPaying(false);
      setAwaiting({ orderId, razorpay, error: outcome.message });
      return;
    }
    const { razorpayOrderId, razorpayPaymentId, razorpaySignature } = outcome;
    verify.mutate(
      {
        id: orderId,
        body: { razorpayOrderId, razorpayPaymentId, razorpaySignature },
      },
      {
        onSuccess: () => showOrder(orderId),
        // The payment went through; Razorpay's webhook confirms the order even if this call fails.
        onError: () => showOrder(orderId),
        onSettled: () => setPaying(false),
      },
    );
  };

  // The cart is already emptied into the order, so this takes over the screen.
  if (awaiting) {
    return (
      <PaymentPending
        error={awaiting.error}
        busy={paying}
        onRetry={() => void pay(awaiting.orderId, awaiting.razorpay)}
        onViewOrder={() => showOrder(awaiting.orderId)}
      />
    );
  }

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
  const methods = data.paymentMethods;
  const method = chosenMethod && methods.includes(chosenMethod) ? chosenMethod : methods[0];

  const placeOrder = () => {
    if (!selected || !method) return;
    checkout.mutate(
      {
        body: {
          storeId: store.id,
          addressId: selected.id,
          paymentMethod: method,
        },
        idempotencyKey: attemptKey.current,
      },
      {
        onSuccess: (res) => {
          attemptKey.current = newIdempotencyKey();
          if (res.razorpay) void pay(res.order.id, res.razorpay);
          else showOrder(res.order.id);
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
                  <Image
                    source={{ uri: resolveMediaUrl(line.imageUrl, base) }}
                    className="h-full w-full"
                    resizeMode="contain"
                  />
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
          {methods.map((m) => (
            <PaymentOption key={m} method={m} selected={m === method} onPress={() => setMethod(m)} />
          ))}
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
          title={`${method === "ONLINE" ? "Pay" : "Place order"} · ${formatPaise(bill.totalPaise)}`}
          onPress={placeOrder}
          loading={checkout.isPending || paying}
          disabled={!data.canCheckout || !selected || !method}
        />
      </View>
    </View>
  );
}

type IconName = ComponentProps<typeof Ionicons>["name"];

const METHOD_COPY: Record<PaymentMethod, { icon: IconName; title: string; note: string }> = {
  ONLINE: {
    icon: "card-outline",
    title: "Pay online",
    note: "UPI, cards, netbanking or wallets via Razorpay.",
  },
  COD: {
    icon: "cash-outline",
    title: "Cash / UPI on delivery",
    note: "Pay the rider when your order arrives.",
  },
};

function PaymentOption({
  method,
  selected,
  onPress,
}: {
  method: PaymentMethod;
  selected: boolean;
  onPress: () => void;
}) {
  const copy = METHOD_COPY[method];
  return (
    <Pressable
      testID={`pay-${method}`}
      onPress={onPress}
      className={`flex-row items-center gap-3 rounded-md border p-3 ${selected ? "border-brand bg-brand-light" : "border-line"}`}
    >
      <Ionicons name={copy.icon} size={20} color={selected ? colors.brand : colors.muted} />
      <View className="flex-1">
        <Text className="font-semibold text-ink">{copy.title}</Text>
        <Text className="text-sm text-muted">{copy.note}</Text>
      </View>
      <Ionicons
        name={selected ? "radio-button-on" : "radio-button-off"}
        size={18}
        color={selected ? colors.brand : colors.muted}
      />
    </Pressable>
  );
}

/** Shown when Razorpay was closed or failed: the order waits, unpaid, until it expires. */
function PaymentPending({
  error,
  busy,
  onRetry,
  onViewOrder,
}: {
  error?: string;
  busy: boolean;
  onRetry: () => void;
  onViewOrder: () => void;
}) {
  return (
    <View className="flex-1 items-center justify-center gap-4 bg-background p-6">
      <View className="h-16 w-16 items-center justify-center rounded-full bg-brand-light">
        <Ionicons name="card-outline" size={30} color={colors.brand} />
      </View>
      <Text className="text-center text-lg font-bold text-ink">Payment not completed</Text>
      <Text className="text-center text-sm text-muted">
        {error ?? "Your payment didn't go through."} Your order is saved for a few minutes; pay now to confirm it.
      </Text>
      <View className="w-full gap-2">
        <Button testID="retry-payment" title="Retry payment" onPress={onRetry} loading={busy} />
        <Button title="View order" variant="secondary" onPress={onViewOrder} disabled={busy} />
      </View>
    </View>
  );
}

function Row({ label, value, bold, icon }: { label: string; value: string; bold?: boolean; icon?: IconName }) {
  return (
    <View className="flex-row items-center gap-2">
      {icon ? <Ionicons name={icon} size={16} color={colors.muted} /> : null}
      <Text className={`flex-1 text-sm ${bold ? "font-bold text-ink" : "text-muted"}`}>{label}</Text>
      <Text className={`text-sm ${bold ? "font-bold text-ink" : "text-ink"}`}>{value}</Text>
    </View>
  );
}
