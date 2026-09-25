import { formatIndianPhone, useAuth, useCurrentUser } from "@vigo/api-client";
import { Button, Screen, StatusCard } from "@vigo/ui";
import { Text, View } from "react-native";
import { useStore } from "../../lib/location";

export default function Account() {
  const user = useCurrentUser();
  const store = useStore();
  const { signOut } = useAuth();

  return (
    <Screen scroll>
      <View className="gap-1">
        <Text className="text-3xl font-bold text-brand">Account</Text>
        <Text className="text-base text-muted">{formatIndianPhone(user.phone)}</Text>
      </View>
      <StatusCard
        title="Delivering from"
        rows={[
          { label: store.name, tone: "ok", value: store.code },
          { label: "Estimated delivery", tone: "ok", value: `${store.etaMinutes} min` },
        ]}
      />
      {/* TODO(phase-4): saved addresses. */}
      <Button title="Sign out" variant="secondary" onPress={() => void signOut()} />
    </Screen>
  );
}
