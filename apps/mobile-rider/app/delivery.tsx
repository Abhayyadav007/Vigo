import {
  ApiError,
  formatIndianPhone,
  formatPaise,
  useActiveDelivery,
  useDeliver,
  useDepart,
  usePickup,
  useUnassign,
} from "@vigo/api-client";
import { Button, colors, EmptyState, Screen, TextField } from "@vigo/ui";
import { useKeepAwake } from "expo-keep-awake";
import { Stack, router } from "expo-router";
import { useState } from "react";
import { ActivityIndicator, Alert, Pressable, Text, View } from "react-native";
import { call, navigateTo } from "../lib/maps";
import { startTracking } from "../lib/tracking";

export default function DeliveryScreen() {
  useKeepAwake();
  const active = useActiveDelivery();
  const pickup = usePickup();
  const depart = useDepart();
  const deliver = useDeliver();
  const unassign = useUnassign();
  const [bags, setBags] = useState<number | null>(null);
  const [otp, setOtp] = useState("");
  const [cashTaken, setCashTaken] = useState(false);
  const [error, setError] = useState<string>();

  if (active.isPending) {
    return (
      <View className="flex-1 items-center justify-center">
        <ActivityIndicator size="large" color={colors.brand} />
      </View>
    );
  }
  const d = active.data;
  if (!d) {
    return (
      <EmptyState title="No active delivery" message="Accepted orders show up here.">
        <Button title="Back" size="lg" onPress={() => router.replace("/")} />
      </EmptyState>
    );
  }

  const fail = (e: Error) => setError(e instanceof ApiError ? e.message : "Something went wrong");
  const atStore = d.status === "RIDER_ASSIGNED";
  const bagCount = bags ?? d.bagCount ?? 1;
  const cod = d.collectPaise > 0;

  return (
    <Screen scroll>
      <Stack.Screen options={{ headerShown: true, title: d.number }} />

      {atStore ? (
        <>
          <Text className="text-sm font-semibold text-muted">STEP 1 · PICK UP</Text>
          <View className="gap-1">
            <Text className="text-2xl font-bold text-ink">{d.storeName}</Text>
            <Text className="text-base text-muted">{d.storeAddress}</Text>
          </View>
          <Button title="Navigate to store" variant="secondary" size="lg" onPress={() => navigateTo(d.storeLocation.lat, d.storeLocation.lng)} />
          <View className="items-center gap-1 rounded-lg bg-ink p-5">
            <Text className="text-sm text-white">COLLECT FROM SLOT</Text>
            <Text className="text-4xl font-bold text-white">{d.stagingSlot ?? "Ask the picker"}</Text>
            <Text className="text-base text-white">
              {d.bagCount ?? "?"} bag(s) · {d.itemCount} items
            </Text>
          </View>
          <View className="flex-row items-center justify-between">
            <Text className="text-lg text-ink">Bags collected</Text>
            <View className="flex-row items-center gap-3">
              <Button title="−" variant="secondary" size="lg" onPress={() => setBags(Math.max(1, bagCount - 1))} />
              <Text className="w-10 text-center text-2xl font-bold text-ink">{bagCount}</Text>
              <Button title="+" variant="secondary" size="lg" onPress={() => setBags(bagCount + 1)} />
            </View>
          </View>
          {error ? <Text className="text-base text-danger">{error}</Text> : null}
          <Button
            testID="confirm-pickup"
            title="Confirm pickup"
            size="lg"
            loading={pickup.isPending}
            onPress={() => {
              setError(undefined);
              pickup.mutate(
                { orderId: d.orderId, bagCount },
                { onSuccess: () => void startTracking("delivery").catch(() => undefined), onError: fail },
              );
            }}
          />
          <Button
            title="I can't take this order"
            variant="ghost"
            onPress={() =>
              Alert.alert("Drop this order?", "It will be offered to another rider.", [
                { text: "Keep it", style: "cancel" },
                {
                  text: "Drop",
                  style: "destructive",
                  onPress: () => unassign.mutate(d.orderId, { onSuccess: () => router.replace("/"), onError: fail }),
                },
              ])
            }
          />
        </>
      ) : (
        <>
          <Text className="text-sm font-semibold text-muted">STEP 2 · DELIVER</Text>
          <View className="gap-1">
            <Text className="text-2xl font-bold text-ink">{d.drop.label}</Text>
            <Text className="text-base text-ink">
              {[d.drop.line1, d.drop.line2, d.drop.landmark].filter(Boolean).join(", ")}
            </Text>
            <Text className="text-base text-muted">
              {d.drop.city} {d.drop.pincode}
            </Text>
          </View>
          <View className="flex-row gap-3">
            <View className="flex-1">
              <Button title="Navigate" size="lg" onPress={() => navigateTo(d.drop.lat, d.drop.lng)} />
            </View>
            <View className="flex-1">
              <Button title="Call customer" variant="secondary" size="lg" onPress={() => call(d.customerPhone)} />
            </View>
          </View>
          <Text className="text-sm text-muted">Customer: {formatIndianPhone(d.customerPhone)}</Text>
          {d.status === "PICKED_UP" ? (
            <Button title="Start trip" variant="secondary" onPress={() => depart.mutate(d.orderId, { onError: fail })} />
          ) : null}

          <View className="gap-3 rounded-lg border-2 border-line p-4">
            <TextField
              testID="otp-input"
              label="Delivery OTP from the customer"
              keyboardType="number-pad"
              maxLength={4}
              value={otp}
              onChangeText={setOtp}
              error={error}
            />
            <Text className="text-sm text-muted">{d.otpAttemptsLeft} attempt(s) left</Text>
            {cod ? (
              <Pressable
                accessibilityRole="checkbox"
                accessibilityState={{ checked: cashTaken }}
                onPress={() => setCashTaken(!cashTaken)}
                className={`flex-row items-center gap-3 rounded-md p-3 ${cashTaken ? "bg-brand-light" : "bg-surface"}`}
              >
                <View className={`h-6 w-6 rounded-sm border-2 ${cashTaken ? "border-brand bg-brand" : "border-muted"}`} />
                <Text className="flex-1 text-lg font-semibold text-ink">
                  Collected {formatPaise(d.collectPaise)} (cash or UPI)
                </Text>
              </Pressable>
            ) : (
              <Text className="text-base text-success">Prepaid — nothing to collect</Text>
            )}
            <Button
              testID="complete-delivery"
              title="Complete delivery"
              size="lg"
              loading={deliver.isPending}
              disabled={otp.length !== 4 || (cod && !cashTaken)}
              onPress={() => {
                setError(undefined);
                deliver.mutate(
                  { orderId: d.orderId, otp, ...(cod ? { codCollectedPaise: d.collectPaise } : {}) },
                  {
                    onSuccess: () => {
                      void startTracking("idle").catch(() => undefined);
                      Alert.alert("Delivered", "Nice work!");
                      router.replace("/");
                    },
                    onError: fail,
                  },
                );
              }}
            />
          </View>
        </>
      )}
    </Screen>
  );
}
