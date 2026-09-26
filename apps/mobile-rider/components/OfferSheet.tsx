import { ApiError, formatPaise, useAcceptOffer, useDeclineOffer } from "@vigo/api-client";
import type { DeliveryOffer } from "@vigo/types";
import { Button } from "@vigo/ui";
import { router } from "expo-router";
import { useEffect, useState } from "react";
import { Alert, Modal, Text, View } from "react-native";
import { metres } from "../lib/maps";

function useSecondsLeft(expiresAt: string) {
  const deadline = new Date(expiresAt).getTime();
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const t = setInterval(() => setNow(Date.now()), 250);
    return () => clearInterval(t);
  }, []);
  return Math.max(0, Math.ceil((deadline - now) / 1000));
}

/** Full-screen offer with a countdown; the first rider to accept gets it. */
export function OfferSheet({ offer }: { offer: DeliveryOffer }) {
  const accept = useAcceptOffer();
  const decline = useDeclineOffer();
  const secondsLeft = useSecondsLeft(offer.expiresAt);
  const busy = accept.isPending || decline.isPending;

  return (
    <Modal visible animationType="slide" transparent>
      <View className="flex-1 justify-end bg-black/50">
        <View className="gap-5 rounded-t-lg bg-background p-6 pb-10">
          <View className="flex-row items-center justify-between">
            <Text className="text-2xl font-bold text-ink">New delivery</Text>
            <View className={`rounded-pill px-4 py-1 ${secondsLeft <= 10 ? "bg-danger" : "bg-brand"}`}>
              <Text className="text-lg font-bold text-white" testID="offer-countdown">
                {secondsLeft}s
              </Text>
            </View>
          </View>

          <View className="gap-1">
            <Text className="text-sm text-muted">PICKUP · {metres(offer.toStoreM)} away</Text>
            <Text className="text-lg font-semibold text-ink">{offer.storeName}</Text>
            <Text className="text-sm text-muted">{offer.storeAddress}</Text>
          </View>
          <View className="gap-1">
            <Text className="text-sm text-muted">DROP · {metres(offer.tripM)} from store</Text>
            <Text className="text-lg font-semibold text-ink">{offer.dropArea}</Text>
          </View>
          <View className="flex-row justify-between rounded-md bg-surface p-4">
            <Text className="text-base text-ink">
              {offer.itemCount} items · {offer.bagCount ?? "?"} bag(s)
            </Text>
            <Text className="text-base font-bold text-ink">
              {offer.collectPaise > 0 ? `Collect ${formatPaise(offer.collectPaise)}` : "Prepaid"}
            </Text>
          </View>

          <Button
            testID="accept-offer"
            title="Accept"
            size="lg"
            loading={accept.isPending}
            disabled={busy || secondsLeft === 0}
            onPress={() =>
              accept.mutate(offer.orderId, {
                onSuccess: () => router.push("/delivery"),
                onError: (e) => Alert.alert("Couldn't accept", e instanceof ApiError ? e.message : "Try again"),
              })
            }
          />
          <Button
            title="Decline"
            variant="secondary"
            size="lg"
            disabled={busy}
            onPress={() => decline.mutate(offer.orderId)}
          />
        </View>
      </View>
    </Modal>
  );
}
