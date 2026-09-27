import { useAuth, useRiderMe, useRiderOffers, useSetOnline, type LiveStatus } from "@vigo/api-client";
import type { DeliveryOffer } from "@vigo/types";
import { Button, colors, EmptyState, Screen } from "@vigo/ui";
import * as Haptics from "expo-haptics";
import { router } from "expo-router";
import { useCallback, useEffect } from "react";
import { ActivityIndicator, Alert, Pressable, Text, View } from "react-native";
import { OfferSheet } from "../components/OfferSheet";
import { requestPermissions, startTracking, stopTracking } from "../lib/tracking";

export default function Home() {
  const { signOut } = useAuth();
  const me = useRiderMe();
  const setOnline = useSetOnline();
  const onOffer = useCallback((_offer: DeliveryOffer) => {
    void Haptics.notificationAsync(Haptics.NotificationFeedbackType.Warning);
  }, []);
  const offers = useRiderOffers(onOffer);

  const online = me.data?.isOnline ?? false;
  const onDelivery = !!me.data?.activeOrderId;

  // Keep GPS in step with the rider's state (e.g. after an app restart). Only
  // online/delivery changes matter: `me` itself refetches on every live event.
  const loaded = !!me.data;
  useEffect(() => {
    if (!loaded) return;
    if (!online) void stopTracking();
    else void startTracking(onDelivery ? "delivery" : "idle").catch(() => undefined);
  }, [loaded, online, onDelivery]);

  if (me.isPending) {
    return (
      <View className="flex-1 items-center justify-center">
        <ActivityIndicator size="large" color={colors.brand} />
      </View>
    );
  }
  if (!me.data?.storeId) {
    return (
      <EmptyState title="No store assigned" message="Ask an admin to assign you to a dark store.">
        <Button title="Sign out" variant="secondary" size="lg" onPress={() => void signOut()} />
      </EmptyState>
    );
  }

  const toggle = async () => {
    if (!online) {
      if (!(await requestPermissions())) {
        Alert.alert(
          "Location needed",
          "Allow location access 'Always' so we can offer you nearby deliveries while the app is in the background.",
        );
        return;
      }
      await startTracking("idle");
    }
    setOnline.mutate(!online, {
      onSuccess: () => {
        if (online) void stopTracking();
      },
      onError: (e) => Alert.alert("Couldn't update", e.message),
    });
  };

  const offer = offers.data?.[0];

  return (
    <Screen scroll>
      <View className="flex-row items-center justify-between">
        <Text className="text-3xl font-bold text-brand">Vigo Rider</Text>
        <LiveBadge status={offers.live} />
      </View>
      <Text className="text-base text-muted">
        {me.data.storeName} · {me.data.deliveredToday} delivered today
      </Text>

      <Pressable
        testID="online-toggle"
        accessibilityRole="switch"
        accessibilityState={{ checked: online }}
        disabled={setOnline.isPending || onDelivery}
        onPress={() => void toggle()}
        className={`items-center gap-2 rounded-lg p-8 ${online ? "bg-brand" : "bg-surface"}`}
      >
        <Text className={`text-4xl font-bold ${online ? "text-white" : "text-ink"}`}>{online ? "ONLINE" : "OFFLINE"}</Text>
        <Text className={online ? "text-white" : "text-muted"}>
          {onDelivery ? "On a delivery" : online ? "Waiting for orders nearby — tap to go offline" : "Tap to start taking orders"}
        </Text>
      </Pressable>

      {onDelivery ? <Button title="Continue delivery" size="lg" onPress={() => router.push("/delivery")} /> : null}
      <Button title="Profile & history" variant="secondary" size="lg" onPress={() => router.push("/profile")} />
      <Button title="Sign out" variant="ghost" onPress={() => void signOut()} disabled={online} />

      {offer && !onDelivery ? <OfferSheet key={offer.orderId} offer={offer} /> : null}
    </Screen>
  );
}

function LiveBadge({ status }: { status: LiveStatus }) {
  const live = status === "open";
  return (
    <View className="flex-row items-center gap-2">
      <View className={`h-3 w-3 rounded-pill ${live ? "bg-success" : "bg-warning"}`} />
      <Text className="text-sm text-muted">{live ? "Live" : "Connecting…"}</Text>
    </View>
  );
}
