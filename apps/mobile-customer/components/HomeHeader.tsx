import Ionicons from "@expo/vector-icons/Ionicons";
import { useAddresses } from "@vigo/api-client";
import { router } from "expo-router";
import { useEffect, useState } from "react";
import { Pressable, Text, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { useStore } from "../lib/location";
import { colors } from "../lib/theme";

const SEARCH_HINTS = ["milk", "atta", "bread", "eggs", "paneer", "chips", "detergent"];

/** Delivery promise, drop address and a search pill that stays pinned above the feed. */
export function HomeHeader() {
  const store = useStore();
  const insets = useSafeAreaInsets();
  const addresses = useAddresses();
  const address = addresses.data?.find((a) => a.servingStoreId === store.id);

  return (
    <View className="gap-3 bg-brand-light px-4 pb-3" style={{ paddingTop: insets.top + 8 }}>
      <View className="flex-row items-center gap-3">
        <View className="flex-1">
          <Text className="text-xs font-bold uppercase tracking-widest text-brand-dark">Delivery in</Text>
          <Text className="text-3xl font-extrabold text-ink" testID="eta">
            {store.etaMinutes} minutes
          </Text>
          <Pressable
            accessibilityRole="button"
            accessibilityLabel="Change delivery address"
            onPress={() => router.push(address ? "/account" : "/address/new")}
            className="flex-row items-center gap-1 active:opacity-70"
          >
            <Text className="flex-shrink text-sm text-muted" numberOfLines={1}>
              {address ? (
                <>
                  <Text className="font-bold text-ink">{address.label}</Text>
                  {` · ${[address.line1, address.city].filter(Boolean).join(", ")}`}
                </>
              ) : (
                <>
                  <Text className="font-bold text-ink">Add address</Text>
                  {` · delivering from ${store.name}`}
                </>
              )}
            </Text>
            <Ionicons name="chevron-down" size={14} color={colors.ink} />
          </Pressable>
        </View>
        <Pressable
          accessibilityRole="button"
          accessibilityLabel="Account"
          onPress={() => router.navigate("/account")}
          className="h-11 w-11 items-center justify-center rounded-pill border border-line bg-surface active:opacity-70"
        >
          <Ionicons name="person" size={20} color={colors.ink} />
        </Pressable>
      </View>
      <SearchPill />
    </View>
  );
}

function SearchPill() {
  const [i, setI] = useState(0);
  useEffect(() => {
    const t = setInterval(() => setI((n) => (n + 1) % SEARCH_HINTS.length), 2500);
    return () => clearInterval(t);
  }, []);
  return (
    <Pressable
      accessibilityRole="search"
      accessibilityLabel="Search products"
      onPress={() => router.push("/search")}
      className="h-12 flex-row items-center gap-3 rounded-lg border border-line bg-surface px-4 active:opacity-80"
    >
      <Ionicons name="search" size={20} color={colors.muted} />
      <Text className="text-base text-muted">Search "{SEARCH_HINTS[i]}"</Text>
    </Pressable>
  );
}
