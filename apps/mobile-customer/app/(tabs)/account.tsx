import Ionicons from "@expo/vector-icons/Ionicons";
import { formatIndianPhone, useAddresses, useAuth, useCurrentUser, useDeleteAddress } from "@vigo/api-client";
import { Screen } from "@vigo/ui";
import { router } from "expo-router";
import type { ComponentProps } from "react";
import { Alert, Pressable, Text, View } from "react-native";
import { useStore } from "../../lib/location";
import { colors } from "../../lib/theme";

export default function Account() {
  const user = useCurrentUser();
  const store = useStore();
  const { signOut, deleteAccount } = useAuth();

  const confirmDelete = () =>
    Alert.alert(
      "Delete your account?",
      "Your saved addresses and personal details are erased. Past orders are kept without your details for our records.",
      [
        { text: "Cancel", style: "cancel" },
        {
          text: "Delete",
          style: "destructive",
          onPress: () =>
            void deleteAccount().catch((e: unknown) =>
              Alert.alert("Couldn't delete", e instanceof Error ? e.message : "Try again"),
            ),
        },
      ],
    );

  return (
    <Screen scroll>
      <View className="flex-row items-center gap-4 rounded-lg bg-surface p-4">
        <View className="h-14 w-14 items-center justify-center rounded-pill bg-brand-light">
          <Ionicons name="person" size={26} color={colors.brand} />
        </View>
        <View className="flex-1">
          <Text className="text-xl font-bold text-ink">My account</Text>
          <Text className="text-base text-muted">{formatIndianPhone(user.phone)}</Text>
        </View>
      </View>

      <View className="flex-row gap-3">
        <QuickLink icon="receipt-outline" label="Your orders" onPress={() => router.navigate("/orders")} />
        <QuickLink icon="grid-outline" label="Categories" onPress={() => router.navigate("/categories")} />
        <QuickLink icon="search-outline" label="Search" onPress={() => router.push("/search")} />
      </View>

      <View className="flex-row items-center gap-3 rounded-lg bg-surface p-4">
        <Ionicons name="storefront-outline" size={22} color={colors.brand} />
        <View className="flex-1">
          <Text className="text-sm text-muted">Delivering from</Text>
          <Text className="text-base font-semibold text-ink">{store.name}</Text>
        </View>
        <Text className="text-sm font-semibold text-ink">~{store.etaMinutes} min</Text>
      </View>

      <AddressList />

      <View className="overflow-hidden rounded-lg bg-surface">
        <MenuRow icon="log-out-outline" label="Sign out" onPress={() => void signOut()} />
        <View className="mx-4 h-px bg-line" />
        <MenuRow icon="trash-outline" label="Delete account" danger onPress={confirmDelete} />
      </View>
    </Screen>
  );
}

type IconName = ComponentProps<typeof Ionicons>["name"];

function QuickLink({ icon, label, onPress }: { icon: IconName; label: string; onPress: () => void }) {
  return (
    <Pressable
      accessibilityRole="button"
      onPress={onPress}
      className="flex-1 items-center gap-2 rounded-lg bg-surface py-4 active:opacity-70"
    >
      <Ionicons name={icon} size={22} color={colors.ink} />
      <Text className="text-xs font-semibold text-ink">{label}</Text>
    </Pressable>
  );
}

function MenuRow({ icon, label, onPress, danger }: { icon: IconName; label: string; onPress: () => void; danger?: boolean }) {
  return (
    <Pressable accessibilityRole="button" onPress={onPress} className="flex-row items-center gap-3 p-4 active:bg-background">
      <Ionicons name={icon} size={20} color={danger ? colors.danger : colors.ink} />
      <Text className={`flex-1 text-base ${danger ? "text-danger" : "text-ink"}`}>{label}</Text>
      <Ionicons name="chevron-forward" size={18} color={colors.muted} />
    </Pressable>
  );
}

function AddressList() {
  const addresses = useAddresses();
  const remove = useDeleteAddress();
  return (
    <View className="gap-3 rounded-lg bg-surface p-4">
      <View className="flex-row items-center justify-between">
        <Text className="text-base font-bold text-ink">Saved addresses</Text>
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
