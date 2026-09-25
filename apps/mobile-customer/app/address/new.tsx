import { useSaveAddress } from "@vigo/api-client";
import { Button, Screen, TextField } from "@vigo/ui";
import { Stack, router } from "expo-router";
import { useState } from "react";
import { Pressable, Text, View } from "react-native";
import { useDelivery } from "../../lib/location";

const LABELS = ["Home", "Work", "Other"] as const;

export default function NewAddress() {
  const { state } = useDelivery();
  const save = useSaveAddress();
  const [label, setLabel] = useState<string>("Home");
  const [line1, setLine1] = useState("");
  const [line2, setLine2] = useState("");
  const [landmark, setLandmark] = useState("");
  const [city, setCity] = useState("");
  const [pincode, setPincode] = useState("");
  const [error, setError] = useState<string>();

  // TODO(phase-7): let the customer drag a map pin; for now the address is
  // pinned at their current location.
  const coords = state.status === "ready" || state.status === "unserviceable" ? state.coords : null;

  const submit = () => {
    setError(undefined);
    if (!coords) return setError("We need your location to pin this address.");
    if (!line1.trim() || !city.trim()) return setError("Fill in the flat/house and city.");
    if (!/^[1-9]\d{5}$/.test(pincode)) return setError("Enter a valid 6-digit PIN code.");
    save.mutate(
      {
        body: {
          label,
          line1: line1.trim(),
          city: city.trim(),
          pincode,
          location: coords,
          ...(line2.trim() ? { line2: line2.trim() } : {}),
          ...(landmark.trim() ? { landmark: landmark.trim() } : {}),
        },
      },
      { onSuccess: () => router.back(), onError: (e) => setError(e.message) },
    );
  };

  return (
    <Screen scroll>
      <Stack.Screen options={{ headerShown: true, title: "Add address" }} />
      <Text className="text-sm text-muted">
        {coords ? "Pinned at your current location." : "Waiting for your location…"}
      </Text>
      <View className="flex-row gap-2">
        {LABELS.map((l) => (
          <Pressable
            key={l}
            onPress={() => setLabel(l)}
            className={`rounded-pill border px-4 py-2 ${label === l ? "border-brand bg-brand-light" : "border-line"}`}
          >
            <Text className={label === l ? "font-semibold text-brand" : "text-ink"}>{l}</Text>
          </Pressable>
        ))}
      </View>
      <TextField label="Flat / house no. / floor" value={line1} onChangeText={setLine1} />
      <TextField label="Building, street, area (optional)" value={line2} onChangeText={setLine2} />
      <TextField label="Landmark (optional)" value={landmark} onChangeText={setLandmark} />
      <TextField label="City" value={city} onChangeText={setCity} />
      <TextField label="PIN code" value={pincode} onChangeText={setPincode} keyboardType="number-pad" maxLength={6} error={error} />
      <Button title="Save address" onPress={submit} loading={save.isPending} />
    </Screen>
  );
}
