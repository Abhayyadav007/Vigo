import { ApiError, formatPaise, useDeliveryHistory, useRiderMe, useUpdateRiderProfile } from "@vigo/api-client";
import type { VehicleType } from "@vigo/types";
import { Button, Screen, TextField } from "@vigo/ui";
import { Stack } from "expo-router";
import { useState } from "react";
import { Pressable, Text, View } from "react-native";

const VEHICLES: { type: VehicleType; label: string }[] = [
  { type: "SCOOTER", label: "Scooter" },
  { type: "EV_SCOOTER", label: "EV scooter" },
  { type: "MOTORCYCLE", label: "Motorcycle" },
  { type: "BICYCLE", label: "Bicycle" },
];

export default function Profile() {
  const me = useRiderMe();
  const update = useUpdateRiderProfile();
  const history = useDeliveryHistory();
  const [vehicle, setVehicle] = useState<VehicleType | null>(null);
  const [number, setNumber] = useState<string | null>(null);

  const type = vehicle ?? me.data?.vehicleType ?? "SCOOTER";
  const plate = number ?? me.data?.vehicleNumber ?? "";

  return (
    <Screen scroll>
      <Stack.Screen options={{ headerShown: true, title: "Profile" }} />
      <Text className="text-lg font-bold text-ink">Vehicle</Text>
      <View className="flex-row flex-wrap gap-2">
        {VEHICLES.map((v) => (
          <Pressable
            key={v.type}
            onPress={() => setVehicle(v.type)}
            className={`rounded-pill border px-4 py-2 ${type === v.type ? "border-brand bg-brand-light" : "border-line"}`}
          >
            <Text className={type === v.type ? "font-semibold text-brand" : "text-ink"}>{v.label}</Text>
          </Pressable>
        ))}
      </View>
      {type !== "BICYCLE" ? (
        <TextField
          label="Registration number"
          placeholder="KA01AB1234"
          autoCapitalize="characters"
          value={plate}
          onChangeText={(t) => setNumber(t.toUpperCase().replace(/[^A-Z0-9]/g, ""))}
          error={update.error instanceof ApiError ? update.error.message : undefined}
        />
      ) : null}
      <Button
        title="Save"
        loading={update.isPending}
        onPress={() =>
          update.mutate({ vehicleType: type, ...(type !== "BICYCLE" && plate ? { vehicleNumber: plate } : {}) })
        }
      />

      <Text className="mt-4 text-lg font-bold text-ink">Recent deliveries</Text>
      {history.data?.items.length === 0 ? <Text className="text-muted">None yet.</Text> : null}
      {history.data?.items.map((h) => (
        <View key={`${h.orderId}-${h.assignedAt}`} className="flex-row justify-between border-b border-line py-3">
          <View>
            <Text className="font-semibold text-ink">{h.number}</Text>
            <Text className="text-sm text-muted">
              {new Date(h.assignedAt).toLocaleString("en-IN", { dateStyle: "medium", timeStyle: "short" })}
            </Text>
          </View>
          <View className="items-end">
            <Text className="text-sm text-ink">{h.status.replaceAll("_", " ").toLowerCase()}</Text>
            {h.codCollectedPaise ? (
              <Text className="text-sm text-muted">Cash {formatPaise(h.codCollectedPaise)}</Text>
            ) : null}
          </View>
        </View>
      ))}
    </Screen>
  );
}
