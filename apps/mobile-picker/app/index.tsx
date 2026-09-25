import { formatIndianPhone, useAuth, useCurrentUser, useHealth } from "@vigo/api-client";
import { Button, Screen, StatusCard, type StatusTone } from "@vigo/ui";
import { Text, View } from "react-native";

// TODO(phase-5): replace with the real Vigo Picker home screen.
export default function Home() {
  const user = useCurrentUser();
  const { signOut } = useAuth();
  const health = useHealth();
  const tone = (s: "ok" | "down" | undefined): StatusTone => s ?? (health.isError ? "down" : "pending");
  const label = (s: "ok" | "down" | undefined) => s ?? (health.isError ? "unreachable" : "…");

  return (
    <Screen scroll>
      <View className="gap-1">
        <Text className="text-3xl font-bold text-brand">Vigo Picker</Text>
        <Text className="text-base text-muted">Signed in as {formatIndianPhone(user.phone)}</Text>
      </View>
      <StatusCard
        title="Account"
        rows={[
          { label: "Role", tone: "ok", value: user.role },
          { label: "Store", tone: user.storeId ? "ok" : "pending", value: user.storeId ?? "not assigned" },
        ]}
      />
      <StatusCard
        title="Backend"
        rows={[
          { label: "API", tone: tone(health.data?.status), value: label(health.data?.status) },
          { label: "Postgres", tone: tone(health.data?.database), value: label(health.data?.database) },
          { label: "Redis", tone: tone(health.data?.redis), value: label(health.data?.redis) },
        ]}
      />
      <Button title="Sign out" variant="secondary" size="lg" onPress={() => void signOut()} />
    </Screen>
  );
}
