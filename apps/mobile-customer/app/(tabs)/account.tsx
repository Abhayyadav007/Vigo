import { formatIndianPhone, useAddresses, useAuth, useCurrentUser, useDeleteAddress } from "@vigo/api-client";
import { Button, Screen, StatusCard } from "@vigo/ui";
import { router } from "expo-router";
import { Alert, Pressable, Text, View } from "react-native";
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
      <AddressList />
      <Button title="Sign out" variant="secondary" onPress={() => void signOut()} />
    </Screen>
  );
}

function AddressList() {
  const addresses = useAddresses();
  const remove = useDeleteAddress();
  return (
    <View className="gap-2 rounded-lg bg-surface p-4">
      <View className="flex-row items-center justify-between">
        <Text className="text-base font-semibold text-ink">Saved addresses</Text>
        <Pressable onPress={() => router.push("/address/new")}>
          <Text className="font-semibold text-brand">+ Add</Text>
        </Pressable>
      </View>
      {addresses.data?.length === 0 ? <Text className="text-sm text-muted">No saved addresses yet.</Text> : null}
      {addresses.data?.map((a) => (
        <View key={a.id} className="flex-row items-center gap-3">
          <View className="flex-1">
            <Text className="font-medium text-ink">{a.label}</Text>
            <Text className="text-sm text-muted" numberOfLines={1}>
              {[a.line1, a.city, a.pincode].join(", ")}
              {a.servingStoreId ? "" : " · outside delivery area"}
            </Text>
          </View>
          <Pressable
            onPress={() =>
              Alert.alert("Delete address?", a.line1, [
                { text: "Keep", style: "cancel" },
                { text: "Delete", style: "destructive", onPress: () => remove.mutate(a.id) },
              ])
            }
          >
            <Text className="text-danger">Delete</Text>
          </Pressable>
        </View>
      ))}
    </View>
  );
}
